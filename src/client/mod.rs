//! This module provides the Warpgrapher client.

use super::error::{Error, ErrorKind};
use actix_web::client::Client;
use inflector::Inflector;
use log::{debug, trace};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Takes and executes a raw GraphQL query. Takes an optional input which is inserted
/// in the GraphQL request as "input" in the variables.
///
/// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
///
/// # Examples
///
/// ```rust,no_run
/// use std::env::var_os;
/// use warpgrapher::client::graphql;
///
/// let query = "query { Project { id name } }";
/// let results = graphql("http://localhost:5000/graphql".to_owned(), query.to_owned(),
///     Some("1234".to_string()), None);
/// let projects = results.unwrap().get("Project");
/// ```
#[actix_rt::main]
pub async fn graphql(
    endpoint: String,
    query: String,
    partition_key: Option<String>,
    input: Option<Value>,
) -> Result<Value, Error> {
    // TODO: return a Future
    let req_body = json!({
        "query": query.to_string(),
        "variables": {
            "partitionKey": partition_key,
            "input": input
        }
    });
    trace!("client::graphql request body: {:#?}", req_body);
    let mut res = Client::default()
        .post(endpoint)
        .header("Content-Type", "application/json")
        .send_json(&req_body)
        .await
        .map_err(|e| Error::new(ErrorKind::ClientRequestFailed(format!("{:#?}", e)), None))?;

    let body: Value = res
        .json()
        .await
        .map_err(|_e| Error::new(ErrorKind::ClientReceivedInvalidJson, None))?;

    trace!("Response Body: {:#?}", body);
    match body.get("data") {
        None => Err(Error::new(
            ErrorKind::ClientRequestUnexpectedPayload(body.to_owned()),
            None,
        )),
        Some(data) => {
            debug!("Result Data: {:#?}", data);
            Ok(data.to_owned())
        }
    }
}

/// A Warpgrapher GraphQL client
///
/// The [`WarpgrapherClient`] provides a set of CRUD operations that will
/// automatically generate GraphQL queries that conform to the wargrapher API
///
/// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
///
/// # Examples
///
/// ```rust
/// use std::env::var_os;
/// use warpgrapher::WarpgrapherClient;
///
/// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
/// ```
pub struct WarpgrapherClient {
    endpoint: String,
}

impl WarpgrapherClient {
    /// Takes an endpoint string and creates a new [`WarpgrapherClient`].
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    /// ```
    pub fn new(endpoint: &str) -> WarpgrapherClient {
        WarpgrapherClient {
            endpoint: endpoint.to_string(),
        }
    }

