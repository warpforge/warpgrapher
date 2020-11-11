//! Contains the input, node, and relationship data structures used for Warpgrapher's
//! auto-generated CRUD query endpoints. Optionally, these structured are available for use by
//! custom resolver code, as well.

use super::context::GraphQLContext;
use super::schema::{ArgumentKind, Info, NodeType, Property, PropertyKind, TypeKind};
use crate::engine::context::RequestContext;
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

mod resolvers;

#[derive(Clone, Debug)]
struct Input<RequestCtx>
where
    RequestCtx: RequestContext,
{
    value: Value,
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> Input<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn new(value: Value) -> Input<RequestCtx> {
        Input {
            value,
            _rctx: PhantomData,
        }
    }
}

impl<RequestCtx> FromInputValue for Input<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn from_input_value(v: &InputValue) -> Option<Self> {
        serde_json::to_value(v)
            .ok()
            .and_then(|val| val.try_into().ok())
            .map(Input::new)
    }
}

impl<RequestCtx> GraphQLType for Input<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
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
                (_, false, false) => registry.arg::<Option<Input<RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, false, true) => registry.arg::<Option<Vec<Input<RequestCtx>>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, false) => registry.arg::<Input<RequestCtx>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, true) => registry.arg::<Vec<Input<RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_input_object_type::<Input<RequestCtx>>(info, &args)
            .into_meta()
    }
}

