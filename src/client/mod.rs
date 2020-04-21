//! This module provides the Warpgrapher client.

use super::error::{Error, ErrorKind};
use inflector::Inflector;
use log::{debug};
use serde_json::{json, Value};
use std::collections::BTreeMap;

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
/// use warpgrapher::client::WarpgrapherClient;;
///
/// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
/// ```
#[derive(Clone, Hash, Debug, Default)]
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
    /// use warpgrapher::client::WarpgrapherClient;
    ///
    /// let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    /// ```
    pub fn new(endpoint: &str) -> WarpgrapherClient {
        WarpgrapherClient {
            endpoint: endpoint.to_string(),
        }
    }
    
    /// Executes a raw graphql query.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    ///     let query = "query { Project { id name } }";
    ///     let results = client.graphql(
    ///         "query { Project { id name } }",
    ///         None
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn graphql(
        &mut self,
        query: &str,
        input: Option<&Value>
    ) -> Result<Value, Error> {

        // format request body
        let req_body = json!({
            "query": query.to_string(),
            "variables": {
                "input": input
            }
        });

        // send request
        let client = reqwest::Client::new();
        let resp = client.post(self.endpoint.as_str())
            .json(&req_body)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::ClientRequestFailed, Some(Box::new(e))))?;

        // parse result
        let body = resp.json::<serde_json::Value>()
            .await
            .map_err(|_e| Error::new(ErrorKind::ClientReceivedInvalidJson, None))?;
        
        // extract data from result
        match body.get("data") {
            None => Err(Error::new(
                ErrorKind::ClientRequestUnexpectedPayload(body.to_owned()),
                None,
            )),
            Some(data) => {
                Ok(data.to_owned())
            }
        }
    }

    /// Takes the name of a WarpgrapherType and executes a NodeCreate operation. Requires
    /// a query shape and the input of the node being created.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.create_node(
    ///         "Project",
    ///         "id name description",
    ///         &json!({"name": "TodoApp", "description": "TODO list tracking application"}),
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn create_node(
        &mut self,
        type_name: &str,
        shape: &str,
        input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_create_node_query(type_name, shape);
        debug!(
            "WarpgrapherClient create_node -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, Some(input)).await?;
        self.strip(&result, &format!("{}Create", type_name))
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a RelCreate operation. Requires also a query shape,
    /// an input to match the source node(s) for which the rel(s) will be created,
    /// and an input of the relationship being created.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.create_rel(
    ///         "Project",
    ///         "issues",
    ///         "id props { since } src { id name } dst { id name }",
    ///         &json!({"name": "ProjectName"}),
    ///         &json!({"props": {"since": "2000"},
    ///                "dst": {"Feature": {"NEW": {"name": "NewFeature"}}}})
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn create_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        match_input: &Value,
        create_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_create_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "create": create_input});
        debug!(
            "WarpgrapherClient create_rel -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, Some(&input)).await?;
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
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.delete_node(
    ///         "Project",
    ///         Some(&json!({"name": "MJOLNIR"})),
    ///         None
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn delete_node(
        &mut self,
        type_name: &str,
        match_input: Option<&Value>,
        delete_input: Option<&Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_delete_node_query(type_name);
        let input = json!({"match": match_input, "delete": delete_input});
        debug!(
            "WarpgrapherClient delete_node -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, Some(&input)).await?;
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
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.delete_rel("Project", "issues",
    ///        Some(&json!({"props": {"since": "2000"}})),
    ///        None,
    ///        Some(&json!({"Bug": {"force": true}}))
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn delete_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
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
        let value : serde_json::Value;
        let input = if m.is_empty() { 
            None 
        } else { 
            value = json!(m);
            Some(&value)
        };
        debug!(
            "WarpgrapherClient delete_rel -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, input).await?;
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
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.read_node("Project", "id name description", None).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn read_node(
        &mut self,
        type_name: &str,
        shape: &str,
        input: Option<&Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_read_node_query(type_name, shape);
        debug!(
            "WarpgrapherClient read_node -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, input).await?;
        self.strip(&result, type_name)
    }

    /// Takes the name of a WarpgrapherType and a relationship property on that
    /// types and executes a read operation. Also takes an option input with match
    /// criteria for selecting the relationship to read.
    ///
    /// [`WarpgrapherClient`]: ./struct.WarpgrapherClient.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.read_rel("Project", "issues",
    ///         "id props { since }",
    ///         Some(&json!({"props": {"since": "2000"}}))
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn read_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        input: Option<&Value>,
    ) -> Result<Value, Error> {
        let query = self.fmt_read_rel_query(type_name, rel_name, shape);
        debug!(
            "WarpgrapherClient read_rel -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, input).await?;
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
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.update_node(
    ///         "Project",
    ///         "id name status",
    ///         Some(&json!({"name": "TodoApp"})),
    ///         &json!({"status": "ACTIVE"}),
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn update_node(
        &mut self,
        type_name: &str,
        shape: &str,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_update_node_query(type_name, shape);
        let input = json!({"match": match_input, "modify": update_input});
        debug!(
            "WarpgrapherClient update_node -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, Some(&input)).await?;
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
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    /// [`ClientRequestFailed`] - when the HTTP response is a non-OK
    /// [`ClientReceivedInvalidJson`] - when the HTTP response body is not valid JSON
    /// [`ClientRequestUnexepctedPayload`] - when the HTTP response does not match a proper GraphQL response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use std::env::var_os;
    /// use warpgrapher::client::WarpgrapherClient;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = WarpgrapherClient::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.update_rel("Project", "issues",
    ///         "id props {since} src {id name} dst {id name}",
    ///         Some(&json!({"props": {"since": "2000"}})),
    ///         &json!({"props": {"since": "2010"}})
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn update_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        let query = self.fmt_update_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "update": update_input});
        debug!(
            "WarpgrapherClient update_rel -- query: {:#?}, input: {:#?}",
            query, input
        );
        let result = self.graphql(&query, Some(&input)).await?;
        self.strip(
            &result,
            &format!("{}{}Update", type_name, rel_name.to_title_case()),
        )
    }

    fn fmt_create_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($input: {type_name}CreateMutationInput!) {{ 
                {type_name}Create(input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_create_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($input: {type_name}{rel_name}CreateInput!) {{
                {type_name}{rel_name}Create(input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_delete_node_query(&self, type_name: &str) -> String {
        format!(
            "mutation Delete($input: {type_name}DeleteInput!) {{ 
                {type_name}Delete(input: $input)
            }}",
            type_name = type_name
        )
    }

    fn fmt_delete_rel_query(&self, type_name: &str, rel_name: &str) -> String {
        format!(
            "mutation Delete($input: {type_name}{rel_name}DeleteInput!) {{
                {type_name}{rel_name}Delete(input: $input)
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
        )
    }

    fn fmt_read_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "query Read($input: {type_name}QueryInput) {{ 
                {type_name}(input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_read_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "query Read($input: {type_name}{rel_name}QueryInput) {{
                {type_name}{rel_name}(input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_update_node_query(&self, type_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($input: {type_name}UpdateInput!) {{
                {type_name}Update(input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_update_rel_query(&self, type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($input: {type_name}{rel_name}UpdateInput!) {{
                {type_name}{rel_name}Update(input: $input) {{ {shape} }}
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
        let expected = r#"query Read($input: ProjectQueryInput) { 
                Project(input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn fmt_create_node_query() {
        let endpoint = "http://localhost:5000/graphql";
        let client = WarpgrapherClient::new(&endpoint);

        let actual = client.fmt_create_node_query("Project", "id");
        let expected = r#"mutation Create($input: ProjectCreateMutationInput!) { 
                ProjectCreate(input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }
}
