//! Contains the input, node, and relationship data structures used for Warpgrapher's
//! auto-generated CRUD query endpoints. Optionally, these structured are available for use by
//! custom resolver code, as well.

pub mod resolvers;

use super::context::GraphQLContext;
use super::schema::{ArgumentKind, Info, NodeType, Property, PropertyKind, TypeKind};
use crate::engine::context::{GlobalContext, RequestContext};
#[cfg(feature = "cosmos")]
use crate::engine::database::cosmos::CosmosTransaction;
#[cfg(feature = "neo4j")]
use crate::engine::database::neo4j::Neo4jTransaction;
use crate::engine::database::DatabasePool;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use crate::engine::database::Transaction;
use crate::engine::resolvers::Object;
use crate::engine::value::Value;
use crate::error::Error;
use juniper::meta::MetaType;
pub use juniper::GraphQLType;
use juniper::{
    Arguments, DefaultScalarValue, ExecutionResult, Executor, FromInputValue, InputValue, Registry,
    Selection, ID,
};
use log::{error, trace};
use resolvers::Resolver;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
struct Input<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    value: Value,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<GlobalCtx, RequestCtx> Input<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn new(value: Value) -> Input<GlobalCtx, RequestCtx> {
        Input {
            value,
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, RequestCtx> FromInputValue for Input<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn from_input_value(v: &InputValue) -> Option<Self> {
        serde_json::to_value(v)
            .ok()
            .and_then(|val| val.try_into().ok())
            .map(Input::new)
    }
}

impl<GlobalCtx, RequestCtx> GraphQLType for Input<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<GlobalCtx, RequestCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        trace!("Input::name called for {}", info.name());
        Some(&info.name())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Input::meta called for {}", info.name());

        let nt = info.type_def_by_name(info.name()).unwrap_or_else(|e| {
            // this path is only reached if there is a bug in the code
            error!(
                "Input::meta expected type '{}' that was not found in GraphQL schema",
                info.name().to_string()
            );
            panic!(e)
        });

        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|p| p.name());

        let args = props
            .iter()
            .map(|p| match (p.type_name(), p.required(), p.list()) {
                ("Boolean", false, false) => registry.arg::<Option<bool>>(p.name(), &()),
                ("Boolean", false, true) => registry.arg::<Option<Vec<bool>>>(p.name(), &()),
                ("Boolean", true, false) => registry.arg::<bool>(p.name(), &()),
                ("Boolean", true, true) => registry.arg::<Vec<bool>>(p.name(), &()),
                ("Float", false, false) => registry.arg::<Option<f64>>(p.name(), &()),
                ("Float", false, true) => registry.arg::<Option<Vec<f64>>>(p.name(), &()),
                ("Float", true, false) => registry.arg::<f64>(p.name(), &()),
                ("Float", true, true) => registry.arg::<Vec<f64>>(p.name(), &()),
                ("ID", false, false) => registry.arg::<Option<ID>>(p.name(), &()),
                ("ID", false, true) => registry.arg::<Option<Vec<ID>>>(p.name(), &()),
                ("ID", true, false) => registry.arg::<ID>(p.name(), &()),
                ("ID", true, true) => registry.arg::<Vec<ID>>(p.name(), &()),
                ("Int", false, false) => registry.arg::<Option<i32>>(p.name(), &()),
                ("Int", false, true) => registry.arg::<Option<Vec<i32>>>(p.name(), &()),
                ("Int", true, false) => registry.arg::<i32>(p.name(), &()),
                ("Int", true, true) => registry.arg::<Vec<i32>>(p.name(), &()),
                ("String", false, false) => registry.arg::<Option<String>>(p.name(), &()),
                ("String", false, true) => registry.arg::<Option<Vec<String>>>(p.name(), &()),
                ("String", true, false) => registry.arg::<String>(p.name(), &()),
                ("String", true, true) => registry.arg::<Vec<String>>(p.name(), &()),
                (_, false, false) => registry.arg::<Option<Input<GlobalCtx, RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, false, true) => registry.arg::<Option<Vec<Input<GlobalCtx, RequestCtx>>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, false) => registry.arg::<Input<GlobalCtx, RequestCtx>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, true) => registry.arg::<Vec<Input<GlobalCtx, RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_input_object_type::<Input<GlobalCtx, RequestCtx>>(info, &args)
            .into_meta()
    }
}