/// Represents a node in the graph data structure for auto-generated CRUD operations and custom
/// resolvers.
///
/// # Examples
///
/// ```rust, no_run
/// # use std::collections::HashMap;
/// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
/// # use warpgrapher::engine::value::Value;
///
/// fn custom_resolve(facade: ResolverFacade<()>) -> ExecutionResult {
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
pub struct Node<RequestCtx>
where
    RequestCtx: Debug + RequestContext,
{
    concrete_typename: String,
    fields: HashMap<String, Value>,
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> Node<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        concrete_typename: String,
        fields: HashMap<String, Value>,
    ) -> Node<RequestCtx> {
        Node {
            concrete_typename,
            fields,
            _rctx: PhantomData,
        }
    }

    /// Returns the fields of a [`Node`].
    ///
    /// # Example
    /// ```rust
    /// use warpgrapher::engine::objects::Node;
    ///
    /// fn handle_node(n: Node<()>) {
    ///     let properties = n.fields();
    /// }
    /// ```
    pub fn fields(&self) -> &HashMap<String, Value> {
        &self.fields
    }

    pub(crate) fn id(&self) -> Result<&Value, Error> {
        trace!("Node::id called");
        self.fields
            .get(&"id".to_string())
            .ok_or_else(|| Error::ResponseItemNotFound {
                name: "id".to_string(),
            })
    }

    pub(crate) fn type_name(&self) -> &String {
        &self.concrete_typename
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
                    registry
                        .get_type::<Node<RequestCtx>>(&Info::new(ut.to_string(), info.type_defs()))
                })
                .collect::<Vec<_>>(),
        };
        registry
            .build_union_type::<Node<RequestCtx>>(info, &types)
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
                        .field::<Option<Rel<RequestCtx>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        ),
                    (_, false, false, _) => registry.field::<Option<Node<RequestCtx>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, false, true, PropertyKind::Rel { rel_name: _ }) => {
                        registry.field::<Option<Vec<&Rel<RequestCtx>>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        )
                    }
                    (_, false, true, _) => registry.field::<Option<Vec<&Node<RequestCtx>>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, true, false, PropertyKind::Rel { rel_name: _ }) => registry
                        .field::<Rel<RequestCtx>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        ),
                    (_, true, false, _) => registry.field::<Node<RequestCtx>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, true, true, PropertyKind::Rel { rel_name: _ }) => {
                        registry.field::<Vec<&Rel<RequestCtx>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        )
                    }
                    (_, true, true, _) => registry.field::<Vec<&Node<RequestCtx>>>(
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
                            f.argument(registry.arg::<Option<Input<RequestCtx>>>(
                                "input",
                                &Info::new(type_name.to_string(), info.type_defs()),
                            ))
                        }
                        ("input", type_name, ArgumentKind::Required) => {
                            f.argument(registry.arg::<Input<RequestCtx>>(
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
            .build_object_type::<Node<RequestCtx>>(info, &fields)
            .into_meta()
    }

    pub(crate) fn typename(&self) -> &str {
        &self.concrete_typename
    }
}

impl<RequestCtx> GraphQLType for Node<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
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
            TypeKind::Union => Node::<RequestCtx>::union_meta(nt, info, registry),
            _ => Node::<RequestCtx>::object_meta(nt, info, registry),
        }
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
            "Node::resolve_field called -- sn: {}, field_name: {}",
            sn,
            field_name,
        );

        let p = info.type_def()?.property(field_name)?;
        let input_opt: Option<Input<RequestCtx>> = args.get("input");

        // The partition key is only in the arguments for the outermost query or mutation.
        // For lower-level field resolution, the partition key is read from the field of the parent.
        // An alternate design would've been to carry the partitionKey in context, but this way
        // recursive resolve calls from custom resolvers that execute cross-partition queries will
        // work correctly, as each node carries its own partition, for any recursion to fill out the
        // other rels and nodes loaded by the shape.
        let arg_partition_key = args.get("partitionKey");
        let partition_key_opt: Option<&Value> = arg_partition_key
            .as_ref()
            .or_else(|| self.fields.get("partitionKey"));

        let mut resolver = Resolver::new(partition_key_opt);

        let result = match p.kind() {
            PropertyKind::CustomResolver => resolver.resolve_custom_endpoint(
                info,
                field_name,
                Object::Node(self),
                args,
                executor,
            ),
            PropertyKind::DynamicScalar => resolver.resolve_custom_field(
                info,
                field_name,
                p.resolver(),
                Object::Node(self),
                args,
                executor,
            ),
            PropertyKind::DynamicRel { rel_name } => resolver.resolve_custom_rel(
                info,
                &rel_name,
                p.resolver(),
                Object::Node(self),
                args,
                executor,
            ),
            PropertyKind::Input => Err(Error::TypeNotExpected.into()),
            PropertyKind::NodeCreateMutation => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_create_mutation(field_name, info, input, executor)
            }
            PropertyKind::NodeDeleteMutation { label } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_delete_mutation(field_name, &label, info, input, executor)
            }
            PropertyKind::NodeUpdateMutation => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_node_update_mutation(field_name, info, input, executor)
            }
            PropertyKind::Object => {
                resolver.resolve_node_read_query(field_name, info, input_opt, executor)
            }
            PropertyKind::Rel { rel_name } => {
                let io = match sn {
                    "Mutation" | "Query" => input_opt,
                    _ => {
                        let mut src_node = HashMap::new();
                        src_node.insert("id".to_string(), self.id()?.clone());
                        let mut src = HashMap::new();
                        src.insert(
                            info.type_def()?.type_name().to_string(),
                            Value::Map(src_node),
                        );
                        let mut hm = HashMap::new();
                        hm.insert("src".to_string(), Value::Map(src));
                        Some(Input::new(Value::Map(hm)))
                    }
                };
                resolver.resolve_rel_read_query(field_name, &rel_name, info, io, executor)
            }
            PropertyKind::RelCreateMutation {
                src_label,
                rel_name,
            } => {
                let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                    name: "input".to_string(),
                })?;
                resolver.resolve_rel_create_mutation(
                    field_name, &src_label, &rel_name, info, input, executor,
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
                    field_name, &src_label, &rel_name, info, input, executor,
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
                    field_name, &src_label, &rel_name, info, input, executor,
                )
            }
            PropertyKind::Scalar => {
                resolver.resolve_scalar_field(info, field_name, &self.fields, executor)
            }
            PropertyKind::Union => Err(Error::TypeNotExpected.into()),
            PropertyKind::VersionQuery => resolver.resolve_static_version_query(executor),
        };

        trace!("Node::resolve_field -- result: {:#?}", result);

        result
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

