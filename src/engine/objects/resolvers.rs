#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use super::visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input, SuffixGenerator,
};
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use super::Input;
use super::{Node, Rel};
use crate::engine::context::{GraphQLContext, RequestContext};
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use crate::engine::database::{QueryResult, Transaction};
use crate::engine::objects::Object;
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::{Error, ErrorKind};
use core::hash::BuildHasher;
use inflector::Inflector;
use juniper::{Arguments, Executor, FieldError};
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use log::debug;
use log::{error, trace};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

pub use juniper::ExecutionResult;

pub type ResolverFunc<GlobalCtx, ReqCtx> =
    fn(ResolverContext<GlobalCtx, ReqCtx>) -> ExecutionResult;

pub type Resolvers<GlobalCtx, ReqCtx> = HashMap<String, Box<ResolverFunc<GlobalCtx, ReqCtx>>>;

#[derive(Clone, Debug)]
pub struct GraphNode<'a> {
    typename: &'a str,
    props: &'a HashMap<String, Value>,
}

impl<'a> GraphNode<'a> {
    pub fn new(typename: &'a str, props: &'a HashMap<String, Value>) -> GraphNode<'a> {
        GraphNode { typename, props }
    }
    pub fn typename(&self) -> &str {
        self.typename
    }
    pub fn props(&self) -> &HashMap<String, Value> {
        self.props
    }
}

#[derive(Clone, Debug)]
pub struct GraphRel<'a> {
    id: &'a str,
    props: Option<&'a HashMap<String, Value>>,
    dst: GraphNode<'a>,
}

impl<'a> GraphRel<'a> {
    pub fn new(
        id: &'a str,
        props: Option<&'a HashMap<String, Value>>,
        dst: GraphNode<'a>,
    ) -> GraphRel<'a> {
        GraphRel { id, props, dst }
    }
    pub fn id(&self) -> &str {
        self.id
    }
    pub fn props(&self) -> &Option<&'a HashMap<String, Value>> {
        &self.props
    }
    pub fn dst(&self) -> &'a GraphNode {
        &self.dst
    }
}

/// A Warpgrapher ResolverContext.
///
/// The [`ResolverContext`] struct is a collection of arguments and context
/// structs that are passed as input to a custom resolver.
pub struct ResolverContext<'a, GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + RequestContext,
{
    field_name: String,
    info: &'a Info,
    args: &'a Arguments<'a>,
    parent: Object<'a, GlobalCtx, ReqCtx>,
    executor: &'a Executor<'a, GraphQLContext<GlobalCtx, ReqCtx>>,
}

impl<'a, GlobalCtx, ReqCtx> ResolverContext<'a, GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + RequestContext,
{
    pub fn new(
        field_name: String,
        info: &'a Info,
        args: &'a Arguments,
        parent: Object<'a, GlobalCtx, ReqCtx>,
        executor: &'a Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    ) -> Self {
        ResolverContext {
            field_name,
            info,
            args,
            parent,
            executor,
        }
    }

    pub fn args(&self) -> &Arguments {
        self.args
    }

    pub fn info(&self) -> &Info {
        self.info
    }

    pub fn executor(&self) -> &Executor<GraphQLContext<GlobalCtx, ReqCtx>> {
        self.executor
    }

    /// Returns the global context, if the global context does not exist,
    /// it returns a FieldError.
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     let global_context = context.get_global_context()?;
    ///
    ///     // use global_context
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn get_global_context(&self) -> Result<&GlobalCtx, FieldError> {
        // TODO: make mutable
        match &self.executor.context().global_context() {
            None => {
                error!("Attempted to access non-existing global context");
                Err(FieldError::new(
                    "Unable to access global context.",
                    juniper::Value::Null,
                ))
            }
            Some(ctx) => Ok(ctx),
        }
    }
    /// Returns the request context, if the request context does not exist,
    /// it returns a FieldError.
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     let request_context = context.get_request_context()?;
    ///
    ///     // use request_context
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn get_request_context(&self) -> Result<&ReqCtx, FieldError> {
        // TODO: make mutable
        match &self.executor.context().request_context() {
            None => {
                error!("Attempted to access non-existing request context");
                Err(FieldError::new(
                    "Unable to access request context.",
                    juniper::Value::Null,
                ))
            }
            Some(ctx) => Ok(ctx),
        }
    }

