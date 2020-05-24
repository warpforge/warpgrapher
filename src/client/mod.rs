//! This module provides the Warpgrapher client.

use crate::Error;
use inflector::Inflector;
use log::{debug, trace};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// A Warpgrapher GraphQL client
///
/// The [`Client`] provides a set of CRUD operations that will
/// automatically generate GraphQL queries that conform to the wargrapher API
///
/// [`Client`]: ./struct.Client.html
///
/// # Examples
///
/// ```rust
/// use warpgrapher::client::Client;;
///
/// let mut client = Client::new("http://localhost:5000/graphql");
/// ```
#[derive(Clone, Hash, Debug, Default)]
pub struct Client {
    endpoint: String,
}

impl Client {
    /// Takes the URL of a Warpgrapher service endpoint and returns a new ['Client'] initialized to
    /// query that endpoint.
    ///
    /// [`Client`]: ./struct.Client.html
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::client::Client;
    ///
    /// let mut client = Client::new("http://localhost:5000/graphql");
    /// ```
    pub fn new(endpoint: &str) -> Client {
        trace!("Client::new called -- endpoint: {}", endpoint);
        Client {
            endpoint: endpoint.to_string(),
        }
    }

    /// Executes a graphql query
    ///
    /// # Arguments
    ///
    /// * query - text of the query statement, parameterized to avoid query injection attacks
    /// * partition_key - used to scope a query to a Cosmos DB partition. In future, when Neo4J is
    /// supported, it is anticipated that the partition_key will be used to select among Neo4J
    /// fabric shards.
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    /// * result_field - name of the field under 'data' that holds the GraphQL response
    ///
    /// [`Client`]: ./struct.Client.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http://localhost:5000/graphql");
    ///
    ///     let query = "query { Project { id name } }";
    ///     let results = client.graphql("query { Project { id name } }", Some("1234"), None,
    ///         "Project").await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn graphql(
        &mut self,
        query: &str,
        partition_key: Option<&str>,
        input: Option<&Value>,
        result_field: &str,
    ) -> Result<Value, Error> {
        trace!(
            "Client::graphql called -- query: {} | partition_key: {:#?} | input: {:#?} | result_field: {}",
            query,
            partition_key,
            input,
            result_field
        );

        // format request body
        let req_body = json!({
            "query": query.to_string(),
            "variables": {
                "partitionKey": partition_key,
                "input": input
            }
        });

        // send request
        let client = reqwest::Client::new();
        debug!(
            "Client::graphql posting request -- endpoint: {} | req_body: {}",
            self.endpoint, req_body
        );
        let raw_resp = client
            .post(self.endpoint.as_str())
            .json(&req_body)
            .send()
            .await;
        debug!(
            "Client::graphql receiving response -- response: {:#?}",
            raw_resp
        );
        let resp = raw_resp?;

        // parse result
        let mut body = resp.json::<serde_json::Value>().await?;
        debug!("Client::graphql -- response body: {:#?}", body);

        body.as_object_mut()
            .and_then(|m| m.remove("data"))
            .and_then(|mut d| d.as_object_mut().and_then(|dm| dm.remove(result_field)))
            .ok_or_else(|| Error::PayloadNotFound {
                response: body.to_owned(),
            })
    }

    /// Creates a node
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for which to create a node
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    ///
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.create_node("Project", "id name description", Some("1234"),
    ///         &json!({"name": "TodoApp", "description": "TODO list tracking application"})).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn create_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<&str>,
        input: &Value,
    ) -> Result<Value, Error> {
        trace!(
            "Client::create_node called -- type_name: {} | shape: {} | partition_key: {:#?} | input: {:#?}",
            type_name,
            shape,
            partition_key,
            input
        );

        let query = Client::fmt_create_node_query(type_name, shape);
        let result_field = type_name.to_string() + "Create";
        self.graphql(&query, partition_key, Some(input), &result_field)
            .await
    }

    /// Creates one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for which to create a relationship
    /// * rel_name - the name of the [`Relationship`] to create
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to select the node(s) on which to create the relationship
    /// * create_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to use in creating the relationship
    ///
    /// [`Relationship`]: ../engine/config/struct.Relationship.html
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.create_rel("Project",
    ///         "issues",
    ///         "id props { since } src { id name } dst { id name }",
    ///         Some("1234"),
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
        partition_key: Option<&str>,
        match_input: &Value,
        create_input: &Value,
    ) -> Result<Value, Error> {
        trace!(
            "Client::create_rel called -- type_name: {} | rel_name: {} | shape: {} | partition_key: {:#?} | match_input: {:#?} | create_input: {:#?}",
            type_name,
            rel_name,
            shape,
            partition_key,
            match_input,
            create_input
        );

        let query = Client::fmt_create_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "create": create_input});
        let result_field = type_name.to_string() + &rel_name.to_title_case() + "Create";
        self.graphql(&query, partition_key, Some(&input), &result_field)
            .await
    }

    /// Deletes one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] of the node to delete
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to select the node(s) on which to create the relationship
    /// * delete_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to use in deleting the relationship. By default, all
    /// relationships incoming to and outgoing from the node are deleted. The delete input argument
    /// allows for extending the delete operation through relationships to destination nodes.
    ///
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response, a count of the nodes deleted
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use warpgrapher::client::Client;;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.delete_node("Project", Some("1234"),
    ///         Some(&json!({"name": "MJOLNIR"})), None).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn delete_node(
        &mut self,
        type_name: &str,
        partition_key: Option<&str>,
        match_input: Option<&Value>,
        delete_input: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::delete_node called -- type_name: {} | partition_key: {:#?} | match_input: {:#?} | delete_input: {:#?}",
            type_name,
            partition_key,
            match_input,
            delete_input
        );

        let query = Client::fmt_delete_node_query(type_name);
        let input = json!({"match": match_input, "delete": delete_input});
        let result_field = type_name.to_string() + "Delete";
        self.graphql(&query, partition_key, Some(&input), &result_field)
            .await
    }

    /// Deletes one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for which to delete a relationship
    /// * rel_name - the name of the [`Relationship`] to delete
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to select the relationship(s) to delete
    /// * src_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to use in deleting the src node. By default, nodes are not
    /// deleted along with a relationship, but this parameter can be used to delete the source of
    /// the relationship as well.
    /// * dst_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to use in deleting the destination node. By default, nodes are
    /// not deleted along with a relationship, but this parameter can be used to delete the
    /// destination node of the relationship as well.
    ///
    /// [`Relationship`]: ../engine/config/struct.Relationship.html
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response, a count of the relationships deleted
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.delete_rel("Project", "issues",
    ///        Some("1234"),
    ///        Some(&json!({"props": {"since": "2000"}})),
    ///        None,
    ///        Some(&json!({"Bug": {}}))
    ///     ).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn delete_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        partition_key: Option<&str>,
        match_input: Option<&Value>,
        src_input: Option<&Value>,
        dst_input: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::delete_rel called -- type_name: {} | rel_name: {} | partition_key: {:#?} | match_input: {:#?} | src_input: {:#?} | dst_input: {:#?}",
            type_name,
            rel_name,
            partition_key,
            match_input,
            src_input,
            dst_input
        );

        let query = Client::fmt_delete_rel_query(type_name, rel_name);
        let mut m = BTreeMap::new();
        if let Some(mi) = match_input {
            m.insert("match".to_string(), mi);
        }
        if let Some(src) = src_input {
            m.insert("src".to_string(), src);
        }
        if let Some(dst) = dst_input {
            m.insert("dst".to_string(), dst);
        }
        let value: serde_json::Value;
        let input = if m.is_empty() {
            None
        } else {
            value = json!(m);
            Some(&value)
        };
        let result_field = type_name.to_string() + &rel_name.to_title_case() + "Delete";
        self.graphql(&query, partition_key, input, &result_field)
            .await
    }

    /// Queries to retrieve one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] to be retrieved
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    ///
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.read_node("Project", "id name description", Some("1234"),
    ///         None).await;
    /// }
    /// ```
    ///
    #[allow(clippy::needless_doctest_main)]
    pub async fn read_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<&str>,
        input: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::read_node called -- type_name: {} | shape: {} | partition_key: {:#?} | input: {:#?} ",
            type_name,
            shape,
            partition_key,
            input,
        );

        let query = Client::fmt_read_node_query(type_name, shape);
        self.graphql(&query, partition_key, input, type_name).await
    }

    /// Queries for one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for the source node in the relationship
    /// * rel_name - the name of the [`Relationship`] to find
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query to select the relationship(s) to return
    ///
    /// [`Relationship`]: ../engine/config/struct.Relationship.html
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.read_rel("Project", "issues", "id props { since }",
    ///         Some("1234"), Some(&json!({"props": {"since": "2000"}}))).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn read_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        partition_key: Option<&str>,
        input: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::read_rel called -- type_name: {} | rel_name: {} | shape: {} | partition_key: {:#?} | input: {:#?} ",
            type_name,
            rel_name,
            shape,
            partition_key,
            input,
        );

        let query = Client::fmt_read_rel_query(type_name, rel_name, shape);
        let result_field = type_name.to_string() + &rel_name.to_title_case();
        self.graphql(&query, partition_key, input, &result_field)
            .await
    }

    /// Updates one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] to be updated
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to select the set of nodes to update
    /// * update_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arugments to the graph query used to change the properties of the nodes being updated
    ///
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http://localhost:5000/graphql");
    ///
    ///     let projects = client.update_node("Project", "id name status", Some("1234"),
    ///         Some(&json!({"name": "TodoApp"})), &json!({"status": "ACTIVE"})).await;
    /// }
    /// ```
    #[allow(clippy::needless_doctest_main)]
    pub async fn update_node(
        &mut self,
        type_name: &str,
        shape: &str,
        partition_key: Option<&str>,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        trace!(
            "Client::update_node called -- type_name: {} | shape: {} | | partition_key: {:#?} | match_input: {:#?} | update_input: {:#?}",
            type_name,
            shape,
            partition_key,
            match_input,
            update_input
        );

        let query = Client::fmt_update_node_query(type_name, shape);
        let input = json!({"match": match_input, "modify": update_input});
        let result_field = type_name.to_string() + "Update";
        self.graphql(&query, partition_key, Some(&input), &result_field)
            .await
    }

    /// Updates one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for the source node in the relationship(s) to update
    /// * rel_name - the name of the [`Relationship`] to find and update
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * partition_key - the partition_key is used to scope a query to a Cosmos DB partition. In
    /// future, when Neo4J is supported, it is anticipated that the partition_key will be used to
    /// select among Neo4J fabric shards.
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to select the set of relationships to update
    /// * update_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to change the properties of the items being updated
    ///
    /// [`Relationship`]: ../engine/config/struct.Relationship.html
    /// [`Type`]: ../engine/config/struct.Type.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of the following kinds:
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`ClientRequestUnexepctedPayload`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`ClientRequestUnexpectedPayload`]: ../enum.Error.html#variant.ClientRequestUnexpectedPayload
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use warpgrapher::client::Client;;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = Client::new("http:://localhost:5000/graphql");
    ///
    ///     let proj_issues = client.update_rel("Project", "issues",
    ///         "id props {since} src {id name} dst {id name}",
    ///         Some("1234"),
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
        partition_key: Option<&str>,
        match_input: Option<&Value>,
        update_input: &Value,
    ) -> Result<Value, Error> {
        trace!(
            "Client::update_rel called -- type_name: {} | rel_name: {} | shape: {} | | partition_key: {:#?} | match_input: {:#?} | update_input: {:#?}",
            type_name,
            rel_name,
            shape,
            partition_key,
            match_input,
            update_input
        );

        let query = Client::fmt_update_rel_query(type_name, rel_name, shape);
        let input = json!({"match": match_input, "update": update_input});
        let result_field = type_name.to_string() + &rel_name.to_title_case() + "Update";
        self.graphql(&query, partition_key, Some(&input), &result_field)
            .await
    }

    fn fmt_create_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($partitionKey: String, $input: {type_name}CreateMutationInput!) {{ 
                {type_name}Create(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_create_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($partitionKey: String, $input: {type_name}{rel_name}CreateInput!) {{
                {type_name}{rel_name}Create(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_delete_node_query(type_name: &str) -> String {
        format!(
            "mutation Delete($partitionKey: String, $input: {type_name}DeleteInput!) {{ 
                {type_name}Delete(partitionKey: $partitionKey, input: $input)
            }}",
            type_name = type_name
        )
    }

    fn fmt_delete_rel_query(type_name: &str, rel_name: &str) -> String {
        format!(
            "mutation Delete($partitionKey: String, $input: {type_name}{rel_name}DeleteInput!) {{
                {type_name}{rel_name}Delete(partitionKey: $partitionKey, input: $input)
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
        )
    }

    fn fmt_read_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "query Read($partitionKey: String, $input: {type_name}QueryInput) {{ 
                {type_name}(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_read_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "query Read($partitionKey: String, $input: {type_name}{rel_name}QueryInput) {{
                {type_name}{rel_name}(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }

    fn fmt_update_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($partitionKey: String, $input: {type_name}UpdateInput!) {{
                {type_name}Update(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_update_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($partitionKey: String, $input: {type_name}{rel_name}UpdateInput!) {{
                {type_name}{rel_name}Update(partitionKey: $partitionKey, input: $input) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = rel_name.to_title_case(),
            shape = shape
        )
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    /// Passes if a new client is created with the endpoint passed into the constructor
    #[test]
    fn new() {
        let endpoint = "http://localhost:5000/graphql";
        let client = Client::new(&endpoint);
        assert_eq!(client.endpoint, endpoint);
    }

    /// Passes if a client formats a read node query correctly
    #[test]
    fn fmt_read_node_query() {
        let actual = Client::fmt_read_node_query("Project", "id");
        let expected = r#"query Read($partitionKey: String, $input: ProjectQueryInput) { 
                Project(partitionKey: $partitionKey, input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    /// Passes if a client formats a create node query correctly
    #[test]
    fn fmt_create_node_query() {
        let actual = Client::fmt_create_node_query("Project", "id");
        let expected = r#"mutation Create($partitionKey: String, $input: ProjectCreateMutationInput!) { 
                ProjectCreate(partitionKey: $partitionKey, input: $input) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    /// Passes if Client implements the Send trait
    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Client>();
    }

    /// Passes if Client implements the Sync trait
    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Client>();
    }
}