/// Represents a reference to a [`Node`] object as either an [`Identifier`]
/// containing a type and id, or a complete [`Node`] struct.
#[derive(Clone, Debug)]
pub(crate) enum NodeRef<RequestCtx: RequestContext> {
    Identifier { id: Value, label: String },
    Node(Node<RequestCtx>),
}

/// Represents a relationship in the graph data structure for auto-generated CRUD operations and
/// custom resolvers.
///
/// # Examples
///
/// ```rust, no_run
/// # use std::collections::HashMap;
/// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
/// # use warpgrapher::engine::value::Value;
///
/// fn custom_resolve(facade: ResolverFacade<()>) -> ExecutionResult {
///     // do work
///     let node_id = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
///
///     let mut hm1 = HashMap::new();
///     hm1.insert("role".to_string(), Value::String("member".to_string()));
///
///     // return rel
///     facade.resolve_rel(&facade.create_rel(
///         Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
///         Some(hm1), node_id, "DstNodeLabel")?)
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    id: Value,
    partition_key: Option<Value>,
    props: Option<Node<RequestCtx>>,
    src_ref: NodeRef<RequestCtx>,
    dst_ref: NodeRef<RequestCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        id: Value,
        partition_key: Option<Value>,
        props: Option<Node<RequestCtx>>,
        src_ref: NodeRef<RequestCtx>,
        dst_ref: NodeRef<RequestCtx>,
    ) -> Rel<RequestCtx> {
        Rel {
            id,
            partition_key,
            props,
            src_ref,
            dst_ref,
            _rctx: PhantomData,
        }
    }

    pub(crate) fn id(&self) -> &Value {
        &self.id
    }
}

impl<RequestCtx> GraphQLType for Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
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
                (_, false, false) => registry.field::<Option<Node<RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, false, true) => registry.field::<Option<Vec<&Node<RequestCtx>>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, false) => registry.field::<Node<RequestCtx>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
                (_, true, true) => registry.field::<Vec<&Node<RequestCtx>>>(
                    p.name(),
                    &Info::new(p.type_name().to_string(), info.type_defs()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Rel<RequestCtx>>(info, &fields)
            .into_meta()
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        trace!(
            "Rel::resolve_field_with_transaction called -- field_name: {}",
            field_name
        );
        let p = info.type_def()?.property(field_name)?;
        let arg_partition_key = args.get("partitionKey");
        let partition_key_opt: Option<&Value> = arg_partition_key
            .as_ref()
            .or_else(|| self.partition_key.as_ref());

        let mut resolver = Resolver::new(partition_key_opt);

        match (p.kind(), &field_name) {
            (PropertyKind::DynamicScalar, _) => resolver.resolve_custom_field(
                info,
                field_name,
                p.resolver(),
                Object::Rel(self),
                args,
                executor,
            ),
            (PropertyKind::Object, &"props") => match &self.props {
                Some(p) => resolver.resolve_rel_props(info, field_name, p, executor),
                None => Err(Error::TypeNotExpected.into()),
            },
            (PropertyKind::Object, &"src") => match &self.src_ref {
                NodeRef::Identifier { id, label: _ } => {
                    let mut hm = HashMap::new();
                    hm.insert("id".to_string(), id.clone());
                    let input = Input::new(Value::Map(hm));
                    resolver.resolve_node_read_query(field_name, info, Some(input), executor)
                }
                NodeRef::Node(n) => {
                    executor.resolve(&Info::new(n.type_name().clone(), info.type_defs()), &n)
                }
            },
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
            (PropertyKind::Union, _) => match &self.dst_ref {
                NodeRef::Identifier { id, label } => {
                    resolver.resolve_union_field(info, label, field_name, &id, executor)
                }
                NodeRef::Node(n) => {
                    resolver.resolve_union_field_node(info, field_name, &n, executor)
                }
            },
            (_, _) => Err(Error::TypeNotExpected.into()),
        }
    }
}