    /// Returns the parent GraphQL object of the field being resolved as a [`Node`]
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::objects::GraphQLType;
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     let parent_node = context.get_parent_node()?;
    ///     println!("Parent type: {:#?}",
    ///         parent_node.concrete_type_name(context.executor().context(),
    ///             context.info()));
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn get_parent_node(&self) -> Result<&Node<GlobalCtx, ReqCtx>, FieldError> {
        match self.parent {
            Object::Node(n) => Ok(n),
            _ => Err(FieldError::new(
                "Unable to get parent node",
                juniper::Value::Null,
            )),
        }
    }

    /// Returns a GraphQL Null
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return null
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn resolve_null(&self) -> ExecutionResult {
        Ok(juniper::Value::Null)
    }

    /// Returns a GraphQL Scalar
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return string
    ///     context.resolve_scalar("Hello")
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
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return string
    ///     context.resolve_scalar_list(vec![1, 2, 3])
    /// }
    /// ```
    pub fn resolve_scalar_list<T>(&self, v: Vec<T>) -> ExecutionResult
    where
        T: std::convert::Into<juniper::DefaultScalarValue> + Clone,
    {
        /*
        //Ok(juniper::Value::scalar::<T>(v))
        let list : Vec<juniper::Value::Scalar> = v
            .iter()
            .map(|v| juniper::Value::Scalar::<T>(v));
        list
        */
        let x = v
            .iter()
            .map(|i| juniper::Value::scalar::<T>((*i).clone()))
            .collect();
        let list = juniper::Value::List(x);
        Ok(list)
        //Ok(juniper::Value::list(v))
    }

    /// Returns a GraphQL Object representing a graph node defined by
    /// a type and a map of props.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use std::collections::HashMap;
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult, GraphNode};
    /// use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///     let mut hm = HashMap::new();
    ///     hm.insert("name".to_string(), Value::String("John Doe".to_string()));
    ///     hm.insert("age".to_string(), Value::Int64(21));
    ///
    ///     // return node
    ///     context.resolve_node(GraphNode::new("user", &hm))
    /// }
    /// ```
    pub fn resolve_node(&self, node: GraphNode) -> ExecutionResult {
        self.executor.resolve(
            &Info::new(node.typename.to_string(), self.info.type_defs()),
            &Node::new(node.typename.to_string(), node.props().clone()),
        )
    }

    /*
    /// Returns a GraphQL Object array representing graph nodes defined by
    /// a type and a map of props.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult, GraphNode};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return node list
    ///     context.resolve_node_list(
    ///         vec![
    ///             GraphNode::new(
    ///                 "User",
    ///                 json!({
    ///                     "name": "John Doe",
    ///                     "age": 21
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             ),
    ///             GraphNode::new(
    ///                 "User",
    ///                 json!({
    ///                     "name": "Jane Smith",
    ///                     "age": 22
    ///                 })
    ///                 .as_object
    ///                 .unwrap()
    ///             )
    ///         ]
    ///     })
    /// }
    /// ```
    pub fn resolve_node_list(
        &self,
        nodes: Vec<GraphNode>
    ) -> ExecutionResult {
        let node_list : Vec<Node<GlobalCtx, ReqCtx>> = nodes
            .iter()
            .map(|node| {
                Node::new(
                    node.typename,
                    node.props
                )
            })
            .collect();
        // TODO: investigate the effect of returning a list of variable node types
        self.executor.resolve(
            &Info::new(object_name, self.info.type_defs.clone()),
            &node_list
        )
    }
    */

    /// Returns a GraphQL Object representing a graph relationship defined by
    /// an ID, props, and a destination Warpgrapher Node.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use std::collections::HashMap;
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult, GraphNode, GraphRel};
    /// use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///     let mut hm1 = HashMap::new();
    ///     hm1.insert("role".to_string(), Value::String("member".to_string()));
    /// 
    ///     let mut hm2 = HashMap::new();
    ///     hm2.insert("name".to_string(), Value::String("Jane Smith".to_string()));
    ///     hm2.insert("age".to_string(), Value::Int64(24));
    ///
    ///     // return rel
    ///     context.resolve_rel(GraphRel::new("655c4e13-5075-45ea-97de-b43f800e5854", Some(&hm1), GraphNode::new("user", &hm2)))
    /// }
    /// ```
    pub fn resolve_rel(&self, rel: GraphRel) -> ExecutionResult
    where
        GlobalCtx: Debug,
        ReqCtx: Debug + RequestContext,
    {
        let id = serde_json::Value::String(rel.id.to_string());

        let props = match &rel.props {
            None => None,
            Some(p) => Some(Node::new("props".to_string(), (*p).clone())),
        };

        let parent_node = match self.parent {
            Object::Node(n) => n.to_owned(),
            _ => {
                return Err(FieldError::new(
                    "Invalid parent passed",
                    juniper::Value::Null,
                ))
            }
        };

        let src = Node::new(
            parent_node.concrete_typename.clone(),
            parent_node.fields.clone(),
        );

        let dst = Node::new(rel.dst.typename.to_string(), rel.dst.props.clone());

        let r = Rel::new(id.try_into()?, props, src, dst);

        let object_name = format!(
            "{}{}{}",
            self.info.name().to_string(),
            self.field_name.to_owned().to_title_case(),
            "Rel".to_string()
        );
        self.executor
            .resolve(&Info::new(object_name, self.info.type_defs()), &r)
    }

    /// Returns a GraphQL Object array representing Warpgrapher Rels defined by
    /// an ID, props, and a destination Warpgrapher Node.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use std::collections::HashMap;
    /// use warpgrapher::engine::objects::resolvers::{ResolverContext, ExecutionResult, GraphNode, GraphRel};
    /// use warpgrapher::engine::value::Value;
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
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
    ///     context.resolve_rel_list(vec![
    ///         GraphRel::new("655c4e13-5075-45ea-97de-b43f800e5854", Some(&hm1), GraphNode::new("User", &hm2)),
    ///         GraphRel::new("713c4e13-5075-45ea-97de-b43f800e5854", Some(&hm3), GraphNode::new("user", &hm4))
    ///     ])
    /// }
    /// ```
    pub fn resolve_rel_list(&self, rels: Vec<GraphRel>) -> ExecutionResult
    where
        GlobalCtx: Debug,
        ReqCtx: Debug + RequestContext,
    {
        let object_name = format!(
            "{}{}{}",
            self.info.name().to_string(),
            self.field_name.to_string().to_title_case(),
            "Rel".to_string()
        );
        let parent_node = match self.parent {
            Object::Node(n) => n.to_owned(),
            _ => {
                return Err(FieldError::new(
                    "Invalid parent passed",
                    juniper::Value::Null,
                ))
            }
        };

        let rel_list: Vec<Rel<GlobalCtx, ReqCtx>> = rels
            .iter()
            .map(|rel| {
                Rel::new(
                    Value::String(rel.id.to_string()),
                    match &rel.props {
                        None => None,
                        Some(p) => Some(Node::new("props".to_string(), (*p).clone())),
                    },
                    Node::new(
                        parent_node.concrete_typename.clone(),
                        parent_node.fields.clone(),
                    ),
                    Node::new(rel.dst.typename.to_string(), rel.dst.props.clone()),
                )
            })
            .collect();

        self.executor
            .resolve(&Info::new(object_name, self.info.type_defs()), &rel_list)
    }
}