    /// Takes the name of a WarpgrapherType and executes a NodeCreate operation. Requires
    /// a query shape and the input of the node being created.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    /// let projects = client.create_node(
    ///     "Project",
    ///     "id name description", Some("1234".to_string()),
    ///     &json!({"name": "TodoApp", "description": "TODO list tracking application"}),
    /// );
    /// ```
    pub fn create_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<String>,
        input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_create_node_query(type_name, shape);
        debug!(
            "WarpGrapherClient create_node -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(
            self.endpoint.to_owned(),
            query,
            partition_key,
            Some(input.to_owned()),
        )?;
        self.strip(&result, &format!("{}Create", type_name))
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a RelCreate operation. Requires also a query shape,
    /// an input to match the source node(s) for which the rel(s) will be created,
    /// and an input of the relationship being created.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    /// let proj_issues = client.create_rel(
    ///     "Project",
    ///     "issues",
    ///     "id props { since } src { id name } dst { id name }",
    ///     Some("1234".to_string()),
    ///     &json!({"name": "ProjectName"}),
    ///     &json!({"props": {"since": "2000"},
    ///            "dst": {"Feature": {"NEW": {"name": "NewFeature"}}}})
    /// );
    /// ```
    pub fn create_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        partition_key: Option<String>,
        match_input: &Value,
        create_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_create_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "create": create_input});
        debug!(
            "WarpGrapherClient create_rel -- query: {:#?}, partition_key {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, Some(input))?;
        self.strip(
            &result,
            &format!("{}{}Create", type_name, rel_name.to_title_case()),
        )
    }

    /// Takes the name of a WarpgrapherType and executes a Delete operation.
    /// Takes two optional inputs.  The first selects the node(s) for deletion.  
    /// The second contains options for forcing the delete, meaning deleting
    /// node along with any relationships to and from it -- without the force flag,
    /// deleting a node with relationships will fail.  Also contains options for
    /// deleting additional nodes and rels connected to the target node. Returns
    /// the number of nodes of the WarpgrapherType deleted.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    /// use serde_json::json;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    /// let projects = client.delete_node(
    ///     "Project", Some("1234".to_string()),
    ///     Some(&json!({"name": "MJOLNIR"})),
    ///     None);
    /// ```
    pub fn delete_node(
        &mut self,
        type_name: &str,
        partition_key: Option<String>,
        match_input: Option<&Value>,
        delete_input: Option<&Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_delete_node_query(type_name);
        let input = json!({"match": match_input, "delete": delete_input});
        debug!(
            "WarpGrapherClient delete_node -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, Some(input))?;
        self.strip(&result, &(type_name.to_string() + "Delete"))
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a RelDelete operation. Takes three optional inputs.  The
    /// first selects the relationship(s) for deletion. The second and third,
    /// if present, request the deletion of the src and dst nodes associated with the
    /// relationship, and potentially additional nodes and rels as well. Returns
    /// the number of matched relationships deleted.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    /// let proj_issues = client.delete_rel("Project", "issues", Some("1234".to_string()),
    ///     Some(&json!({"props": {"since": "2000"}})),
    ///     None,
    ///     Some(&json!({"Bug": {"force": true}}))
    /// );
    /// ```
    pub fn delete_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        partition_key: Option<String>,
        match_input: Option<&Value>,
        src_input: Option<&Value>,
        dst_input: Option<&Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_delete_rel_query(type_name, rel_name);
        let mut m = BTreeMap::new();
        if let Some(mi) = match_input {
            m.insert("match".to_owned(), mi);
        }
        if let Some(src) = src_input {
            m.insert("src".to_owned(), src);
        }
        if let Some(dst) = dst_input {
            m.insert("dst".to_owned(), dst);
        }
        let input = if m.is_empty() { None } else { Some(json!(m)) };
        debug!(
            "WarpGrapherClient delete_rel -- query: {:#?}, parition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, input)?;
        self.strip(
            &result,
            &format!("{}{}Delete", type_name, rel_name.to_title_case()),
        )
    }

    /// Takes the name of a WarpgrapherType and executes a Read operation. Requires
    /// a query shape and takes an optional input which filters the results.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    /// let projects = client.read_node("Project", "id name description", Some("1234".to_string()), None);
    /// ```
    pub fn read_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<String>,
        input: Option<Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_read_node_query(type_name, shape);
        debug!(
            "WarpGrapherClient read_node -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, input)?;
        self.strip(&result, type_name)
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a read operation. Also takes an option input with match
    /// criteria for selecting the relationship to read.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    /// let proj_issues = client.read_rel("Project", "issues",
    ///     "id props { since }", Some("1234".to_string()),
    ///     Some(json!({"props": {"since": "2000"}}))
    /// );
    /// ```
    pub fn read_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        partition_key: Option<String>,
        input: Option<Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_read_rel_query(type_name, rel_name, shape);
        debug!(
            "WarpGrapherClient read_rel -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, input)?;
        self.strip(
            &result,
            &format!("{}{}", type_name, rel_name.to_title_case()),
        )
    }

    /// Takes the name of a WarpgrapherType and executes an Update operation.
    /// Requires a query shape, an optional match component of the input, and a
    /// mandatory update component of the input.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    /// let projects = client.update_node(
    ///     "Project",
    ///     "id name status", Some("1234".to_string()),
    ///     Some(&json!({"name": "TodoApp"})),
    ///     &json!({"status": "ACTIVE"}),
    /// );
    /// ```
    pub fn update_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<String>,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_update_node_query(type_name, shape);
        let input = json!({"match": match_input, "modify": update_input});
        debug!(
            "WarpGrapherClient update_node -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, Some(input))?;
        self.strip(&result, &format!("{}Update", type_name))
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a RelUpdate operation.  Requires a shape of result to
    /// be returned.  Takes an optional input that selects the relationship for
    /// update, and a mandatory input describing the update to be performed.  
    /// Returns the number of matched relationships deleted.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    /// let proj_issues = client.update_rel("Project", "issues",
    ///     "id props {since} src {id name} dst {id name}", Some("1234".to_string()),
    ///     Some(&json!({"props": {"since": "2000"}})),
    ///     &json!({"props": {"since": "2010"}})
    /// );
    /// ```
    pub fn update_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        partition_key: Option<String>,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_update_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "update": update_input});
        debug!(
            "WarpGrapherClient update_rel -- query: {:#?}, partition_key: {:#?}, input: {:#?}",
            query, partition_key, input
        );
        let result = graphql(self.endpoint.to_owned(), query, partition_key, Some(input))?;
        self.strip(
            &result,
            &format!("{}{}Update", type_name, rel_name.to_title_case()),
        )
    }

    fn fmt_create_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($partitionKey: String, $input: {type_name}CreateMutationInput!) {{ 
                {type_name}Create(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_create_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($partitionKey: String, $input: {type_name}{rel_name}CreateInput!) {{
                {type_name}{rel_name}Create(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_delete_node_query(&self, type_name: &str) -> String {
        format!(
            "mutation Delete($partitionKey: String, $input: {type_name}DeleteInput!) {{ 
                {type_name}Delete(partitionKey: $partitionKey, input: $input)
            }}",
            type_name = type_name
        )
    }

    fn fmt_delete_rel_query(&self, type_name: &str, rel_name: &str) -> String {
        format!(
            "mutation Delete($partitionKey: String, $input: {type_name}{rel_name}DeleteInput!) {{
                {type_name}{rel_name}Delete(partitionKey: $partitionKey, input: $input)
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
        )
    }

    fn fmt_read_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "query Read($partitionKey: String, $input: {type_name}QueryInput) {{ 
                {type_name}(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_read_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "query Read($partitionKey: String, $input: {type_name}{rel_name}QueryInput) {{
                {type_name}{rel_name}(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_update_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($partitionKey: String, $input: {type_name}UpdateInput!) {{
                {type_name}Update(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_update_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($partitionKey: String, $input: {type_name}{rel_name}UpdateInput!) {{
                {type_name}{rel_name}Update(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn strip(&self, data: &Value, name: &str) -> Result<Value, Error> {
        match data.get(name) {
            None => Err(Error::new(
                ErrorKind::ClientRequestUnexpectedPayload(data.to_owned()),
                None,
            )),
            Some(v) => Ok(v.to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::WarpgrapherClient;

    #[test]
    fn new() {
        let endpoint = "http://localhost:5000/graphql";
        let client = WarpgrapherClient::new(&endpoint);
        assert_eq!(client.endpoint, endpoint);
    }

    #[test]
    fn fmt_read_node_query() {
        let endpoint = "http://localhost:5000/graphql";
        let client = WarpgrapherClient::new(&endpoint);

        let actual = client.fmt_read_node_query("Project", "id");
        let expected = r#"query Read($partitionKey: String, $input: ProjectQueryInput) { 
                Project(partitionKey: $partitionKey, input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn fmt_create_node_query() {
        let endpoint = "http://localhost:5000/graphql";
        let client = WarpgrapherClient::new(&endpoint);

        let actual = client.fmt_create_node_query("Project", "id");
        let expected = r#"mutation Create($partitionKey: String, $input: ProjectCreateMutationInput!) { 
                ProjectCreate(partitionKey: $partitionKey, input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }
}
