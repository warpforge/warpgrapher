use super::context::{GraphQLContext, RequestContext};
use super::objects::{Input, Node, Object, Rel};
use super::schema::Info;
use super::visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input, SuffixGenerator,
};
use crate::error::{Error, ErrorKind};
use inflector::Inflector;
use juniper::{Arguments, Executor, FieldError};
use log::{debug, trace};
use r2d2_cypher::CypherConnectionManager;
use rusted_cypher::Statement;
use serde_json::Map;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

pub use juniper::ExecutionResult;

pub type ResolverFunc<GlobalCtx, ReqCtx> =
    fn(ResolverContext<GlobalCtx, ReqCtx>) -> ExecutionResult;

pub type Resolvers<GlobalCtx, ReqCtx> = HashMap<String, Box<ResolverFunc<GlobalCtx, ReqCtx>>>;

#[derive(Clone, Debug)]
pub struct GraphNode<'a> {
    pub typename: &'a str,
    pub props: &'a serde_json::map::Map<String, Value>,
}

#[derive(Clone, Debug)]
pub struct GraphRel<'a> {
    pub id: &'a str,
    pub props: Option<&'a serde_json::map::Map<String, Value>>,
    pub dst: GraphNode<'a>,
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
    pub field_name: String,
    pub info: &'a Info,
    pub args: &'a Arguments<'a>,
    pub parent: Object<'a, GlobalCtx, ReqCtx>,
    pub executor: &'a Executor<'a, GraphQLContext<GlobalCtx, ReqCtx>>,
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

    /// Returns a handle to the database from the connection pool.
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     let db = context.get_db()?;
    ///     // execute db queries
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn get_db(&self) -> Result<r2d2::PooledConnection<CypherConnectionManager>, FieldError> {
        self.executor.context().pool.get().map_err(|_| {
            FieldError::new(
                "Unable to access database driver pool.",
                juniper::Value::Null,
            )
        })
    }

    /// Returns the parent GraphQL object of the field being resolved as a [`Node`]
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     let parent_node = context.get_parent_node()?;
    ///     println!("Parent type: {:#?}", parent_node.concrete_typename);
    ///
    ///     context.resolve_null()
    /// }
    /// ```
    pub fn get_parent_node(&self) -> Result<&Node<GlobalCtx, ReqCtx>, FieldError> {
        match self.parent {
            Object::Node(n) => Ok(n),
            _ => {
                Err(FieldError::new(
                    "Unable to get parent node",
                    juniper::Value::Null,
                ))
            }
        }
    }

    /// Returns a GraphQL Null
    ///
    /// # Examples
    /// ```rust, norun
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};
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
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};
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
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};
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
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult, GraphNode};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return node
    ///     context.resolve_node(
    ///         GraphNode {
    ///             typename: "User",
    ///             props: json!({
    ///                 "name": "John Doe",
    ///                 "age": 21
    ///             })
    ///             .as_object()
    ///             .unwrap()
    ///         }
    ///     )
    /// }
    /// ```
    pub fn resolve_node(&self, node: GraphNode) -> ExecutionResult {
        self.executor.resolve(
            &Info::new(node.typename.to_string(), self.info.type_defs.clone()),
            &Node::new(node.typename.to_string(), node.props.clone()),
        )
    }

    /*
    /// Returns a GraphQL Object array representing graph nodes defined by
    /// a type and a map of props.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult, GraphNode};
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
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult, GraphNode, GraphRel};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return rel
    ///     context.resolve_rel(
    ///         GraphRel {
    ///             id: "655c4e13-5075-45ea-97de-b43f800e5854",
    ///             props: Some(
    ///                 json!({
    ///                     "role": "member",
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             ),
    ///             dst: GraphNode {
    ///                 typename: "User",
    ///                 props: json!({
    ///                     "name": "Jane Smith",
    ///                      "age": 24
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             }
    ///         }
    ///     )
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

        let r = Rel::new(id, props, src, dst);

        let object_name = format!(
            "{}{}{}",
            self.info.name.to_owned(),
            self.field_name.to_owned().to_title_case(),
            "Rel".to_string()
        );
        self.executor
            .resolve(&Info::new(object_name, self.info.type_defs.clone()), &r)
    }

    /// Returns a GraphQL Object array representing Warpgrapher Rels defined by
    /// an ID, props, and a destination Warpgrapher Node.
    ///
    /// # Examples
    /// ```rust, norun
    /// use serde_json::json;
    /// use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult, GraphNode, GraphRel};
    ///
    /// fn custom_resolve(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     // do work
    ///
    ///     // return rel list
    ///     context.resolve_rel_list(vec![
    ///         GraphRel {
    ///             id: "655c4e13-5075-45ea-97de-b43f800e5854",
    ///             props: Some(
    ///                 json!({
    ///                     "role": "member",
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             ),
    ///             dst: GraphNode {
    ///                 typename: "User",
    ///                 props: json!({
    ///                     "name": "John Does",
    ///                     "age": 21
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             }
    ///         },
    ///         GraphRel {
    ///             id: "655c4e13-5075-45ea-97de-b43f800e5854",
    ///             props: Some(
    ///                 json!({
    ///                     "role": "leader",
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             ),
    ///             dst: GraphNode {
    ///                 typename: "User",
    ///                 props: json!({
    ///                     "name": "Jane Smith",
    ///                     "age": 24
    ///                 })
    ///                 .as_object()
    ///                 .unwrap()
    ///             }
    ///         }
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
            self.info.name.to_owned(),
            self.field_name.to_owned().to_title_case(),
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
                    serde_json::Value::String(rel.id.to_string()),
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

        self.executor.resolve(
            &Info::new(object_name, self.info.type_defs.clone()),
            &rel_list,
        )
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
        info.name,
    );

    // load resolver function
    let resolvers = &executor.context().resolvers;

    let func = resolvers.get(field_name).ok_or_else(|| {
        Error::new(
            ErrorKind::ResolverNotFound(
                format!(
                    "Could not find custom endpoint resolver {field_name}.",
                    field_name = field_name
                ),
                field_name.to_owned(),
            ),
            None,
        )
    })?;

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
        info.name,
    );

    let resolvers = &executor.context().resolvers;

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

    let func = resolvers.get(resolver_name).ok_or_else(|| {
        Error::new(
            ErrorKind::ResolverNotFound(
                format!(
                    "Could not find resolver {resolver_name} for field {field_name}.",
                    resolver_name = resolver_name,
                    field_name = field_name
                ),
                resolver_name.to_owned(),
            ),
            None,
        )
    })?;

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
        info.name,
    );

    let resolvers = &executor.context().resolvers;

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

    let func = resolvers.get(resolver_name).ok_or_else(|| {
        Error::new(
            ErrorKind::ResolverNotFound(
                format!(
                    "Could not find resolver {resolver_name} for rel {rel_name}.",
                    resolver_name = resolver_name,
                    rel_name = rel_name
                ),
                resolver_name.to_owned(),
            ),
            None,
        )
    })?;

    func(ResolverContext::new(
        rel_name.to_string(),
        info,
        args,
        parent,
        executor,
    ))
}

pub fn resolve_node_create_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_node_create_mutation called -- info.name: {:#?}, field_name: {}",
        info.name,
        field_name,
    );

    let graph = executor.context().pool.get()?;
    let validators = &executor.context().validators;

    let mut transaction = graph.transaction().begin()?.0;

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;

    let raw_result = visit_node_create_mutation_input(
        &p.type_name,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        validators,
        &mut transaction,
    );

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;
    trace!("resolve_node_create_mutation Results: {:#?}", results);

    executor.resolve(
        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
        &Node::new(
            p.type_name.to_owned(),
            results
                .rows()
                .next()
                .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?
                .get("n")?,
        ),
    )
}

pub fn resolve_node_delete_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    del_type: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_node_delete_mutation called -- info.name: {:#?}, field_name: {}",
        info.name,
        field_name,
    );

    let graph = executor.context().pool.get()?;
    let mut transaction = graph.transaction().begin()?.0;
    let mut sg = SuffixGenerator::new();

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;

    let var_suffix = sg.get_suffix();

    let raw_results = visit_node_delete_input(
        del_type,
        &var_suffix,
        &mut sg,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        &mut transaction,
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

    let ret_row = results
        .rows()
        .next()
        .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;

    let ret_val = ret_row
        .get("count")
        .map_err(|_| Error::new(ErrorKind::MissingResultElement("count".to_owned()), None))?;

    if let Value::Number(n) = ret_val {
        if let Some(i_val) = n.as_i64() {
            executor.resolve_with_ctx(&(), &(i_val as i32))
        } else {
            Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
        }
    } else {
        Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
    }
}

pub fn resolve_node_read_query<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    info: &Info,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_node_read_query called -- field_name: {}, info.name: {:#?}, input_opt: {:#?}",
        field_name,
        info.name,
        input_opt
    );

    let graph = executor.context().pool.get()?;
    let mut transaction = graph.transaction().begin()?.0;
    let mut sg = SuffixGenerator::new();

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;

    let var_suffix = sg.get_suffix();

    let mut params = BTreeMap::new();

    let query = visit_node_query_input(
        &p.type_name,
        &var_suffix,
        false,
        true,
        "",
        &mut params,
        &mut sg,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        input_opt.as_ref().map(|i| &i.value),
    )?;

    let mut statement = Statement::new(query);
    statement.set_parameters(&params)?;
    debug!("resolve_node_read_query Query: {:#?}", statement);
    let raw_results = transaction.exec(statement);
    debug!("resolve_node_read_query Raw result: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;

    if p.list {
        let mut v: Vec<Node<GlobalCtx, ReqCtx>> = Vec::new();
        for row in results.rows() {
            v.push(Node::new(
                p.type_name.to_owned(),
                row.get(&(p.type_name.to_owned() + &var_suffix))?,
            ))
        }

        executor.resolve(
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            &v,
        )
    } else {
        let row = results
            .rows()
            .next()
            .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;
        executor.resolve(
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            &Node::new(
                p.type_name.to_owned(),
                row.get(&(p.type_name.to_owned() + &var_suffix))?,
            ),
        )
    }
}

pub fn resolve_node_update_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_node_update_mutation called -- info.name: {:#?}, field_name: {}, input: {:#?}",
        info.name,
        field_name,
        input
    );

    let graph = executor.context().pool.get()?;
    let validators = &executor.context().validators;

    let mut transaction = graph.transaction().begin()?.0;

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;

    let raw_result = visit_node_update_input(
        &p.type_name,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        validators,
        &mut transaction,
    );
    trace!("resolve_node_update_mutation Raw Result: {:#?}", raw_result);

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;

    let mut v: Vec<Node<GlobalCtx, ReqCtx>> = Vec::new();
    for row in results.rows() {
        v.push(Node::new(p.type_name.to_owned(), row.get("n")?))
    }

    executor.resolve(
        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
        &v,
    )
}

pub fn resolve_object_field<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    _id_opt: Option<&Value>,
    info: &Info,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_object_field called -- info.name: {}, field_name: {}, input_opt: {:#?}",
        info.name,
        field_name,
        input_opt
    );

    let td = info.get_type_def()?;
    let _p = td.get_prop(field_name)?;

    if td.type_name == "Query" {
        resolve_node_read_query(field_name, info, input_opt, executor)
    } else {
        Err(Error::new(
            ErrorKind::InvalidPropertyType("To be implemented.".to_owned()),
            None,
        )
        .into())
    }
}

pub fn resolve_rel_create_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_create_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name,
        field_name,
        src_label,
        rel_name, input
    );

    let graph = executor.context().pool.get()?;
    let validators = &executor.context().validators;

    let mut transaction = graph.transaction().begin()?.0;

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;
    let rtd = info.get_type_def_by_name(&p.type_name)?;

    let raw_result = visit_rel_create_input(
        src_label,
        rel_name,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        validators,
        &mut transaction,
    );

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;
    trace!("resolve_rel_create_mutation Results: {:#?}", results);

    let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
    for row in results.rows() {
        if let Value::Array(labels) = row.get("b_label")? {
            if let Value::String(dst_type) = &labels[0] {
                v.push(
                    Rel::new(
                        row.get::<Value>("r")?
                            .get("id")
                            .ok_or_else(|| {
                                Error::new(ErrorKind::MissingProperty("id".to_string(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None)
                            })?
                            .to_owned(),
                        match rtd.get_prop("props") {
                            Ok(pp) => Some(Node::new(pp.type_name.to_owned(), row.get("r")?)),
                            Err(_e) => None,
                        },
                        Node::new(rtd.get_prop("src")?.type_name.to_owned(), row.get("a")?),
                        Node::new(dst_type.to_string(), row.get("b")?),
                    ),
                );
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidPropertyType(String::from("b_label")),
                    None,
                )
                .into());
            }
        } else {
            return Err(Error::new(
                ErrorKind::InvalidPropertyType(String::from("b_label")),
                None,
            )
            .into());
        }
    }

    let mutations = match info.type_defs.get("Mutation") {
        Some(v) => v,
        None => {
            return Err(Error::new(
                ErrorKind::MissingSchemaElement("Mutation".to_string()),
                None,
            )
            .into());
        }
    };
    let endpoint_td = match mutations.props.get(field_name) {
        Some(v) => v,
        None => {
            return Err(Error::new(
                ErrorKind::MissingSchemaElement("Mutation".to_string()),
                None,
            )
            .into());
        }
    };

    if endpoint_td.list {
        executor.resolve(
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            &v,
        )
    } else {
        executor.resolve(
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            &v[0],
        )
    }
}

pub fn resolve_rel_delete_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_delete_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name,
        field_name,
        src_label, rel_name, input
    );

    let graph = executor.context().pool.get()?;
    let mut transaction = graph.transaction().begin()?.0;

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;

    let raw_results = visit_rel_delete_input(
        src_label,
        None,
        rel_name,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        &mut transaction,
    );
    trace!("Raw results: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;

    let ret_row = results
        .rows()
        .next()
        .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;

    let ret_val = ret_row
        .get("count")
        .map_err(|_| Error::new(ErrorKind::MissingResultElement("count".to_owned()), None))?;

    if let Value::Number(n) = ret_val {
        if let Some(i_val) = n.as_i64() {
            executor.resolve_with_ctx(&(), &(i_val as i32))
        } else {
            Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
        }
    } else {
        Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
    }
}

pub fn resolve_rel_field<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    id_opt: Option<&Value>,
    rel_name: &str,
    info: &Info,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_field called -- info.name: {}, field_name: {}, id_opt: {:#?}, rel_name: {}, input_opt: {:#?}",
        info.name,
        field_name,
        id_opt,
        rel_name,
        input_opt
    );

    let td = info.get_type_def()?;
    let _p = td.get_prop(field_name)?;

    if let Some(Value::String(id)) = id_opt {
        resolve_rel_read_query(
            field_name,
            Some(&[id.to_owned()]),
            rel_name,
            info,
            input_opt,
            executor,
        )
    } else {
        resolve_rel_read_query(field_name, None, rel_name, info, input_opt, executor)
    }
}

pub fn resolve_rel_props<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    field_name: &str,
    props: &Node<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_props called -- info.name: {:#?}, field_name: {}",
        info.name,
        field_name,
    );

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;

    executor.resolve(
        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
        props,
    )
}

pub fn resolve_rel_read_query<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    src_ids_opt: Option<&[String]>,
    rel_name: &str,
    info: &Info,
    input_opt: Option<Input<GlobalCtx, ReqCtx>>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_read_query called -- info.name: {:#?}, field_name: {}, src_ids: {:#?}, rel_name: {}, input_opt: {:#?}",
        info.name,
        field_name,
        src_ids_opt,
        rel_name,
        input_opt
    );

    let graph = executor.context().pool.get()?;
    let mut transaction = graph.transaction().begin()?.0;
    let mut sg = SuffixGenerator::new();

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;
    let rtd = info.get_type_def_by_name(&p.type_name)?;
    let props_prop = rtd.get_prop("props");
    let src_prop = rtd.get_prop("src")?;
    let dst_prop = rtd.get_prop("dst")?;

    let mut params = BTreeMap::new();

    let src_suffix = sg.get_suffix();
    let dst_suffix = sg.get_suffix();

    let query = visit_rel_query_input(
        &src_prop.type_name,
        &src_suffix,
        src_ids_opt,
        rel_name,
        &dst_prop.type_name,
        &dst_suffix,
        true,
        "",
        &mut params,
        &mut sg,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        input_opt.as_ref().map(|i| &i.value),
    )?;

    let mut statement = Statement::new(query);
    statement.set_parameters(&params)?;
    debug!("resolve_rel_read_query Query: {:#?}", statement);
    let raw_results = transaction.exec(statement);
    debug!("resolve_rel_read_query Raw result: {:#?}", raw_results);

    if raw_results.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_results?;
    trace!("resolve_rel_read_query Results: {:#?}", results);

    if p.list {
        let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();

        for row in results.rows() {
            if let Value::Array(labels) =
                row.get(&(String::from(&dst_prop.type_name) + &dst_suffix + "_label"))?
            {
                if let Value::String(dst_type) = &labels[0] {
                    v.push(Rel::new(
                        row.get::<Value>(&(String::from(rel_name) + &src_suffix + &dst_suffix))?
                            .get("id")
                            .ok_or_else(|| {
                                Error::new(ErrorKind::MissingResultElement("id".to_string()), None)
                            })?
                            .to_owned(),
                        match &props_prop {
                            Ok(p) => Some(Node::new(
                                p.type_name.to_owned(),
                                row.get(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
                            )),
                            Err(_e) => None,
                        },
                        Node::new(
                            src_prop.type_name.to_owned(),
                            row.get(&(String::from(&src_prop.type_name) + &src_suffix))?,
                        ),
                        Node::new(
                            dst_type.to_owned(),
                            row.get(&(String::from(&dst_prop.type_name) + &dst_suffix))?,
                        ),
                    ))
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(
                            String::from(&dst_prop.type_name) + &dst_suffix + "_label",
                        ),
                        None,
                    )
                    .into());
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidPropertyType(
                        String::from(&dst_prop.type_name) + &dst_suffix + "_label",
                    ),
                    None,
                )
                .into());
            };
        }

        executor.resolve(
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            &v,
        )
    } else {
        let row = results
            .rows()
            .next()
            .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;

        if let Value::Array(labels) =
            row.get(&(String::from(&dst_prop.type_name) + &dst_suffix + "_label"))?
        {
            if let Value::String(dst_type) = &labels[0] {
                executor.resolve(
                    &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                    &Rel::new(
                        row.get::<Value>(&(String::from(rel_name) + &src_suffix + &dst_suffix))?
                            .get("id")
                            .ok_or_else(|| {
                                Error::new(ErrorKind::MissingProperty("id".to_string(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None)
                            })?
                            .to_owned(),
                        match props_prop {
                            Ok(pp) => Some(Node::new(
                                pp.type_name.to_owned(),
                                row.get(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
                            )),
                            Err(_e) => None,
                        },
                        Node::new(
                            src_prop.type_name.to_owned(),
                            row.get(&(String::from(&src_prop.type_name) + &src_suffix))?,
                        ),
                        Node::new(
                            dst_type.to_string(),
                            row.get(&(String::from(&dst_prop.type_name) + &dst_suffix))?,
                        ),
                    ),
                )
            } else {
                Err(Error::new(
                    ErrorKind::InvalidPropertyType(
                        String::from(&dst_prop.type_name) + &dst_suffix + "_label",
                    ),
                    None,
                )
                .into())
            }
        } else {
            Err(Error::new(
                ErrorKind::InvalidPropertyType(
                    String::from(&dst_prop.type_name) + &dst_suffix + "_label",
                ),
                None,
            )
            .into())
        }
    }
}

pub fn resolve_rel_update_mutation<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    field_name: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    input: Input<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_rel_update_mutation called -- info.name: {:#?}, field_name: {}, src_label: {}, rel_name: {}, input: {:#?}",
        info.name,
        field_name,
        src_label, rel_name,
        input
    );

    let graph = executor.context().pool.get()?;
    let validators = &executor.context().validators;

    let mut transaction = graph.transaction().begin()?.0;

    let td = info.get_type_def()?;
    let p = td.get_prop(field_name)?;
    let itd = p.get_input_type_definition(info)?;
    let rtd = info.get_type_def_by_name(&p.type_name)?;

    let raw_result = visit_rel_update_input(
        src_label,
        None,
        rel_name,
        &Info::new(itd.type_name.to_owned(), info.type_defs.clone()),
        &input.value,
        validators,
        &mut transaction,
    );
    trace!("Raw Result: {:#?}", raw_result);

    if raw_result.is_ok() {
        transaction.commit()?;
    } else {
        transaction.rollback()?;
    }

    let results = raw_result?;

    let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
    for row in results.rows() {
        if let Value::Array(labels) = row.get("b_label")? {
            if let Value::String(dst_type) = &labels[0] {
                v.push(Rel::new(
                    row.get::<Value>("r")?
                        .get("id")
                        .ok_or_else(|| {
                            Error::new(ErrorKind::MissingProperty("id".to_string(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None)
                        })?
                        .to_owned(),
                    match rtd.get_prop("props") {
                        Ok(pp) => Some(Node::new(pp.type_name.to_owned(), row.get("r")?)),
                        Err(_e) => None,
                    },
                    Node::new(rtd.get_prop("src")?.type_name.to_owned(), row.get("a")?),
                    Node::new(dst_type.to_string(), row.get("b")?),
                ))
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidPropertyType("b_label".to_string()),
                    None,
                )
                .into());
            }
        } else {
            return Err(
                Error::new(ErrorKind::InvalidPropertyType("b_label".to_string()), None).into(),
            );
        };
    }

    executor.resolve(
        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
        &v,
    )
}

pub fn resolve_scalar_field<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    field_name: &str,
    fields: &Map<String, Value>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_scalar_field called -- info.name: {}, field_name: {}",
        info.name,
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
            Value::Bool(b) => executor.resolve_with_ctx(&(), b),
            Value::Number(n) => {
                if let Some(i_val) = n.as_i64() {
                    executor.resolve_with_ctx(&(), &(i_val as i32))
                } else if n.is_f64() {
                    executor.resolve_with_ctx(
                        &(),
                        &n.as_f64().ok_or_else(|| {
                            Error::new(ErrorKind::InvalidPropertyType("f64".to_owned()), None)
                        })?,
                    )
                } else {
                    Err(Error::new(
                        ErrorKind::InvalidPropertyType(
                            "Could not convert numeric type.".to_owned(),
                        ),
                        None,
                    )
                    .into())
                }
            }
            Value::String(s) => executor.resolve_with_ctx(&(), s),
            Value::Array(a) => {
                match &a.get(0) {
                    None => {
                        executor.resolve_with_ctx(&(), &(vec![] as Vec<String>))
                    },
                    Some(v) if v.is_string() => {
                        let array : Vec<String> = a.iter().map(|x| x.as_str().unwrap().to_string()).collect();
                        executor.resolve_with_ctx(&(), &array)
                    },
                    Some(v) if v.is_boolean() => {
                        let array : Vec<bool> = a.iter().map(|x| x.as_bool().unwrap()).collect();
                        executor.resolve_with_ctx(&(), &array)
                    },
                    Some(v) if v.is_f64() => {
                        let array : Vec<f64> = a.iter().map(|x| x.as_f64().unwrap()).collect();
                        executor.resolve_with_ctx(&(), &array)
                    }
                    Some(v) if v.is_i64() => {
                        let array : Vec<i32> = a.iter().map(|x| x.as_i64().unwrap() as i32).collect();
                        executor.resolve_with_ctx(&(), &array)
                    },
                    Some(_v) => {
                        Err(Error::new(
                            ErrorKind::InvalidPropertyType(
                                String::from(field_name) + " is a non-scalar array. Expected a scalar or a scalar array.",
                            ),
                            None,
                        )
                        .into())
                    }
                }
            },
            Value::Object(_) => Err(Error::new(
                ErrorKind::InvalidPropertyType(
                    String::from(field_name) + " is an object. Expected a scalar or a scalar array.",
                ),
                None,
            )
            .into()),
        },
    )
}

pub fn resolve_static_version_query<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    match &executor.context().version {
        Some(v) => Ok(juniper::Value::scalar(v.clone())),
        None => Ok(juniper::Value::Null),
    }
}

pub fn resolve_union_field<GlobalCtx: Debug, ReqCtx: Debug + RequestContext>(
    info: &Info,
    field_name: &str,
    src: &Node<GlobalCtx, ReqCtx>,
    dst: &Node<GlobalCtx, ReqCtx>,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult {
    trace!(
        "resolve_union_field called -- info.name: {}, field_name: {}, src: {}, dst: {}",
        info.name,
        field_name,
        src.concrete_typename,
        dst.concrete_typename
    );

    match field_name {
        "dst" => executor.resolve(
            &Info::new(dst.concrete_typename.to_owned(), info.type_defs.clone()),
            dst,
        ),
        "src" => executor.resolve(
            &Info::new(src.concrete_typename.to_owned(), info.type_defs.clone()),
            src,
        ),
        _ => Err(Error::new(
            ErrorKind::InvalidProperty(String::from(&info.name) + "::" + field_name),
            None,
        )
        .into()),
    }
}