/// Represents a node in the graph data structure for auto-generated CRUD operations and custom
/// resolvers.
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
#[derive(Clone, Debug)]
pub struct Node<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: Debug + RequestContext,
{
    concrete_typename: String,
    fields: HashMap<String, Value>,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<GlobalCtx, RequestCtx> Node<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        concrete_typename: String,
        fields: HashMap<String, Value>,
    ) -> Node<GlobalCtx, RequestCtx> {
        Node {
            concrete_typename,
            fields,
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }

    fn union_meta<'r>(nt: &NodeType, info: &Info, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!(
            "Node::union_meta called - nt.type_name(): {}",
            nt.type_name()
        );
        let types = match nt.union_types() {
            None => panic!("Missing union_types on NodeType of type Union"),
            Some(union_types) => union_types
                .clone()
                .map(|ut| {
                    registry.get_type::<Node<GlobalCtx, RequestCtx>>(&Info::new(
                        ut.to_string(),
                        info.type_defs(),
                    ))
                })
                .collect::<Vec<_>>(),
        };
        registry
            .build_union_type::<Node<GlobalCtx, RequestCtx>>(info, &types)
            .into_meta()
    }

    fn object_meta<'r>(nt: &NodeType, info: &Info, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Node::object_meta -- nt.type_name(): {}", nt.type_name());
        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| p.name());

        let fields = props
            .iter()
            .map(|p| {
                let f = match (p.type_name(), p.required(), p.list(), p.kind()) {
                    ("Boolean", false, false, _) => registry.field::<Option<bool>>(p.name(), &()),
                    ("Boolean", false, true, _) => {
                        registry.field::<Option<Vec<bool>>>(p.name(), &())
                    }
                    ("Boolean", true, false, _) => registry.field::<bool>(p.name(), &()),
                    ("Boolean", true, true, _) => registry.field::<Vec<bool>>(p.name(), &()),
                    ("Float", false, false, _) => registry.field::<Option<f64>>(p.name(), &()),
                    ("Float", false, true, _) => registry.field::<Option<Vec<f64>>>(p.name(), &()),
                    ("Float", true, false, _) => registry.field::<f64>(p.name(), &()),
                    ("Float", true, true, _) => registry.field::<Vec<f64>>(p.name(), &()),
                    ("ID", false, false, _) => registry.field::<Option<ID>>(p.name(), &()),
                    ("ID", false, true, _) => registry.field::<Option<Vec<ID>>>(p.name(), &()),
                    ("ID", true, false, _) => registry.field::<ID>(p.name(), &()),
                    ("ID", true, true, _) => registry.field::<Vec<ID>>(p.name(), &()),
                    ("Int", false, false, _) => registry.field::<Option<i32>>(p.name(), &()),
                    ("Int", false, true, _) => registry.field::<Option<Vec<i32>>>(p.name(), &()),
                    ("Int", true, false, _) => registry.field::<i32>(p.name(), &()),
                    ("Int", true, true, _) => registry.field::<Vec<i32>>(p.name(), &()),
                    ("String", false, false, _) => registry.field::<Option<String>>(p.name(), &()),
                    ("String", false, true, _) => {
                        registry.field::<Option<Vec<String>>>(p.name(), &())
                    }
                    ("String", true, false, _) => registry.field::<String>(p.name(), &()),
                    ("String", true, true, _) => registry.field::<Vec<String>>(p.name(), &()),
                    (_, false, false, PropertyKind::Rel { rel_name: _ }) => registry
                        .field::<Option<Rel<GlobalCtx, RequestCtx>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        ),
                    (_, false, false, _) => registry.field::<Option<Node<GlobalCtx, RequestCtx>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, false, true, PropertyKind::Rel { rel_name: _ }) => {
                        registry.field::<Option<Vec<&Rel<GlobalCtx, RequestCtx>>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        )
                    }
                    (_, false, true, _) => registry
                        .field::<Option<Vec<&Node<GlobalCtx, RequestCtx>>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        ),
                    (_, true, false, PropertyKind::Rel { rel_name: _ }) => {
                        registry.field::<Rel<GlobalCtx, RequestCtx>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        )
                    }
                    (_, true, false, _) => registry.field::<Node<GlobalCtx, RequestCtx>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, true, true, PropertyKind::Rel { rel_name: _ }) => {
                        registry.field::<Vec<&Rel<GlobalCtx, RequestCtx>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        )
                    }
                    (_, true, true, _) => registry.field::<Vec<&Node<GlobalCtx, RequestCtx>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                };

                p.arguments().fold(f, |f, arg| {
                    match (arg.name(), arg.type_name(), arg.kind()) {
                        (name, "Boolean", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<bool>>(name, &()))
                        }
                        (name, "Boolean", ArgumentKind::Required) => {
                            f.argument(registry.arg::<bool>(name, &()))
                        }
                        (name, "Float", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<f64>>(name, &()))
                        }
                        (name, "Float", ArgumentKind::Required) => {
                            f.argument(registry.arg::<f64>(name, &()))
                        }
                        (name, "ID", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<ID>>(name, &()))
                        }
                        (name, "ID", ArgumentKind::Required) => {
                            f.argument(registry.arg::<ID>(name, &()))
                        }
                        (name, "Int", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<i32>>(name, &()))
                        }
                        (name, "Int", ArgumentKind::Required) => {
                            f.argument(registry.arg::<i32>(name, &()))
                        }
                        ("partitionKey", "String", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<String>>("partitionKey", &()))
                        }
                        (name, "String", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<String>>(name, &()))
                        }
                        (name, "String", ArgumentKind::Required) => {
                            f.argument(registry.arg::<String>(name, &()))
                        }
                        ("input", type_name, ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<Input<GlobalCtx, RequestCtx>>>(
                                "input",
                                &Info::new(type_name.to_string(), info.type_defs()),
                            ))
                        }
                        ("input", type_name, ArgumentKind::Required) => {
                            f.argument(registry.arg::<Input<GlobalCtx, RequestCtx>>(
                                "input",
                                &Info::new(type_name.to_string(), info.type_defs()),
                            ))
                        }
                        (_, _, _) => panic!(Error::TypeNotExpected),
                    }
                })
            })
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Node<GlobalCtx, RequestCtx>>(info, &fields)
            .into_meta()
    }

    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    fn resolve_field_with_transaction<T>(
        &self,
        info: &<Self as GraphQLType>::TypeInfo,
        field_name: &str,
        args: &Arguments,
        executor: &Executor<<Self as GraphQLType>::Context>,
        transaction: &mut T,
    ) -> ExecutionResult
    where
        T: Transaction,
    {
        let sn = Self::name(info).ok_or_else(|| Error::SchemaItemNotFound {
            name: info.name().to_string(),
        })?;
        trace!(
            "Node::resolve_field_with_transaction called -- sn: {}, field_name: {}",
            sn,
            field_name,
        );

        let p = info.type_def()?.property(field_name)?;
        let input_opt: Option<Input<GlobalCtx, RequestCtx>> = args.get("input");

        // The partition key is only in the arguments for the outermost query or mutation.
        // For lower-level field resolution, the partition key is read from the field of the parent.
        // An alternate design would've been to carry the partitionKey in context, but this way
        // recursive resovle calls from custom resolvers that execute cross-partition queries will
        // work correctly, as each node carries its own partition, for any recursion to fill out the
        // other rels and nodes loaded by the shape.
        let arg_partition_key = args.get("partitionKey");
        let partition_key_opt: Option<&Value> = arg_partition_key
            .as_ref()
            .or_else(|| self.fields.get("partitionKey"));

        let mut resolver = Resolver::new(partition_key_opt, executor, transaction);

        match p.kind() {
            PropertyKind::CustomResolver => resolver.resolve_custom_endpoint(
                info,
                field_name,
                Object::Node(self),
                args,
            ),
            PropertyKind::DynamicScalar => resolver.resolve_custom_field(
                info,
                field_name,
                &p.resolver(),
                Object::Node(self),
                args,
            ),
            PropertyKind::DynamicRel { rel_name } => resolver.resolve_custom_rel(
                info,
                rel_name,
                &p.resolver(),
                Object::Node(self),
                args,
            ),
            PropertyKind::Input => Err(Error::TypeNotExpected.into()),
            PropertyKind::NodeCreateMutation => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_create_mutation(
                    field_name,
                    info,
                    input,
                )
            }
            PropertyKind::NodeDeleteMutation { label } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_delete_mutation(
                    field_name,
                    &label,
                    info,
                    input,
                )
            }
            PropertyKind::NodeUpdateMutation => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_update_mutation(
                    field_name,
                    info,
                    input,
                )
            }
            PropertyKind::Object => resolver.resolve_node_read_query(
                field_name,
                info,
                input_opt,
            ),
            PropertyKind::Rel { rel_name } => resolver.resolve_rel_field(
                field_name,
                self.fields.get("id").cloned(),
                rel_name,
                info,
                input_opt,
            ),
            PropertyKind::RelCreateMutation {
                src_label,
                rel_name,
            } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_rel_create_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                )
            }
            PropertyKind::RelDeleteMutation {
                src_label,
                rel_name,
            } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_rel_delete_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                )
            }
            PropertyKind::RelUpdateMutation {
                src_label,
                rel_name,
            } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_rel_update_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    input,
                )
            }
            PropertyKind::Scalar => resolver.resolve_scalar_field(
                info,
                field_name,
                &self.fields,
            ),
            PropertyKind::Union => Err(Error::TypeNotExpected.into()),
            PropertyKind::VersionQuery => resolver.resolve_static_version_query(info, args),
        }
    }

    pub(crate) fn typename(&self) -> &str {
        &self.concrete_typename
    }
}

