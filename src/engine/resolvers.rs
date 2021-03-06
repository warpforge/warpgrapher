//! Contains the type aliases, enumerations, and structures to allow for the creation of custom
//! resolvers.

use crate::engine::context::GraphQLContext;
use crate::engine::context::RequestContext;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use crate::engine::database::{DatabaseClient, DatabasePool};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::juniper::BoxFuture;
use crate::Error;
#[cfg(any(feature = "cosmos", feature = "gremlin"))]
use gremlin_client::GremlinClient;
use inflector::Inflector;
#[cfg(feature = "neo4j")]
use mobc::Connection;
#[cfg(feature = "neo4j")]
use mobc_boltrs::BoltConnectionManager;
use std::collections::HashMap;
use std::convert::TryFrom;

pub use juniper::{Arguments, ExecutionResult, Executor, FieldError, FromInputValue};

/// Wraps a Node or Rel, and provides a type-safe distinction between the two, when passing the
/// object on which a field is being resolved to the custom resolver.
#[derive(Debug)]
pub enum Object<'a, RequestCtx: RequestContext> {
    /// Wraps a [`Node`] being passed to a custom resolver
    ///
    /// [`Node`] ../objects/struct.Node.html
    Node(&'a Node<RequestCtx>),

    /// Wraps a [`Rel`] being passed to a custom resolver
    ///
    /// [`Rel`] ../objects/struct.Rel.html
    Rel(&'a Rel<RequestCtx>),
}

/// Type alias for custom resolver functions. Takes a [`ResolverFacade`] and returns an
/// ExecutionResult.
///
/// [`ResolverFacade`]: ./struct.ResolverFacade.html
pub type ResolverFunc<RequestCtx> = fn(ResolverFacade<RequestCtx>) -> BoxFuture<ExecutionResult>;

/// Type alias for a mapping from a custom resolver name to a the Rust function that implements the
/// custom resolver.
pub type Resolvers<RequestCtx> = HashMap<String, Box<ResolverFunc<RequestCtx>>>;

/// Provides a simplified interface to primitive operations such as Node creation, Rel creation,
/// resolution of both scalar and complex types. The [`ResolverFacade`] is the primary mechanism
/// trough which a custom resolver interacts with the rest of the framework.
///
/// [`ResolverFacade`]: ./struct.ResolverFacade.html
pub struct ResolverFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    field_name: String,
    info: &'a Info,
    args: &'a Arguments<'a>,
    parent: Object<'a, RequestCtx>,
    partition_key_opt: Option<&'a Value>,
    executor: &'a Executor<'a, 'a, GraphQLContext<RequestCtx>>,
}

