//! This module provides the Warpgrapher client.

use crate::engine::context::RequestContext;
use crate::{Engine, Error};
use inflector::Inflector;
use log::{debug, trace};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

/// A Warpgrapher GraphQL client
///
/// The [`Client`] provides a set of CRUD operations that will
/// automatically generate GraphQL queries that conform to the wargrapher API
///
/// [`Client`]: ./enum.Client.html
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Client;
///
/// let client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
/// ```
#[derive(Clone, Debug)]
pub enum Client<RequestCtx: RequestContext> {
    Http {
        endpoint: String,
        headers: HeaderMap,
    },
    Local {
        engine: Box<Engine<RequestCtx>>,
        metadata: Option<HashMap<String, String>>,
    },
}

impl<RequestCtx: RequestContext> Client<RequestCtx> {
    /// Takes the URL of a Warpgrapher service endpoint and returns a new ['Client'] initialized to
    /// query that endpoint.  The type parameters are only relevant for a local instance of the
    /// Warpgrapher engine, not for a remote HTTP client, so pass () for both type parameters, as
    /// shown in the example below.
    ///
    /// [`Client`]: ./enum.Client.html
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::Client;
    ///
    /// let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    /// ```
    pub fn new_with_http(
        endpoint: &str,
        headers_opt: Option<HashMap<&str, &str>>,
    ) -> Result<Client<RequestCtx>, Error> {
        trace!("Client::new_with_http called -- endpoint: {}", endpoint);

        let mut header_map = HeaderMap::new();
        if let Some(headers) = headers_opt {
            for (key, value) in headers {
                let header_name = HeaderName::from_str(key)
                    .map_err(|e| Error::InvalidHeaderName { source: e })?;
                let header_value = HeaderValue::from_str(value)
                    .map_err(|e| Error::InvalidHeaderValue { source: e })?;
                header_map.insert(header_name, header_value);
            }
        }

        Ok(Client::<RequestCtx>::Http {
            endpoint: endpoint.to_string(),
            headers: header_map,
        })
    }