pub fn resolve_custom_endpoint<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    field_name: &str,
    parent: Object<GlobalCtx, ReqCtx>,
    args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_custom_endpoint called -- field_name: {}, info.name: {:#?}",
        field_name,
        info.name(),
    );

    // load resolver function
    let func = &executor.context().resolver(field_name)?;

    // TODO:
    // pluginHooks

    // results
    func(ResolverContext::new(
        field_name.to_string(),
        info,
        args,
        parent,
        executor,
    ))
}

pub fn resolve_custom_field<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    field_name: &str,
    resolver: &Option<String>,
    parent: Object<GlobalCtx, ReqCtx>,
    args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_custom_field called -- field_name: {:#?}, info.name: {:#?}",
        field_name,
        info.name(),
    );

    let resolver_name = resolver.as_ref().ok_or_else(|| {
        Error::new(
            ErrorKind::FieldMissingResolverError(
                format!(
                    "Failed to resolve custom field: {field_name}. Missing resolver name.",
                    field_name = field_name
                ),
                field_name.to_string(),
            ),
            None,
        )
    })?;

    let func = &executor.context().resolver(resolver_name)?;

    func(ResolverContext::new(
        field_name.to_string(),
        info,
        args,
        parent,
        executor,
    ))
}

pub fn resolve_custom_rel<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    rel_name: &str,
    resolver: &Option<String>,
    parent: Object<GlobalCtx, ReqCtx>,
    args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: Debug + RequestContext,
{
    trace!(
        "resolve_custom_rel called -- rel_name: {:#?}, info.name: {:#?}",
        rel_name,
        info.name(),
    );

    let resolver_name = resolver.as_ref().ok_or_else(|| {
        Error::new(
            ErrorKind::FieldMissingResolverError(
                format!(
                    "Failed to resolve custom rel: {rel_name}. Missing resolver name.",
                    rel_name = rel_name
                ),
                rel_name.to_string(),
            ),
            None,
        )
    })?;

    let func = &executor.context().resolver(resolver_name)?;

    func(ResolverContext::new(
        rel_name.to_string(),
        info,
        args,
        parent,
        executor,
    ))
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(super) fn resolve_node_create_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_node_create_mutation called -- info.name: {:#?}, field_name: {}",
        info.name(),
        field_name,
    );

    // let graph = executor.context().pool.client()?;
    let validators = &executor.context().validators();

    // let mut transaction = graph.transaction()?;
    // transaction.begin()?;

    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;

    let raw_result = visit_node_create_mutation_input(
        &p.type_name(),
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        validators,
        transaction,
    );

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;
    trace!("resolve_node_create_mutation Results: {:#?}", results);

    executor.resolve(
        &Info::new(p.type_name().to_owned(), info.type_defs()),
        &results.nodes("n", info)?.first(),
    )
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(super) fn resolve_node_delete_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    del_type: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_node_delete_mutation called -- info.name: {:#?}, field_name: {}",
        info.name(),
        field_name,
    );

    let mut sg = SuffixGenerator::new();
    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;
    let var_suffix = sg.suffix();

    transaction.begin()?;
    let raw_results = visit_node_delete_input(
        del_type,
        &var_suffix,
        &mut sg,
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        transaction,
    );
    trace!(
        "resolve_node_delete_mutation Raw results: {:#?}",
        raw_results
    );

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;

    executor.resolve_with_ctx(&(), &(results.count()? as i32))
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
fn resolve_node_read_query<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_node_read_query called -- field_name: {}, info.name: {:#?}, input_opt: {:#?}",
        field_name,
        info.name(),
        input_opt
    );

    let mut sg = SuffixGenerator::new();

    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;

    let var_suffix = sg.suffix();

    let mut params: HashMap<String, Value> = HashMap::new();

    transaction.begin()?;
    let query = visit_node_query_input(
        &p.type_name(),
        &var_suffix,
        false,
        true,
        // "",
        &mut params,
        &mut sg,
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input_opt.map(|i| i.value),
        transaction,
    )?;

    debug!(
        "resolve_node_read_query query, params: {:#?}, {:#?}",
        query, params
    );
    let raw_results = transaction.exec(&query, partition_key_opt, Some(params));
    debug!("resolve_node_read_query Raw result: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;

    if p.list() {
        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results.nodes(&(p.type_name().to_owned() + &var_suffix), info)?,
        )
    } else {
        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results
                .nodes(&(p.type_name().to_owned() + &var_suffix), info)?
                .first(),
        )
    }
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(super) fn resolve_node_update_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
        info.name(),
        field_name,
        input
    );

    let validators = &executor.context().validators();

    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;

    transaction.begin()?;

    let raw_result = visit_node_update_input(
        &p.type_name(),
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        validators,
        transaction,
    );
    trace!("resolve_node_update_mutation Raw Result: {:#?}", raw_result);

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;

    executor.resolve(
        &Info::new(p.type_name().to_owned(), info.type_defs()),
        &results.nodes("n", info)?,
    )
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(super) fn resolve_object_field<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    _id_opt: Option<&Value>,
    info: &Info,
    partition_key_opt: &Option<String>,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_object_field called -- info.name: {}, field_name: {}, input_opt: {:#?}",
        info.name(),
        field_name,
        input_opt
    );

    let td = info.type_def()?;
    let _p = td.prop(field_name)?;

    if td.type_name() == "Query" {
        resolve_node_read_query(
            field_name,
            info,
            partition_key_opt,
            input_opt,
            executor,
            transaction,
        )
    } else {
        Err(Error::new(
            ErrorKind::InvalidPropertyType("To be implemented.".to_owned()),
            None,
        )
        .into())
    }
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_rel_create_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label,
        rel_name, input
    );

    let validators = &executor.context().validators();

    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;
    let rtd = info.type_def_by_name(p.type_name())?;

    let raw_result = visit_rel_create_input(
        src_label,
        rel_name,
        // The conversion from Error to None using ok() is actually okay here,
        // as it's expected that some relationship types may not have props defined
        // in their schema, in which case the missing property is fine.
        rtd.prop("props").map(|pp| pp.type_name()).ok(),
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        validators,
        transaction,
    );

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let rels = raw_result?;
    trace!("resolve_rel_create_mutation Rels: {:#?}", rels);

    let mutations = info.type_def_by_name("Mutation")?;
    let endpoint_td = mutations.prop(field_name)?;

    if endpoint_td.list() {
        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &rels,
        )
    } else {
        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &rels[0],
        )
    }
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_rel_delete_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name, input
    );

    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;

    let raw_results = visit_rel_delete_input(
        src_label,
        None,
        rel_name,
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        transaction,
    );
    trace!("Raw results: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;

    executor.resolve_with_ctx(&(), &(results.count()? as i32))
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_rel_field<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    id_opt: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_rel_field called -- info.name: {}, field_name: {}, id_opt: {:#?}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
        id_opt,
        rel_name,
        partition_key_opt,
        input_opt
    );

    let td = info.type_def()?;
    let _p = td.prop(field_name)?;

    resolve_rel_read_query(
        field_name,
        id_opt,
        rel_name,
        info,
        partition_key_opt,
        input_opt,
        executor,
        transaction,
    )
}