impl<GlobalCtx, RequestCtx> GraphQLType for Node<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<GlobalCtx, RequestCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name())
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_def_by_name(&info.name())
            .unwrap_or_else(|e| {
                error!(
                    "Node::concrete_type_name panicking on type: {}",
                    info.name()
                );
                panic!(e)
            })
            .type_name()
            .to_string();
        trace!(
            "Node::concrete_type_name -- info.name: {:#?}, returning {:#?}",
            info.name(),
            tn
        );

        tn
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Node::meta called -- info.name: {}", info.name());
        let nt = info.type_def_by_name(&info.name()).unwrap_or_else(|e| {
            error!("Node::meta panicking on type: {}", info.name().to_string());
            panic!(e)
        });

        match nt.type_kind() {
            TypeKind::Union => Node::<GlobalCtx, RequestCtx>::union_meta(nt, info, registry),
            _ => Node::<GlobalCtx, RequestCtx>::object_meta(nt, info, registry),
        }
    }

    #[allow(unused_variables)]
    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        let sn = Self::name(info).ok_or_else(|| Error::SchemaItemNotFound {
            name: info.name().to_string(),
        })?;
        trace!(
            "Node::resolve_field called -- sn: {}, field_name: {}",
            sn,
            field_name
        );

        match &executor.context().pool() {
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let graph = p.get()?;
                let mut transaction = Neo4jTransaction::new(graph.transaction().begin()?.0);
                self.resolve_field_with_transaction(
                    info,
                    field_name,
                    args,
                    executor,
                    &mut transaction,
                )
            }
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                let mut transaction = CosmosTransaction::new(c.clone());
                self.resolve_field_with_transaction(
                    info,
                    field_name,
                    args,
                    executor,
                    &mut transaction,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound.into()),
        }
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        type_name: &str,
        _selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        let sn = Self::name(info).ok_or_else(|| Error::SchemaItemNotFound {
            name: info.name().to_string(),
        })?;

        trace!(
            "Node::resolve_into_type called -- sn: {}, info.name: {}, type_name: {}, self.concrete_typename: {}",
            sn,
            info.name(),
            type_name,
            self.concrete_typename
        );

        // this mismatch can occur when query fragments are used. correct behavior is to not
        // resolve it
        if info.name() != type_name {
            trace!(
                "info.name() {} != type_name {}, returning NULL",
                info.name(),
                type_name
            );
            return Ok(juniper::Value::Null);
        }

        executor.resolve(
            &Info::new(self.concrete_typename.to_owned(), info.type_defs()),
            &Some(self),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Rel<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    id: Value,
    partition_key: Option<Value>,
    props: Option<Node<GlobalCtx, RequestCtx>>,
    src: Node<GlobalCtx, RequestCtx>,
    dst: Node<GlobalCtx, RequestCtx>,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<GlobalCtx: GlobalContext, RequestCtx> Rel<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        id: Value,
        partition_key: Option<Value>,
        props: Option<Node<GlobalCtx, RequestCtx>>,
        src: Node<GlobalCtx, RequestCtx>,
        dst: Node<GlobalCtx, RequestCtx>,
    ) -> Rel<GlobalCtx, RequestCtx> {
        Rel {
            id,
            partition_key,
            props,
            src,
            dst,
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }

    fn resolve_field_with_transaction<T: Transaction>(
        &self, info: &<Self as GraphQLType>::TypeInfo, field_name: &str, args: &Arguments, executor: &Executor<<Self as GraphQLType>::Context>, transaction: &mut T
    ) -> ExecutionResult {
        let p = info.type_def()?.property(field_name)?;

        let arg_partition_key = args.get("partitionKey");
        let partition_key_opt: Option<&Value> = arg_partition_key
            .as_ref()
            .or_else(|| self.partition_key.as_ref());

        let mut resolver = Resolver::new(partition_key_opt, executor, transaction);

        match (p.kind(), &field_name) {
            (PropertyKind::DynamicScalar, _) => resolver.resolve_custom_field(
                info,
                field_name,
                p.resolver(),
                Object::Rel(self),
                args,
            ),
            (PropertyKind::Object, &"props") => match &self.props {
                Some(p) => resolver.resolve_rel_props(info, field_name, p),
                None => Err(Error::TypeNotExpected.into()),
            },
            (PropertyKind::Object, &"src") => executor.resolve(
                &Info::new(self.src.concrete_typename.to_owned(), info.type_defs()),
                &self.src,
            ),
            (PropertyKind::Object, _) => Err(Error::ResponseItemNotFound {
                name: field_name.to_string(),
            }
            .into()),
            (PropertyKind::Scalar, _) => {
                if field_name == "id" {
                    executor.resolve_with_ctx(&(), &TryInto::<String>::try_into(self.id.clone())?)
                } else {
                    executor.resolve_with_ctx(&(), &None::<String>)
                }
            }
            (PropertyKind::Union, _) => {
                resolver.resolve_union_field(info, field_name, &self.src, &self.dst)
            }
            (_, _) => Err(Error::TypeNotExpected.into()),
        }
    }
}

