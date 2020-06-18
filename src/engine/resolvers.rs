//! Contains the type aliases, enumerations, and structures to allow for the creation of custom
//! resolvers.

use crate::engine::context::GraphQLContext;
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::Error;
use inflector::Inflector;
use std::borrow::Cow;
use std::collections::HashMap;

pub use juniper::{Arguments, ExecutionResult, Executor};

/// Wraps a Node or Rel, and provides a type-safe distinction between the two, when passing the
/// object on which a field is being resolved to the custom resolver.
#[derive(Debug)]
pub enum Object<'a, GlobalCtx: GlobalContext, RequestCtx: RequestContext> {
    /// Wraps a [`Node`] being passed to a custom resolver
    ///
    /// [`Node`] ../objects/struct.Node.html
    Node(&'a Node<GlobalCtx, RequestCtx>),

    /// Wraps a [`Rel`] being passed to a custom resolver
    ///
    /// [`Rel`] ../objects/struct.Rel.html
    Rel(&'a Rel<'a, GlobalCtx, RequestCtx>),
}

/// Type alias for custom resolver functions. Takes a [`ResolverFacade`] and returns an
/// ExecutionResult.
///
/// [`ResolverFacade`]: ./struct.ResolverFacade.html
pub type ResolverFunc<GlobalCtx, RequestCtx> =
    fn(ResolverFacade<GlobalCtx, RequestCtx>) -> ExecutionResult;

/// Type alias for a mapping from a custom resolver name to a the Rust function that implements the
/// custom resolver.
pub type Resolvers<GlobalCtx, RequestCtx> =
    HashMap<String, Box<ResolverFunc<GlobalCtx, RequestCtx>>>;

/// Provides a simplified interface to primitive operations such as Node creation, Rel creation,
/// resolution of both scalar and complex types. The [`ResolverFacade`] is the primary mechanism
/// trough which a custom resolver interacts with the rest of the framework.
///
/// [`ResolverFacade`]: ./struct.ResolverFacade.html
pub struct ResolverFacade<'a, GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    field_name: String,
    info: &'a Info,
    args: &'a Arguments<'a>,
    parent: Object<'a, GlobalCtx, RequestCtx>,
    partition_key_opt: Option<&'a Value>,
    executor: &'a Executor<'a, GraphQLContext<GlobalCtx, RequestCtx>>,
}