pub(super) fn resolve_rel_props<GlobalCtx, ReqCtx>(
    info: &Info,
    field_name: &str,
    props: &Node<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
{
    trace!(
        "resolve_rel_props called -- info.name: {:#?}, field_name: {}",
        info.name(),
        field_name,
    );

    let td = info.type_def()?;
    let p = td.prop(field_name)?;

    executor.resolve(
        &Info::new(p.type_name().to_owned(), info.type_defs()),
        props,
    )
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[allow(clippy::too_many_arguments)]
fn resolve_rel_read_query<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    src_ids_opt: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, src_ids: {:#?}, rel_name: {}, partition_key_opt: {:#?}, input_opt: {:#?}",
        info.name(),
        field_name,
        src_ids_opt,
        rel_name,
        partition_key_opt,
        input_opt
    );

    let mut sg = SuffixGenerator::new();
    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;
    let rtd = info.type_def_by_name(&p.type_name())?;
    let props_prop = rtd.prop("props");
    let src_prop = rtd.prop("src")?;
    let dst_prop = rtd.prop("dst")?;

    let mut params: HashMap<String, Value> = HashMap::new();

    let src_suffix = sg.suffix();
    let dst_suffix = sg.suffix();

    let query = visit_rel_query_input(
        &src_prop.type_name(),
        &src_suffix,
        src_ids_opt,
        rel_name,
        &dst_prop.type_name(),
        &dst_suffix,
        true,
        // "",
        &mut params,
        &mut sg,
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input_opt.map(|i| i.value),
        transaction,
    )?;

    debug!(
        "resolve_rel_read_query Query query, params: {:#?} {:#?}",
        query, params
    );
    let raw_results = transaction.exec(&query, partition_key_opt, Some(params));
    // debug!("resolve_rel_read_query Raw result: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;
    // trace!("resolve_rel_read_query Results: {:#?}", results);

    trace!("resolve_rel_read_query calling rels.");
    if p.list() {
        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &results.rels(
                &src_prop.type_name(),
                &src_suffix,
                rel_name,
                &dst_prop.type_name(),
                &dst_suffix,
                props_prop.map(|_| p.type_name()).ok(),
                info,
            )?,
        )
    } else {
        let v = results.rels(
            &src_prop.type_name(),
            &src_suffix,
            rel_name,
            &dst_prop.type_name(),
            &dst_suffix,
            props_prop.map(|_| p.type_name()).ok(),
            info,
        )?;

        if v.len() > 1 {
            return Err(Error::new(
                ErrorKind::InvalidType(
                    "Multiple results for a single-node relationship.".to_string(),
                ),
                None,
            )
            .into());
        }

        executor.resolve(
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            &v.first(),
        )
    }
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_rel_update_mutation<GlobalCtx, ReqCtx, T>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
    transaction: &mut T,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name(),
        field_name,
        src_label, rel_name,
        input
    );

    let validators = &executor.context().validators();
    let td = info.type_def()?;
    let p = td.prop(field_name)?;
    let itd = p.input_type_definition(info)?;
    let rtd = info.type_def_by_name(&p.type_name())?;
    let props_prop = rtd.prop("props");
    let src_prop = rtd.prop("src")?;
    // let dst_prop = rtd.prop("dst")?;

    let raw_result = visit_rel_update_input(
        src_label,
        None,
        rel_name,
        &Info::new(itd.type_name().to_owned(), info.type_defs()),
        partition_key_opt,
        input.value,
        validators,
        transaction,
    );
    trace!("Raw Result: {:#?}", raw_result);

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;

    trace!("resolve_rel_update_mutation calling rels");
    executor.resolve(
        &Info::new(p.type_name().to_owned(), info.type_defs()),
        &results.rels(
            &src_prop.type_name(),
            "",
            rel_name,
            "dst",
            "",
            props_prop.map(|_| p.type_name()).ok(),
            info,
        )?,
    )
}