impl<'a, RequestCtx> ResolverFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        field_name: String,
        info: &'a Info,
        args: &'a Arguments,
        parent: Object<'a, RequestCtx>,
        partition_key_opt: Option<&'a Value>,
        executor: &'a Executor<GraphQLContext<RequestCtx>>,
    ) -> Self {
        ResolverFacade {
            field_name,
            info,
            args,
            parent,
            partition_key_opt,
            executor,
        }
    }

    /// Returns the resolver input deserialized into a structure of type T that
    /// implements the serde `Deserialize` trait.
    ///
    /// # Errors
    ///
    /// Returns an [`Error]` variant [`InputItemNotFound`] if no input field was passed
    /// to the query, [`TypeConversionFailed`] if unable to convert a [`Value`]
    /// to a serde_json Value, and [`JsonDeserializationFailed`] if unable to parse the
    /// input data into a struct of type T.
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`InputItemNotFound`]: ../../error/enum.Error.html#variant.InputItemNotFound
    /// [`TypeConversionFailed`]: ../../error/enum.Error.html#variant.TypeConversionFailed
    /// [`JsonDeserializationFailed`]: ../../error/enum.Error.html#variant.JsonDeserializationFailed
    /// [`Value`]: ../value/enum.Value.html
    pub fn input<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        let json_value: serde_json::Value = self
            .args()
            .get::<Value>("input")
            .ok_or_else(|| Error::InputItemNotFound {
                name: "input".to_string(),
            })
            .and_then(serde_json::Value::try_from)
            .map_err(|_| Error::TypeConversionFailed {
                src: "warpgrapher::Value".to_string(),
                dst: "serde_json::Value".to_string(),
            })?;
        let parsed_input: T = serde_json::from_value(json_value)
            .map_err(|e| Error::JsonDeserializationFailed { source: e })?;
        Ok(parsed_input)
    }

    /// Returns the execution metadata that was passed to the engine. If no metadata was
    /// passed to the engine's `execute` method, an empty HashMap is returned.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use tokio::runtime::Runtime;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> ExecutionResult {
    ///     let execution_metadata = facade.metadata();
    ///     // use metadata
    ///     
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn metadata(&self) -> &HashMap<String, String> {
        self.executor.context().metadata()
    }

    /// Returns a neo4j database driver from the pool.
    ///
    /// # Errors
    ///
    /// Returns an [`Error]` variant [`DatabaseNotFound`] if a neo4j database pool
    /// is not found
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`DatabaseNotFound`]: ../../error/enum.Error.html#variant.DatabaseNotFound
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::context::RequestContext;
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    ///
    /// # impl RequestContext for AppCtx {
    /// #    type DBEndpointType = Neo4jEndpoint;
    /// #    fn new() -> Self {AppCtx {}}
    /// # }
    ///
    /// async fn custom_resolve(facade: ResolverFacade<'_, AppCtx>) -> ExecutionResult {
    ///     let neo4j_client = facade.db_into_neo4j().await?;
    ///     // use client
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    #[cfg(feature = "neo4j")]
    pub async fn db_into_neo4j(&self) -> Result<Box<Connection<BoltConnectionManager>>, Error> {
        if let DatabaseClient::Neo4j(client) = self.executor().context().pool().client().await? {
            Ok(client)
        } else {
            Err(Error::DatabaseNotFound)
        }
    }

    /// Returns a cosmos database client from the pool
    ///
    /// # Errors
    ///
    /// Returns an [`Error]` variant [`DatabaseNotFound`] if a neo4j database pool
    /// is not found
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`DatabaseNotFound`]: ../../error/enum.Error.html#variant.DatabaseNotFound
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::context::RequestContext;
    /// # use warpgrapher::engine::database::gremlin::CosmosEndpoint;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    ///
    /// # impl RequestContext for AppCtx {
    /// #    type DBEndpointType = CosmosEndpoint;
    /// #    fn new() -> Self {AppCtx {}}
    /// # }
    ///
    /// async fn custom_resolve(facade: ResolverFacade<'_, AppCtx>) -> ExecutionResult {
    ///
    ///     let cosmos_client = facade.db_into_cosmos().await?;
    ///     
    ///     // use client
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    #[cfg(feature = "cosmos")]
    pub async fn db_into_cosmos(&self) -> Result<Box<GremlinClient>, Error> {
        if let DatabaseClient::Gremlin(client) = self.executor().context().pool().client().await? {
            Ok(client)
        } else {
            Err(Error::DatabaseNotFound)
        }
    }

    /// Returns a gremlin database client from the pool
    ///
    /// # Errors
    ///
    /// Returns an [`Error]` variant [`DatabaseNotFound`] if a neo4j database pool
    /// is not found
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`DatabaseNotFound`]: ../../error/enum.Error.html#variant.DatabaseNotFound
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::context::RequestContext;
    /// # use warpgrapher::engine::database::gremlin::GremlinEndpoint;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    ///
    /// # impl RequestContext for AppCtx {
    /// #    type DBEndpointType = GremlinEndpoint;
    /// #    fn new() -> Self {AppCtx {}}
    /// # }
    ///
    /// async fn custom_resolve(facade: ResolverFacade<'_, AppCtx>) -> ExecutionResult {
    ///
    ///     let gremlin_client = facade.db_into_gremlin().await?;
    ///     
    ///     // use client
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    #[cfg(feature = "gremlin")]
    pub async fn db_into_gremlin(&self) -> Result<Box<GremlinClient>, Error> {
        if let DatabaseClient::Gremlin(client) = self.executor().context().pool().client().await? {
            Ok(client)
        } else {
            Err(Error::DatabaseNotFound)
        }
    }

    /// Returns the arguments provided to the resolver in the GraphQL query
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> ExecutionResult {
    ///     let args = facade.args();
    ///
    ///     // use arguments
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn args(&self) -> &Arguments {
        self.args
    }

    /// Creates a [`Node`], of a given type, with a set of properites
    ///
    /// [`Node`]: ../objects/struct.Node.html
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let typename = "User";
    ///
    ///         let mut props = HashMap::new();
    ///         props.insert("role".to_string(), Value::String("Admin".to_string()));
    ///
    ///         let n = facade.create_node(typename, props);
    ///
    ///         facade.resolve_node(&n).await
    ///     })
    /// }
    /// ```
    pub fn create_node(&self, typename: &str, props: HashMap<String, Value>) -> Node<RequestCtx> {
        Node::new(typename.to_string(), props)
    }

    /// Creates a [`Rel`], with a id, properties, and destination node id and label. The src node
    /// of the relationship is the parent node on which the field is being resolved.
    ///
    /// # Error
    ///
    /// Returns an [`Error`] of variant [`TypeNotExpected`] if the parent object isn't a node
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`TypeNotExpected`]: ../../error/enum.Error.html#variant.TypeNotExpected
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let node_id = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
    ///
    ///         let rel_id = Value::String("1e2ac081-b0a6-4f68-bc88-99bdc4111f00".to_string());
    ///         let mut rel_props = HashMap::new();
    ///         rel_props.insert("since".to_string(), Value::String("2020-01-01".to_string()));
    ///
    ///         let rel = facade.
    ///             create_rel(rel_id, Some(rel_props), node_id, "DstNodeLabel")?;
    ///         facade.resolve_rel(&rel).await
    ///     })
    /// }
    /// ```
    pub fn create_rel(
        &self,
        id: Value,
        props: Option<HashMap<String, Value>>,
        dst_id: Value,
        dst_label: &str,
    ) -> Result<Rel<RequestCtx>, Error> {
        if let Object::Node(parent_node) = self.parent {
            Ok(Rel::new(
                id,
                self.partition_key_opt.cloned(),
                props.map(|p| Node::new("props".to_string(), p)),
                NodeRef::Identifier {
                    id: parent_node.id()?.clone(),
                    label: parent_node.type_name().to_string(),
                },
                NodeRef::Identifier {
                    id: dst_id,
                    label: dst_label.to_string(),
                },
            ))
        } else {
            Err(Error::TypeNotExpected { details: None })
        }
    }

    /// Creates a [`Rel`], with a id, properties, and destination node. The src node of the
    /// relationship is the parent node on which the field is being resolved.
    ///
    /// # Error
    ///
    /// Returns an [`Error`] of variant [`TypeNotExpected`] if the parent object isn't a node
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`TypeNotExpected`]: ../../error/enum.Error.html#variant.TypeNotExpected
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let node_id = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
    ///         let typename = "User";
    ///         let mut props = HashMap::new();
    ///         props.insert("role".to_string(), Value::String("Admin".to_string()));
    ///         let n = facade.create_node(typename, props);
    ///
    ///         let rel_id = Value::String("1e2ac081-b0a6-4f68-bc88-99bdc4111f00".to_string());
    ///         let mut rel_props = HashMap::new();
    ///         rel_props.insert("since".to_string(), Value::String("2020-01-01".to_string()));
    ///
    ///         let rel = facade.create_rel_with_dst_node(rel_id, Some(rel_props), n)?;
    ///         facade.resolve_rel(&rel).await
    ///     })
    /// }
    /// ```
    pub fn create_rel_with_dst_node(
        &self,
        id: Value,
        props: Option<HashMap<String, Value>>,
        dst: Node<RequestCtx>,
    ) -> Result<Rel<RequestCtx>, Error> {
        if let Object::Node(parent_node) = self.parent {
            Ok(Rel::new(
                id,
                self.partition_key_opt.cloned(),
                props.map(|p| Node::new("props".to_string(), p)),
                NodeRef::Identifier {
                    id: parent_node.id()?.clone(),
                    label: parent_node.type_name().to_string(),
                },
                NodeRef::Node(dst),
            ))
        } else {
            Err(Error::TypeNotExpected { details: None })
        }
    }

    /// Returns the [`Info`] struct containing the type schema for the GraphQL model.
    ///
    /// [`Info`]: ../schema/struct.Info.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let info = facade.info();
    ///
    ///         // use info
    ///
    ///         facade.resolve_null()
    ///     })
    /// }
    /// ```
    pub fn info(&self) -> &Info {
        self.info
    }

    /// Returns the [`Executor`] struct used to orchestrate calls to resolvers and to marshall
    /// results into a query response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::resolvers::{Executor, ExecutionResult};
    /// # use warpgrapher::engine::resolvers::ResolverFacade;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let exeuctor = facade.executor();
    ///
    ///         // use executor
    ///
    ///         facade.resolve_null()
    ///     })
    /// }
    /// ```
    pub fn executor(&self) -> &Executor<GraphQLContext<RequestCtx>> {
        self.executor
    }

    /// Returns the parent GraphQL object of the field being resolved as a [`Node`]
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of variant [`TypeNotExpected`] if the parent object is not a node
    ///
    /// [`Error`]: ../../error/enum.Error.html
    /// [`TypeNotExpected`]: ../../error/enum.Error.html#variant.TypeNotExpected
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::juniper::BoxFuture;
    /// # use warpgrapher::juniper::{GraphQLType, GraphQLValue};
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let parent_node = facade.parent_node()?;
    ///         println!("Parent type: {:#?}",
    ///             parent_node.concrete_type_name(facade.executor().context(), facade.info()));
    ///
    ///         facade.resolve_null()
    ///     })
    /// }
    /// ```
    pub fn parent_node(&self) -> Result<&Node<RequestCtx>, Error> {
        if let Object::Node(n) = self.parent {
            Ok(n)
        } else {
            Err(Error::TypeNotExpected { details: None })
        }
    }

    /// Returns a GraphQL Null
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         // do work
    ///
    ///         // return null
    ///         facade.resolve_null()
    ///     })
    /// }
    /// ```
    pub fn resolve_null(&self) -> ExecutionResult {
        Ok(juniper::Value::Null)
    }

    /// Returns a GraphQL Scalar
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         // do work
    ///
    ///         // return string
    ///         facade.resolve_scalar("Hello")
    ///     })
    /// }
    /// ```
    pub fn resolve_scalar<T>(&self, v: T) -> ExecutionResult
    where
        T: std::convert::Into<juniper::DefaultScalarValue>,
    {
        Ok(juniper::Value::scalar::<T>(v))
    }

    /// Returns a GraphQL Scalar list
    ///
    /// # Examples
    /// ```rust, no_run
    /// use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         // do work
    ///
    ///         // return string
    ///         facade.resolve_scalar_list(vec![1, 2, 3])
    ///     })
    /// }
    /// ```
    pub fn resolve_scalar_list<T>(&self, v: Vec<T>) -> ExecutionResult
    where
        T: std::convert::Into<juniper::DefaultScalarValue>,
    {
        let x = v.into_iter().map(juniper::Value::scalar::<T>).collect();
        let list = juniper::Value::List(x);
        Ok(list)
    }

    /// Returns a GraphQL Object representing a graph node defined by a type and a map of props.
    ///
    /// # Examples
    /// ```rust, no_run
    /// use serde_json::json;
    /// use std::collections::HashMap;
    /// use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// use warpgrapher::engine::value::Value;
    /// use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         // do work
    ///         let mut hm = HashMap::new();
    ///         hm.insert("name".to_string(), Value::String("John Doe".to_string()));
    ///         hm.insert("age".to_string(), Value::Int64(21));
    ///
    ///         // return node
    ///         facade.resolve_node(&facade.create_node("User", hm)).await
    ///     })
    /// }
    /// ```
    pub async fn resolve_node(&self, node: &Node<RequestCtx>) -> ExecutionResult {
        self.executor
            .resolve_async(
                &Info::new(node.typename().to_string(), self.info.type_defs()),
                node,
            )
            .await
    }

    /// Returns a GraphQL Object representing a graph relationship defined by an ID, props, and a
    /// destination Warpgrapher Node.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         // do work
    ///         let node_id = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
    ///
    ///         let mut hm1 = HashMap::new();
    ///         hm1.insert("role".to_string(), Value::String("member".to_string()));
    ///
    ///         // return rel
    ///         facade.resolve_rel(&facade.create_rel(
    ///             Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///             Some(hm1), node_id, "DstNodeLabel")?).await
    ///     })
    /// }
    /// ```
    pub async fn resolve_rel(&self, rel: &Rel<RequestCtx>) -> ExecutionResult {
        let rel_name = self.info.name().to_string()
            + &((&self.field_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>())
            + "Rel";

        self.executor
            .resolve_async(&Info::new(rel_name, self.info.type_defs()), rel)
            .await
    }

    /// Returns a GraphQL Object array representing Warpgrapher Rels defined by an ID, props, and
    /// a destination Warpgrapher Node.
    ///
    /// # Examples
    /// ```rust, no_run
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///     // do work
    ///
    ///         let node_id1 = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
    ///         let node_id2 = Value::String("87654321-4321-4321-4321-1234567890ab".to_string());
    ///
    ///         let mut hm1 = HashMap::new();
    ///         hm1.insert("role".to_string(), Value::String("member".to_string()));
    ///
    ///         let mut hm2 = HashMap::new();
    ///         hm2.insert("role".to_string(), Value::String("leader".to_string()));
    ///
    ///         // return rel list
    ///         facade.resolve_rel_list(vec![
    ///             &facade.create_rel(
    ///                 Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///                 Some(hm1), node_id1, "DstNodeLabel")?,
    ///             &facade.create_rel(
    ///                 Value::String("713c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///                 Some(hm2), node_id2, "DstNodeLabel")?
    ///         ]).await
    ///     })
    /// }
    /// ```
    pub async fn resolve_rel_list(&self, rels: Vec<&Rel<RequestCtx>>) -> ExecutionResult {
        let object_name = self.info.name().to_string()
            + &((&self.field_name.to_string().to_title_case())
                .split_whitespace()
                .collect::<String>())
            + "Rel";

        self.executor
            .resolve_async(&Info::new(object_name, self.info.type_defs()), &rels)
            .await
    }

    /// Returns the request context
    ///
    /// # Examples
    /// ```rust, no_run
    ///
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn custom_resolve(context: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         if let Some(request_context) = context.request_context() {
    ///             // use request_context
    ///         }
    ///
    ///         context.resolve_null()
    ///     })
    /// }
    /// ```
    pub fn request_context(&self) -> Option<&RequestCtx> {
        self.executor.context().request_context()
    }
}