impl<'a, GlobalCtx, RequestCtx> ResolverFacade<'a, GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    pub(crate) fn new(
        field_name: String,
        info: &'a Info,
        args: &'a Arguments,
        parent: Object<'a, GlobalCtx, RequestCtx>,
        partition_key_opt: Option<&'a Value>,
        executor: &'a Executor<GraphQLContext<GlobalCtx, RequestCtx>>,
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

    /// Returns the arguments provided to the resolver in the GraphQL query
    ///
    /// # Examples
    ///
    /// ```rust, norun
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
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
    /// ```rust, norun
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     let typename = "User";
    ///
    ///     let mut props = HashMap::new();
    ///     props.insert("role".to_string(), Value::String("Admin".to_string()));
    ///
    ///     let n = facade.create_node(typename, props);
    ///
    ///     facade.resolve_node(&n)
    /// }
    /// ```
    pub fn create_node(
        &self,
        typename: &str,
        props: HashMap<String, Value>,
    ) -> Node<GlobalCtx, RequestCtx> {
        Node::new(typename.to_string(), props)
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
    /// ```rust,norun
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     let id = Value::String("1e2ac081-b0a6-4f68-bc88-99bdc4111f00".to_string());
    ///     
    ///     let mut props = HashMap::new();
    ///     props.insert("since".to_string(), Value::String("2020-01-01".to_string()));
    ///
    ///     let dst = facade.create_node("User", HashMap::new());
    ///
    ///     let rel = facade.create_rel(id, Some(props), &dst)?;
    ///
    ///     facade.resolve_rel(&rel)
    /// }
    /// ```
    pub fn create_rel<'b>(
        &self,
        id: Value,
        props: Option<HashMap<String, Value>>,
        dst: &'b Node<GlobalCtx, RequestCtx>,
    ) -> Result<Rel<'b, GlobalCtx, RequestCtx>, Error>
    where
        'a: 'b,
    {
        if let Object::Node(parent_node) = self.parent {
            Ok(Rel::new(
                id,
                self.partition_key_opt.cloned(),
                props.map(|p| Node::new("props".to_string(), p)),
                Cow::Borrowed(parent_node),
                Cow::Borrowed(dst),
            ))
        } else {
            Err(Error::TypeNotExpected)
        }
    }

    /// Returns the [`Info`] struct containing the type schema for the GraphQL model.
    ///
    /// [`Info`]: ../schema/struct.Info.html
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     let info = facade.info();
    ///
    ///     // use info
    ///
    ///     facade.resolve_null()
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
    /// ```rust,norun
    /// # use warpgrapher::engine::resolvers::{Executor, ExecutionResult};
    /// # use warpgrapher::engine::resolvers::ResolverFacade;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     let exeuctor = facade.executor();
    ///
    ///     // use executor
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn executor(&self) -> &Executor<GraphQLContext<GlobalCtx, RequestCtx>> {
        self.executor
    }

    /// Returns the global context
    ///
    /// # Examples
    ///
    /// ```rust, norun
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     if let Some(global_context) = facade.global_context() {
    ///         // use global_context
    ///     }
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn global_context(&self) -> Option<&GlobalCtx> {
        self.executor.context().global_context()
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
    /// ```rust, norun
    /// # use warpgrapher::engine::objects::GraphQLType;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     let parent_node = facade.parent_node()?;
    ///     println!("Parent type: {:#?}",
    ///         parent_node.concrete_type_name(facade.executor().context(), facade.info()));
    ///
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn parent_node(&self) -> Result<&Node<GlobalCtx, RequestCtx>, Error> {
        if let Object::Node(n) = self.parent {
            Ok(n)
        } else {
            Err(Error::TypeNotExpected)
        }
    }

    /// Returns a GraphQL Null
    ///
    /// # Examples
    ///
    /// ```rust, norun
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return null
    ///     facade.resolve_null()
    /// }
    /// ```
    pub fn resolve_null(&self) -> ExecutionResult {
        Ok(juniper::Value::Null)
    }

    /// Returns a GraphQL Scalar
    ///
    /// # Examples
    ///
    /// ```rust, norun
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return string
    ///     facade.resolve_scalar("Hello")
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
    /// ```rust, norun
    /// use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return string
    ///     facade.resolve_scalar_list(vec![1, 2, 3])
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
    /// ```rust, norun
    /// use serde_json::json;
    /// use std::collections::HashMap;
    /// use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///     let mut hm = HashMap::new();
    ///     hm.insert("name".to_string(), Value::String("John Doe".to_string()));
    ///     hm.insert("age".to_string(), Value::Int64(21));
    ///
    ///     // return node
    ///     facade.resolve_node(&facade.create_node("User", hm))
    /// }
    /// ```
    pub fn resolve_node(&self, node: &Node<GlobalCtx, RequestCtx>) -> ExecutionResult {
        self.executor.resolve(
            &Info::new(node.typename().to_string(), self.info.type_defs()),
            node,
        )
    }

    /// Returns a GraphQL Object representing a graph relationship defined by an ID, props, and a
    /// destination Warpgrapher Node.
    ///
    /// # Examples
    ///
    /// ```rust, norun
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///     let mut hm1 = HashMap::new();
    ///     hm1.insert("role".to_string(), Value::String("member".to_string()));
    ///
    ///     let mut hm2 = HashMap::new();
    ///     hm2.insert("name".to_string(), Value::String("Jane Smith".to_string()));
    ///     hm2.insert("age".to_string(), Value::Int64(24));
    ///
    ///     // return rel
    ///     facade.resolve_rel(&facade.create_rel(
    ///         Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///         Some(hm1), &facade.create_node("user", hm2))?)
    /// }
    /// ```
    pub fn resolve_rel(&self, rel: &Rel<GlobalCtx, RequestCtx>) -> ExecutionResult {
        let rel_name =
            self.info.name().to_string() + &self.field_name.to_string().to_title_case() + "Rel";

        self.executor
            .resolve(&Info::new(rel_name, self.info.type_defs()), rel)
    }

    /// Returns a GraphQL Object array representing Warpgrapher Rels defined by an ID, props, and
    /// a destination Warpgrapher Node.
    ///
    /// # Examples
    /// ```rust, norun
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     // do work
    ///     let mut hm1 = HashMap::new();
    ///     hm1.insert("role".to_string(), Value::String("member".to_string()));
    ///
    ///     let mut hm2 = HashMap::new();
    ///     hm2.insert("name".to_string(), Value::String("John Doe".to_string()));
    ///     hm2.insert("age".to_string(), Value::Int64(21));
    ///
    ///     let mut hm3 = HashMap::new();
    ///     hm3.insert("role".to_string(), Value::String("leader".to_string()));
    ///
    ///     let mut hm4 = HashMap::new();
    ///     hm4.insert("name".to_string(), Value::String("Jane Smith".to_string()));
    ///     hm4.insert("age".to_string(), Value::Int64(24));
    ///
    ///     // return rel list
    ///     facade.resolve_rel_list(vec![
    ///         &facade.create_rel(
    ///             Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///             Some(hm1), &facade.create_node("User", hm2))?,
    ///         &facade.create_rel(
    ///             Value::String("713c4e13-5075-45ea-97de-b43f800e5854".to_string()),
    ///             Some(hm3), &facade.create_node("user", hm4))?
    ///     ])
    /// }
    /// ```
    pub fn resolve_rel_list(&self, rels: Vec<&Rel<GlobalCtx, RequestCtx>>) -> ExecutionResult {
        let object_name =
            self.info.name().to_string() + &self.field_name.to_string().to_title_case() + "Rel";

        self.executor
            .resolve(&Info::new(object_name, self.info.type_defs()), &rels)
    }

    /// Returns the request context
    ///
    /// # Examples
    /// ```rust, norun
    ///
    /// # use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade};
    ///
    /// fn custom_resolve(context: ResolverFacade<(), ()>) -> ExecutionResult {
    ///     if let Some(request_context) = context.request_context() {
    ///         // use request_context
    ///     }
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn request_context(&self) -> Option<&RequestCtx> {
        self.executor.context().request_context()
    }
}