pub(super) fn resolve_scalar_field<GlobalCtx, ReqCtx, S: BuildHasher>(
    info: &Info,
    field_name: &str,
    fields: &HashMap<String, Value, S>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
{
    trace!(
        "resolve_scalar_field called -- info.name: {}, field_name: {}",
        info.name(),
        field_name,
    );

    fields.get(field_name).map_or_else(
        || {
            if field_name == "id" {
                Err(Error::new(ErrorKind::MissingProperty("id".to_owned(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None).into())
            } else {
                executor.resolve_with_ctx(&(), &None::<String>)
            }
        },
        |v| match v {
            Value::Null => executor.resolve_with_ctx(&(), &None::<String>),
            Value::Bool(_) => executor.resolve_with_ctx(&(), &TryInto::<bool>::try_into(v.clone())?),
            Value::Int64(_) | Value::UInt64(_) => executor.resolve_with_ctx(&(), &TryInto::<i32>::try_into(v.clone())?),
            Value::Float64(_) => executor.resolve_with_ctx(&(), &TryInto::<f64>::try_into(v.clone())?),
            Value::String(_) => executor.resolve_with_ctx(&(), &TryInto::<String>::try_into(v.clone())?),
            Value::Array(a) => match a.get(0) {
                Some(Value::Null) | Some(Value::String(_)) => executor.resolve_with_ctx(&(), &TryInto::<Vec<String>>::try_into(v.clone())?),
                Some(Value::Bool(_)) => executor.resolve_with_ctx(&(), &TryInto::<Vec<bool>>::try_into(v.clone())?),
                Some(Value::Int64(_)) | Some(Value::UInt64(_)) | Some(Value::Float64(_)) => {
                    let r = TryInto::<Vec<i32>>::try_into(v.clone());
                    if r.is_ok() {
                        executor.resolve_with_ctx(&(), &r?)
                    } else {
                        executor.resolve_with_ctx(&(), &TryInto::<Vec<f64>>::try_into(v.clone())?)
                    }
                }
                Some(Value::Array(_)) | Some(Value::Map(_)) | None => Err(Error::new(ErrorKind::InvalidPropertyType(String::from(field_name) + " is a non-scalar array. Expected a scalar or a scalar array."), None).into()),
            },
            Value::Map(_) => Err(Error::new(
                ErrorKind::InvalidPropertyType(
                    String::from(field_name) + " is an object. Expected a scalar or a scalar array.",
                ),
                None,
            )
            .into()),
        },
    )
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
pub(super) fn resolve_static_version_query<GlobalCtx, ReqCtx>(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
{
    match &executor.context().version() {
        Some(v) => Ok(juniper::Value::scalar(v.clone())),
        None => Ok(juniper::Value::Null),
    }
}

pub(super) fn resolve_union_field<GlobalCtx, ReqCtx>(
    info: &Info,
    field_name: &str,
    src: &Node<GlobalCtx, ReqCtx>,
    dst: &Node<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    GlobalCtx: Debug,
    ReqCtx: RequestContext,
{
    trace!(
        "resolve_union_field called -- info.name: {}, field_name: {}, src: {}, dst: {}",
        info.name(),
        field_name,
        src.concrete_typename,
        dst.concrete_typename
    );

    match field_name {
        "dst" => executor.resolve(
            &Info::new(dst.concrete_typename.to_owned(), info.type_defs()),
            dst,
        ),
        "src" => executor.resolve(
            &Info::new(src.concrete_typename.to_owned(), info.type_defs()),
            src,
        ),
        _ => Err(Error::new(
            ErrorKind::InvalidProperty(String::from(info.name()) + "::" + field_name),
            None,
        )
        .into()),
    }
}