impl<GlobalCtx, RequestCtx> GraphQLType for Rel<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<GlobalCtx, RequestCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name())
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_def_by_name(&info.name())
            .unwrap_or_else(|e| {
                error!("Rel::concrete_type_name panicking on type: {}", info.name());
                panic!(e)
            })
            .type_name()
            .to_owned();

        trace!(
            "Rel::concrete_type_name called -- info.name: {}, returning {}",
            info.name(),
            tn
        );

        tn
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Rel::meta called for {}", info.name());

        let nt = info.type_def_by_name(&info.name()).unwrap_or_else(|e| {
            error!("Rel::meta panicking on type: {}", info.name().to_string());
            panic!(e)
        });

        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| p.name());

        let fields = props
            .iter()
            .map(|p| match (p.type_name(), p.required(), p.list()) {
                ("ID", false, false) => registry.field::<Option<ID>>(p.name(), &()),
                ("ID", false, true) => registry.field::<Option<Vec<ID>>>(p.name(), &()),
                ("ID", true, false) => registry.field::<ID>(p.name(), &()),
                ("ID", true, true) => registry.field::<Vec<ID>>(p.name(), &()),
                (_, false, false) => registry.field::<Option<Node<GlobalCtx, RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, false, true) => registry.field::<Option<Vec<&Node<GlobalCtx, RequestCtx>>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, false) => registry.field::<Node<GlobalCtx, RequestCtx>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, true) => registry.field::<Vec<&Node<GlobalCtx, RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Rel<GlobalCtx, RequestCtx>>(info, &fields)
            .into_meta()
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        let sn = Self::name(info).ok_or_else(|| Error::SchemaItemNotFound {
            name: info.name().to_string(),
        })?;
        trace!(
            "Rel::resolve_field called -- sn: {}, field_name: {}",
            sn,
            field_name
        );

        match &executor.context().pool() {
            #[cfg(feature = "neo4j")]
            DatabasePool::Neo4j(p) => {
                let graph = p.get()?;
                let mut transaction = Neo4jTransaction::new(graph.transaction().begin()?.0);
                self.resolve_field_with_transaction(
                    info,
                    field_name,
                    args,
                    executor,
                    &mut transaction,
                )
            }
            #[cfg(feature = "cosmos")]
            DatabasePool::Cosmos(c) => {
                let mut transaction = CosmosTransaction::new(c.clone());
                self.resolve_field_with_transaction(
                    info,
                    field_name,
                    args,
                    executor,
                    &mut transaction,
                )
            }
            DatabasePool::NoDatabase => Err(Error::DatabaseNotFound.into()),
        }
    }
}