    /// Takes a Warpgrapher engine and returns a new ['Client'] initialized to query that engine.
    /// The type parameter is the [`RequestContext`] used by the engine.
    ///
    /// [`Client`]: ./enum.Client.html
    /// [`RequestContext`]: ../engine/context/trait.RequestContext.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use tokio::main;
    /// # use warpgrapher::{Client, Configuration, Engine};
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # use warpgrapher::engine::database::no_database::NoDatabaseEndpoint;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let c = Configuration::new(1, Vec::new(), Vec::new());
    /// let endpoint = NoDatabaseEndpoint {};
    /// let engine = Engine::new(c, endpoint.pool().await?).build()?;
    ///
    /// let mut client = Client::<()>::new_with_engine(engine, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_engine(
        engine: Engine<RequestCtx>,
        metadata: Option<HashMap<String, String>>,
    ) -> Client<RequestCtx> {
        trace!("Client::new_with_engine called");
        Client::<RequestCtx>::Local {
            engine: Box::new(engine),
            metadata,
        }
    }

    /// Executes a graphql query
    ///
    /// # Arguments
    ///
    /// * query - text of the query statement, parameterized to avoid query injection attacks
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
    /// * result_field - an optional name of a field under 'data' that holds the GraphQL response.
    /// If present, the object with name `result_field` under `data` will be returned. If `None`,
    /// the `data` object will be returned.
    ///
    /// [`Client`]: ./enum.Client.html
    ///
    /// # Return
    ///
    /// A [`serde_json::Value`] containing the query response
    ///
    /// # Errors
    ///
    /// * [`ClientRequestFailed`] - if the HTTP response is a non-OK
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    ///
    /// let query = "query { Project { id name } }";
    /// let results = client.graphql("query { Project { id name } }", None, None, Some("Project")).await;
    /// # }
    /// ```
    pub async fn graphql(
        &mut self,
        query: &str,
        input: Option<&Value>,
        options: Option<&Value>,
        result_field_opt: Option<&str>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::graphql called -- query: {} | input: {:#?} | options: {:#?} | result_field: {:#?}",
            query,
            input,
            options,
            result_field_opt,
        );

        // format request body
        let req_body = json!({
            "query": query.to_string(),
            "variables": {
                "input": input,
                "options": options
            }
        });

        debug!("Client::graphql making request -- req_body: {}", req_body);
        let mut body = match self {
            Client::Http { endpoint, headers } => {
                let client = reqwest::Client::new();
                let response = client
                    .post(endpoint.as_str())
                    .headers(headers.clone())
                    .json(&req_body)
                    .send()
                    .await?;
                response.json::<serde_json::Value>().await?
            }
            Client::Local { engine, metadata } => {
                engine
                    .execute(
                        query.to_string(),
                        match (input, options) {
                            (Some(i), Some(o)) => {
                                Some(serde_json::json!({"input": i.clone(), "options": o.clone()}))
                            }
                            (Some(i), None) => Some(serde_json::json!({"input": i.clone()})),
                            (None, Some(o)) => Some(serde_json::json!({"options": o.clone()})),
                            (None, None) => None,
                        },
                        metadata.clone().unwrap_or_default(),
                    )
                    .await?
            }
        };
        debug!("Client::graphql -- response body: {:#?}", body);

        if let Some(result_field) = result_field_opt {
            body.as_object_mut()
                .and_then(|m| m.remove("data"))
                .and_then(|mut d| d.as_object_mut().and_then(|dm| dm.remove(result_field)))
                .ok_or_else(|| Error::PayloadNotFound {
                    response: body.to_owned(),
                })
        } else {
            body.as_object_mut()
                .and_then(|m| m.remove("data"))
                .ok_or_else(|| Error::PayloadNotFound {
                    response: body.to_owned(),
                })
        }
    }

    /// Creates a node
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for which to create a node
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    ///
    /// let projects = client.create_node("Project", "id name description",
    ///     &json!({"name": "TodoApp", "description": "Action list tracking application"}), None).await;
    /// # }
    /// ```
    pub async fn create_node(
        &mut self,
        type_name: &str,
        shape: &str,
        input: &Value,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::create_node called -- type_name: {} | shape: {} | input: {:#?} | options: {:#?}",
            type_name,
            shape,
            input,
            options
        );

        let query = Client::<()>::fmt_create_node_query(type_name, shape);
        let result_field = type_name.to_string() + "Create";
        self.graphql(&query, Some(input), options, Some(&result_field))
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
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http:://localhost:5000/graphql", None).unwrap();
    ///
    /// let proj_issues = client.create_rel("Project",
    ///     "issues",
    ///     "id props { since } src { id name } dst { id name }",
    ///     &json!({"name": "ProjectName"}),
    ///     &json!({"props": {"since": "2000"},
    ///            "dst": {"Feature": {"NEW": {"name": "NewFeature"}}}}),
    ///            None
    /// ).await;
    /// # }
    /// ```
    pub async fn create_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        match_input: &Value,
        create_input: &Value,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::create_rel called -- type_name: {} | rel_name: {} | shape: {} | match_input: {:#?} | create_input: {:#?} | options: {:#?}",
            type_name,
            rel_name,
            shape,
            match_input,
            create_input,
            options
        );

        let query = Client::<()>::fmt_create_rel_query(type_name, rel_name, shape);
        let input = json!({"MATCH": match_input, "CREATE": create_input});
        let result_field = type_name.to_string()
            + &*((&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>())
            + "Create";

        self.graphql(&query, Some(&input), options, Some(&result_field))
            .await
    }

    /// Deletes one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] of the node to delete
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to select the node(s) on which to create the relationship
    /// * delete_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query to use in deleting the relationship. By default, all
    /// relationships incoming to and outgoing from the node are deleted. The delete input argument
    /// allows for extending the delete operation through relationships to destination nodes.
    ///  * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use warpgrapher::Client;
    /// # use serde_json::json;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    ///
    /// let projects = client.delete_node("Project",
    ///     Some(&json!({"name": "MJOLNIR"})), None, None).await;
    /// # }
    /// ```
    pub async fn delete_node(
        &mut self,
        type_name: &str,
        match_input: Option<&Value>,
        delete_input: Option<&Value>,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::delete_node called -- type_name: {} | match_input: {:#?} | delete_input: {:#?} | options: {:#?}",
            type_name,
            match_input,
            delete_input,
            options
        );

        let query = Client::<()>::fmt_delete_node_query(type_name);
        let input = if let Some(di) = delete_input {
            json!({"MATCH": match_input, "DELETE": di})
        } else {
            json!({ "MATCH": match_input })
        };
        let result_field = type_name.to_string() + "Delete";
        self.graphql(&query, Some(&input), options, Some(&result_field))
            .await
    }

    /// Deletes one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for which to delete a relationship
    /// * rel_name - the name of the [`Relationship`] to delete
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
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http:://localhost:5000/graphql", None).unwrap();
    ///
    /// let proj_issues = client.delete_rel("Project", "issues",
    ///    Some(&json!({"props": {"since": "2000"}})),
    ///    None,
    ///    Some(&json!({"Bug": {}})),
    ///    None
    /// ).await;
    /// # }
    /// ```
    pub async fn delete_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        match_input: Option<&Value>,
        src_input: Option<&Value>,
        dst_input: Option<&Value>,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::delete_rel called -- type_name: {} | rel_name: {} | match_input: {:#?} | src_input: {:#?} | dst_input: {:#?} | options: {:#?}",
            type_name,
            rel_name,
            match_input,
            src_input,
            dst_input,
            options
        );

        let query = Client::<()>::fmt_delete_rel_query(type_name, rel_name);
        let mut m = HashMap::new();
        if let Some(mi) = match_input {
            m.insert("MATCH".to_string(), mi);
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
        let result_field = type_name.to_string()
            + &*((&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>())
            + "Delete";
        self.graphql(&query, input, options, Some(&result_field))
            .await
    }

    /// Queries to retrieve one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] to be retrieved
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    ///
    /// let projects = client.read_node("Project", "id name description", None, None).await;
    /// # }
    /// ```
    pub async fn read_node(
        &mut self,
        type_name: &str,
        shape: &str,
        input: Option<&Value>,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::read_node called -- type_name: {} | shape: {} | input: {:#?} | options: {:#?}",
            type_name,
            shape,
            input,
            options
        );

        let query = Client::<()>::fmt_read_node_query(type_name, shape);
        self.graphql(&query, input, options, Some(type_name)).await
    }

    /// Queries for one or more relationships
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] for the source node in the relationship
    /// * rel_name - the name of the [`Relationship`] to find
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * input - a [`serde_json::Value`], specifically a Value::Object, containing the arguments
    /// to the graph query to select the relationship(s) to return
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http:://localhost:5000/graphql", None).unwrap();
    ///
    /// let proj_issues = client.read_rel("Project", "issues", "id props { since }",
    ///     Some(&json!({"props": {"since": "2000"}})),
    ///     None
    /// ).await;
    /// # }
    /// ```
    pub async fn read_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        input: Option<&Value>,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::read_rel called -- type_name: {} | rel_name: {} | shape: {} | input: {:#?} | options: {:#?}",
            type_name,
            rel_name,
            shape,
            input,
            options
        );

        let query = Client::<()>::fmt_read_rel_query(type_name, rel_name, shape);
        let result_field = type_name.to_string()
            + &*((&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>());
        self.graphql(&query, input, options, Some(&result_field))
            .await
    }

    /// Updates one or more nodes
    ///
    /// # Arguments
    ///
    /// * type_name - the name of the [`Type`] to be updated
    /// * shape - the GraphQL query shape, meaning the selection of objects and properties to be
    /// returned in the query result
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to select the set of nodes to update
    /// * update_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arugments to the graph query used to change the properties of the nodes being updated
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    ///     let mut client = Client::<()>::new_with_http("http://localhost:5000/graphql", None).unwrap();
    ///
    ///     let projects = client.update_node("Project", "id name status",
    ///         Some(&json!({"name": "TodoApp"})), &json!({"status": "ACTIVE"}),
    ///         None).await;
    /// # }
    /// ```
    pub async fn update_node(
        &mut self,
        type_name: &str,
        shape: &str,
        match_input: Option<&Value>,
        update_input: &Value,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::update_node called -- type_name: {} | shape: {} | | match_input: {:#?} | update_input: {:#?} | options: {:#?}",
            type_name,
            shape,
            match_input,
            update_input,
            options
        );

        let query = Client::<()>::fmt_update_node_query(type_name, shape);
        let input = json!({"MATCH": match_input, "SET": update_input});
        let result_field = type_name.to_string() + "Update";
        self.graphql(&query, Some(&input), options, Some(&result_field))
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
    /// * match_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to select the set of relationships to update
    /// * update_input - a [`serde_json::Value`], specifically a Value::Object, containing the
    /// arguments to the graph query used to change the properties of the items being updated
    /// * options - used to pass additional options to the query,
    ///   - direction - a string literal of `ascending` or `descending` indicating the order in which
    ///     to sort results, if the `orderBy` option is also provided.
    ///   - orderBy - the name of a scalar field by which to sort the data to be returned
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
    /// * [`PayloadNotFound`] - if the JSON response body is not a valid GraphQL
    /// response
    ///
    /// [`ClientRequestFailed`]: ../enum.Error.html#variant.ClientRequestFailed
    /// [`PayloadNotFound`]: ../enum.Error.html#variant.PayloadNotFound
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use serde_json::json;
    /// # use warpgrapher::Client;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = Client::<()>::new_with_http("http:://localhost:5000/graphql", None).unwrap();
    ///
    /// let proj_issues = client.update_rel("Project", "issues",
    ///     "id props {since} src {id name} dst {id name}",
    ///     Some(&json!({"props": {"since": "2000"}})),
    ///     &json!({"props": {"since": "2010"}}),
    ///     None
    /// ).await;
    /// # }
    /// ```
    pub async fn update_rel(
        &mut self,
        type_name: &str,
        rel_name: &str,
        shape: &str,
        match_input: Option<&Value>,
        update_input: &Value,
        options: Option<&Value>,
    ) -> Result<Value, Error> {
        trace!(
            "Client::update_rel called -- type_name: {} | rel_name: {} | shape: {} | | match_input: {:#?} | update_input: {:#?} | options: {:#?}",
            type_name,
            rel_name,
            shape,
            match_input,
            update_input,
            options
        );

        let query = Client::<()>::fmt_update_rel_query(type_name, rel_name, shape);
        let input = json!({"MATCH": match_input, "SET": update_input});
        let result_field = type_name.to_string()
            + &*((&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>())
            + "Update";
        self.graphql(&query, Some(&input), options, Some(&result_field))
            .await
    }

    fn fmt_create_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($input: {type_name}CreateMutationInput!, $options: {type_name}Options) {{ 
                {type_name}Create(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_create_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Create($input: {type_name}{rel_name}CreateInput!, $options: {type_name}{rel_name}Options) {{
                {type_name}{rel_name}Create(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = (&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>(),
            shape = shape
        )
    }

    fn fmt_delete_node_query(type_name: &str) -> String {
        format!(
            "mutation Delete($input: {type_name}DeleteInput!, $options: {type_name}Options) {{ 
                {type_name}Delete(input: $input, options: $options)
            }}",
            type_name = type_name
        )
    }

    fn fmt_delete_rel_query(type_name: &str, rel_name: &str) -> String {
        format!(
            "mutation Delete($input: {type_name}{rel_name}DeleteInput!, $options: {type_name}{rel_name}Options) {{
                {type_name}{rel_name}Delete(input: $input, options: $options)
            }}",
            type_name = type_name,
            rel_name = (&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>(),
        )
    }

    fn fmt_read_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "query Read($input: {type_name}QueryInput, $options: {type_name}Options) {{ 
                {type_name}(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_read_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "query Read($input: {type_name}{rel_name}QueryInput, $options: {type_name}{rel_name}Options) {{
                {type_name}{rel_name}(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = (&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>(),
            shape = shape
        )
    }

    fn fmt_update_node_query(type_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($input: {type_name}UpdateInput!, $options: {type_name}Options) {{
                {type_name}Update(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            shape = shape
        )
    }

    fn fmt_update_rel_query(type_name: &str, rel_name: &str, shape: &str) -> String {
        format!(
            "mutation Update($input: {type_name}{rel_name}UpdateInput!, $options: {type_name}{rel_name}Options) {{
                {type_name}{rel_name}Update(input: $input, options: $options) {{ {shape} }}
            }}",
            type_name = type_name,
            rel_name = (&rel_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>(),
            shape = shape
        )
    }
}

impl<R> Display for Client<R>
where
    R: RequestContext,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::Http { endpoint, headers } => {
                write!(f, "{}, metadata = {:#?}", endpoint, headers)
            }
            Self::Local { engine, metadata } => write!(f, "{}, metadata = {:#?}", engine, metadata),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    /// Passes if a new client is created with the endpoint passed into the constructor
    #[test]
    fn new() {
        let ep = "http://localhost:5000/graphql";
        let client = Client::<()>::new_with_http(ep, None);
        if let Ok(Client::Http { endpoint, .. }) = client {
            assert_eq!(ep, endpoint);
        } else {
            unreachable!()
        }
    }

    /// Passes if a client formats a read node query correctly
    #[test]
    fn fmt_read_node_query() {
        let actual = Client::<()>::fmt_read_node_query("Project", "id");
        let expected = r#"query Read($input: ProjectQueryInput, $options: ProjectOptions) { 
                Project(input: $input, options: $options) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    /// Passes if a client formats a create node query correctly
    #[test]
    fn fmt_create_node_query() {
        let actual = Client::<()>::fmt_create_node_query("Project", "id");
        let expected = r#"mutation Create($input: ProjectCreateMutationInput!, $options: ProjectOptions) { 
                ProjectCreate(input: $input, options: $options) { id }
            }"#;
        assert_eq!(actual, expected);
    }

    /// Passes if Client implements the Send trait
    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Client<()>>();
    }

    /// Passes if Client implements the Sync trait
    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Client<()>>();
    }
}
