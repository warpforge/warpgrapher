//! Contains the input, node, and relationship data structures used for Warpgrapher's
//! auto-generated CRUD query endpoints. Optionally, these structured are available for use by
//! custom resolver code, as well.

use super::context::GraphQLContext;
use super::schema::{ArgumentKind, Info, NodeType, Property, PropertyKind, TypeKind};
use crate::engine::context::RequestContext;
use crate::engine::resolvers::Object;
use crate::engine::value::Value;
use crate::error::Error;
use juniper::meta::{EnumValue, MetaType};
use juniper::{
    Arguments, BoxFuture, DefaultScalarValue, ExecutionResult, Executor, FromInputValue,
    InputValue, Registry, Selection, ID,
};
pub use juniper::{GraphQLType, GraphQLTypeAsync, GraphQLValue, GraphQLValueAsync};
use log::{error, trace};
use resolvers::Resolver;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::marker::PhantomData;

pub(crate) mod resolvers;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sort {
    direction: Direction,
    dst_property: bool,
    property: String,
}

impl Sort {
    pub fn new(direction_opt: Option<String>, order_by: String) -> Sort {
        let property_string = order_by.to_string();
        let property_path = property_string.split(':').collect::<Vec<&str>>();

        Sort {
            direction: if Some("descending".to_string()) == direction_opt {
                Direction::Descending
            } else {
                Direction::Ascending
            },
            dst_property: property_path.len() > 1,
            property: if let Some(s) = property_path.last() {
                s.to_string()
            } else {
                order_by
            },
        }
    }

    pub fn direction(&self) -> &Direction {
        &self.direction
    }

    pub fn dst_property(&self) -> bool {
        self.dst_property
    }

    pub fn property(&self) -> &str {
        &self.property
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Options {
    sort: Vec<Sort>,
}

impl Options {
    pub fn new(sort: Vec<Sort>) -> Options {
        Options { sort }
    }

    pub fn sort(&self) -> &[Sort] {
        &self.sort
    }
}

#[derive(Clone, Debug)]
struct Enumeration<RequestCtx>
where
    RequestCtx: RequestContext,
{
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> Enumeration<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn new() -> Enumeration<RequestCtx> {
        Enumeration { _rctx: PhantomData }
    }
}

impl<RequestCtx> FromInputValue for Enumeration<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn from_input_value(_v: &InputValue) -> Option<Self> {
        /*
        serde_json::to_value(v)
            .ok()
            .and_then(|val| val.try_into().ok())
            .map(Enumeration::new)
            */
        Some(Enumeration::new())
    }
}

impl<RequestCtx> GraphQLType for Enumeration<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Enumeration::meta called for {}", info.name());

        let nt = info.type_def_by_name(info.name()).unwrap_or_else(|e| {
            // this path is only reached if there is a bug in the code
            error!(
                "Input::meta expected type '{}' that was not found in GraphQL schema",
                info.name().to_string()
            );
            panic!("{}", e)
        });

        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|p| p.name());

        let variants: Vec<EnumValue> = props.iter().map(|p| EnumValue::new(p.name())).collect();

        registry
            .build_enum_type::<Enumeration<RequestCtx>>(info, &variants)
            .into_meta()
    }
}

impl<RequestCtx> GraphQLValue for Enumeration<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
    type TypeInfo = Info;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name())
    }
}

impl<RequestCtx> GraphQLValueAsync for Enumeration<RequestCtx> where RequestCtx: RequestContext {}

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
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name())
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
            panic!("{}", e)
        });

        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|p| p.name());

        let args = props
            .iter()
            .filter(|p| !p.hidden())
            .map(
                |p| match (p.kind(), p.type_name(), p.required(), p.list()) {
                    (_, "Boolean", false, false) => registry.arg::<Option<bool>>(p.name(), &()),
                    (_, "Boolean", false, true) => registry.arg::<Option<Vec<bool>>>(p.name(), &()),
                    (_, "Boolean", true, false) => registry.arg::<bool>(p.name(), &()),
                    (_, "Boolean", true, true) => registry.arg::<Vec<bool>>(p.name(), &()),
                    (_, "Float", false, false) => registry.arg::<Option<f64>>(p.name(), &()),
                    (_, "Float", false, true) => registry.arg::<Option<Vec<f64>>>(p.name(), &()),
                    (_, "Float", true, false) => registry.arg::<f64>(p.name(), &()),
                    (_, "Float", true, true) => registry.arg::<Vec<f64>>(p.name(), &()),
                    (_, "ID", false, false) => registry.arg::<Option<ID>>(p.name(), &()),
                    (_, "ID", false, true) => registry.arg::<Option<Vec<ID>>>(p.name(), &()),
                    (_, "ID", true, false) => registry.arg::<ID>(p.name(), &()),
                    (_, "ID", true, true) => registry.arg::<Vec<ID>>(p.name(), &()),
                    (_, "Int", false, false) => registry.arg::<Option<i32>>(p.name(), &()),
                    (_, "Int", false, true) => registry.arg::<Option<Vec<i32>>>(p.name(), &()),
                    (_, "Int", true, false) => registry.arg::<i32>(p.name(), &()),
                    (_, "Int", true, true) => registry.arg::<Vec<i32>>(p.name(), &()),
                    (_, "String", false, false) => registry.arg::<Option<String>>(p.name(), &()),
                    (_, "String", false, true) => {
                        registry.arg::<Option<Vec<String>>>(p.name(), &())
                    }
                    (_, "String", true, false) => registry.arg::<String>(p.name(), &()),
                    (_, "String", true, true) => registry.arg::<Vec<String>>(p.name(), &()),
                    (PropertyKind::Enum, _, true, _) => registry.arg::<Enumeration<RequestCtx>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (PropertyKind::Enum, _, false, _) => registry
                        .arg::<Option<Enumeration<RequestCtx>>>(
                            p.name(),
                            &Info::new(p.type_name().to_string(), info.type_defs()),
                        ),
                    (_, _, false, false) => registry.arg::<Option<Input<RequestCtx>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, _, false, true) => registry.arg::<Option<Vec<Input<RequestCtx>>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, _, true, false) => registry.arg::<Input<RequestCtx>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                    (_, _, true, true) => registry.arg::<Vec<Input<RequestCtx>>>(
                        p.name(),
                        &Info::new(p.type_name().to_string(), info.type_defs()),
                    ),
                },
            )
            .collect::<Vec<_>>();

        registry
            .build_input_object_type::<Input<RequestCtx>>(info, &args)
            .into_meta()
    }
}

impl<RequestCtx> GraphQLValue for Input<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
    type TypeInfo = Info;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name())
    }
}

impl<RequestCtx> GraphQLValueAsync for Input<RequestCtx> where RequestCtx: RequestContext {}

/// Represents a node in the graph data structure for auto-generated CRUD operations and custom
/// resolvers.
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
///         let n = facade.node(typename, props);
///
///         facade.resolve_node(&n).await
///     })
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
    pub fn new(concrete_typename: String, fields: HashMap<String, Value>) -> Node<RequestCtx> {
        Node {
            concrete_typename,
            fields,
            _rctx: PhantomData,
        }
    }

    /// Attempts to deserialize a `Node` into a struct.
    ///
    /// # Example
    /// ```rust
    /// # use serde::Deserialize;
    /// # use warpgrapher::engine::objects::Node;
    ///
    /// #[derive(Deserialize)]
    /// struct Team {
    ///     name: String
    /// }
    ///
    /// fn handle_node(n: Node<()>) {
    ///
    ///     // succeeds if the fields of `n` can be deserialized into `Team`
    ///     let team: Team = n.deser().unwrap();
    ///     
    /// }
    /// ```
    pub fn deser<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        let mut fields = self.fields().clone();
        fields.insert(
            "__label".to_string(),
            Value::String(self.concrete_typename.clone()),
        );
        fields.insert("id".to_string(), self.id()?.clone());
        let m = Value::Map(fields);
        let v = serde_json::Value::try_from(m)?;
        let t: T = serde_json::from_value(v)
            .map_err(|e| Error::JsonDeserializationFailed { source: e })?;
        Ok(t)
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

    pub fn id(&self) -> Result<&Value, Error> {
        self.fields
            .get(&"id".to_string())
            .ok_or_else(|| Error::ResponseItemNotFound {
                name: "id".to_string(),
            })
    }

    pub fn type_name(&self) -> &String {
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
            .filter(|p| !p.hidden())
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
                        (name, "String", ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<String>>(name, &()))
                        }
                        (name, "String", ArgumentKind::Required) => {
                            f.argument(registry.arg::<String>(name, &()))
                        }
                        ("options", type_name, ArgumentKind::Optional) => {
                            f.argument(registry.arg::<Option<Input<RequestCtx>>>(
                                "options",
                                &Info::new(type_name.to_string(), info.type_defs()),
                            ))
                        }
                        ("options", type_name, ArgumentKind::Required) => {
                            f.argument(registry.arg::<Input<RequestCtx>>(
                                "options",
                                &Info::new(type_name.to_string(), info.type_defs()),
                            ))
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
                        (_, _, _) => {
                            panic!(
                                "{}",
                                Error::TypeNotExpected {
                                    details: Some("argument is not valid".to_string())
                                }
                            )
                        }
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
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Node::meta called -- info.name: {}", info.name());
        let nt = info.type_def_by_name(info.name()).unwrap_or_else(|e| {
            error!("Node::meta panicking on type: {}", info.name().to_string());
            panic!("{}", e)
        });

        match nt.type_kind() {
            TypeKind::Union => Node::<RequestCtx>::union_meta(nt, info, registry),
            _ => Node::<RequestCtx>::object_meta(nt, info, registry),
        }
    }
}

impl<RequestCtx> GraphQLValue for Node<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
    type TypeInfo = Info;
    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name())
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_def_by_name(info.name())
            .unwrap_or_else(|e| {
                error!(
                    "Node::concrete_type_name panicking on type: {}",
                    info.name()
                );
                panic!("{}", e)
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
}

impl<RequestCtx> GraphQLValueAsync for Node<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        args: &'a Arguments,
        executor: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        Box::pin(async move {
            let sn = Self::name(info).ok_or_else(|| Error::SchemaItemNotFound {
                name: info.name().to_string(),
            })?;
            trace!(
                "Node::resolve_field_async called -- sn: {}, field_name: {}",
                sn,
                field_name,
            );

            let p = info.type_def()?.property(field_name)?;
            let input_opt: Option<Value> = args.get("input").map(|i: Input<RequestCtx>| i.value);

            let options = if let Some(Value::Map(m)) =
                args.get("options").map(|i: Input<RequestCtx>| i.value)
            {
                Options::new(if let Some(Value::Array(a)) = m.get("sort") {
                    a.iter()
                        .map(|sort| {
                            if let Value::Map(sort_map) = sort {
                                Ok(Sort::new(
                                    sort_map.get("direction").map(|d| d.to_string()),
                                    sort_map.get("orderBy").map(|ob| ob.to_string()).ok_or(
                                        Error::InputItemNotFound {
                                            name: "orderBy".to_string(),
                                        },
                                    )?,
                                ))
                            } else {
                                Err(Error::TypeNotExpected {
                                    details: Some("Expected sort to be a Value::Map".to_string()),
                                })
                            }
                        })
                        .collect::<Result<Vec<Sort>, Error>>()?
                } else {
                    Vec::new()
                })
            } else {
                Options::default()
            };
            trace!("Node::resolve_field_async -- options: {:#?}", options);

            let mut resolver = Resolver::new();

            let result = match p.kind() {
                PropertyKind::CustomResolver => {
                    resolver
                        .resolve_custom_endpoint(
                            info,
                            field_name,
                            Object::Node(self),
                            args,
                            executor,
                        )
                        .await
                }
                PropertyKind::DynamicScalar => {
                    resolver
                        .resolve_custom_field(
                            info,
                            field_name,
                            p.resolver(),
                            Object::Node(self),
                            args,
                            executor,
                        )
                        .await
                }
                PropertyKind::DynamicRel { rel_name } => {
                    resolver
                        .resolve_custom_rel(
                            info,
                            rel_name,
                            p.resolver(),
                            Object::Node(self),
                            args,
                            executor,
                        )
                        .await
                }
                PropertyKind::Enum => Err((Error::TypeNotExpected {
                    details: Some("PropertyKind::Enum not expected.".to_string()),
                })
                .into()),
                PropertyKind::Input => Err((Error::TypeNotExpected {
                    details: Some("PropertyKind::Input not expected".to_string()),
                })
                .into()),
                PropertyKind::NodeCreateMutation => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_node_create_mutation(field_name, info, input, options, executor)
                        .await
                }
                PropertyKind::NodeDeleteMutation { label } => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_node_delete_mutation(
                            field_name, label, info, input, options, executor,
                        )
                        .await
                }
                PropertyKind::NodeUpdateMutation => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_node_update_mutation(field_name, info, input, options, executor)
                        .await
                }
                PropertyKind::Object => {
                    resolver
                        .resolve_node_read_query(field_name, info, input_opt, options, executor)
                        .await
                }
                PropertyKind::Rel { rel_name } => {
                    if sn == "Mutation" || sn == "Query" {
                        // if the sn is Mutation or Query, then this is a root query as opposed to a
                        // relationship reference

                        resolver
                            .resolve_rel_read_query(
                                field_name, rel_name, info, input_opt, options, executor,
                            )
                            .await
                    } else {
                        // If it's not a root query, then it's a relationship reference. Merge the
                        // src node id into the search query input, if the client has added
                        // additional searching / filtering criteria to a query input in the shape,
                        // because we allow filtering on relationships at every nested relationship
                        // in the shape.
                        let mut hm = if let Some(Value::Map(input_map)) = input_opt {
                            input_map
                        } else {
                            HashMap::new()
                        };
                        let mut src = if let Some(Value::Map(src_map)) = hm.remove("src") {
                            src_map
                        } else {
                            HashMap::new()
                        };
                        let mut src_node = if let Some(Value::Map(src_node_map)) =
                            src.remove(info.type_def()?.type_name())
                        {
                            src_node_map
                        } else {
                            HashMap::new()
                        };
                        let mut comparison = HashMap::new();
                        comparison.insert("EQ".to_string(), self.id()?.clone());
                        src_node.insert("id".to_string(), Value::Map(comparison));
                        src.insert(
                            info.type_def()?.type_name().to_string(),
                            Value::Map(src_node),
                        );
                        hm.insert("src".to_string(), Value::Map(src));

                        resolver
                            .resolve_rel_read_query(
                                field_name,
                                rel_name,
                                info,
                                Some(Value::Map(hm)),
                                options,
                                executor,
                            )
                            .await
                    }
                }
                PropertyKind::RelCreateMutation {
                    src_label,
                    rel_name,
                } => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_rel_create_mutation(
                            field_name, src_label, rel_name, info, input, options, executor,
                        )
                        .await
                }
                PropertyKind::RelDeleteMutation {
                    src_label,
                    rel_name,
                } => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_rel_delete_mutation(
                            field_name, src_label, rel_name, info, input, options, executor,
                        )
                        .await
                }
                PropertyKind::RelUpdateMutation {
                    src_label,
                    rel_name,
                } => {
                    let input = input_opt.ok_or_else(|| Error::InputItemNotFound {
                        name: "input".to_string(),
                    })?;
                    resolver
                        .resolve_rel_update_mutation(
                            field_name, src_label, rel_name, info, input, options, executor,
                        )
                        .await
                }
                PropertyKind::Scalar => {
                    resolver
                        .resolve_scalar_field(info, field_name, &self.fields, executor)
                        .await
                }
                PropertyKind::ScalarComp => Err((Error::TypeNotExpected {
                    details: Some("PropertyKind::ScalarComp not expected".to_string()),
                })
                .into()),
                PropertyKind::Union => Err((Error::TypeNotExpected {
                    details: Some("PropertyKind::Union not expected".to_string()),
                })
                .into()),
                PropertyKind::VersionQuery => resolver.resolve_static_version_query(executor).await,
            };

            trace!("Node::resolve_field -- result: {:#?}", result);

            result
        })
    }

    fn resolve_into_type_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        type_name: &str,
        _selection_set: Option<&'a [Selection<'a>]>,
        executor: &'a Executor<'a, 'a, Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        // this mismatch can occur when query fragments are used. correct behavior is to not
        // resolve it
        if info.name() != type_name {
            trace!(
                "info.name() {} != type_name {}, returning NULL",
                info.name(),
                type_name
            );
            return Box::pin(async move { Ok(juniper::Value::Null) });
        }

        Box::pin(async move {
            executor
                .resolve_async(
                    &Info::new(self.concrete_typename.to_owned(), info.type_defs()),
                    &Some(self),
                )
                .await
        })
    }
}

/// Represents a reference to a [`Node`] object as either an [`Identifier`]
/// containing a type and id, or a complete [`Node`] struct.
#[derive(Clone, Debug)]
pub enum NodeRef<RequestCtx: RequestContext> {
    Identifier(Value),
    Node(Node<RequestCtx>),
}

/// Represents a relationship in the graph data structure for auto-generated CRUD operations and
/// custom resolvers.
///
/// # Examples
///
/// ```rust, no_run
/// # use std::collections::HashMap;
/// # use warpgrapher::engine::objects::Options;
/// # use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn custom_resolve(facade: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
///     Box::pin(async move {
///         // do work
///        let node_id = Value::String("12345678-1234-1234-1234-1234567890ab".to_string());
///
///         let mut hm1 = HashMap::new();
///         hm1.insert("role".to_string(), Value::String("member".to_string()));
///
///         // return rel
///         facade.resolve_rel(&facade.create_rel(
///             Value::String("655c4e13-5075-45ea-97de-b43f800e5854".to_string()),
///             "members", hm1, node_id, Options::default())?).await
///     })
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    rel_name: String,
    fields: HashMap<String, Value>,
    src_ref: NodeRef<RequestCtx>,
    dst_ref: NodeRef<RequestCtx>,
    _rctx: PhantomData<RequestCtx>,
}

impl<RequestCtx> Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        rel_name: String,
        fields: HashMap<String, Value>,
        src_ref: NodeRef<RequestCtx>,
        dst_ref: NodeRef<RequestCtx>,
    ) -> Rel<RequestCtx> {
        Rel {
            rel_name,
            fields,
            src_ref,
            dst_ref,
            _rctx: PhantomData,
        }
    }

    pub fn id(&self) -> Result<&Value, Error> {
        self.fields
            .get(&"id".to_string())
            .ok_or_else(|| Error::ResponseItemNotFound {
                name: "id".to_string(),
            })
    }

    pub fn rel_name(&self) -> &String {
        &self.rel_name
    }

    pub fn fields(&self) -> &HashMap<String, Value> {
        &self.fields
    }

    pub fn src_id(&self) -> Result<&Value, Error> {
        match &self.src_ref {
            NodeRef::Identifier(id) => Ok(id),
            NodeRef::Node(n) => n.id(),
        }
    }

    pub fn dst_id(&self) -> Result<&Value, Error> {
        match &self.dst_ref {
            NodeRef::Identifier(id) => Ok(id),
            NodeRef::Node(n) => n.id(),
        }
    }
}

impl<RequestCtx> GraphQLType for Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Rel::meta called for {}", info.name());

        let nt = info.type_def_by_name(info.name()).unwrap_or_else(|e| {
            error!("Rel::meta panicking on type: {}", info.name().to_string());
            panic!("{}", e)
        });

        let mut props = nt.props().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| p.name());

        let fields = props
            .iter()
            .filter(|p| !p.hidden())
            .map(|p| match (p.type_name(), p.required(), p.list()) {
                ("Boolean", false, false) => registry.field::<Option<bool>>(p.name(), &()),
                ("Boolean", false, true) => registry.field::<Option<Vec<bool>>>(p.name(), &()),
                ("Boolean", true, false) => registry.field::<bool>(p.name(), &()),
                ("Boolean", true, true) => registry.field::<Vec<bool>>(p.name(), &()),
                ("Float", false, false) => registry.field::<Option<f64>>(p.name(), &()),
                ("Float", false, true) => registry.field::<Option<Vec<f64>>>(p.name(), &()),
                ("Float", true, false) => registry.field::<f64>(p.name(), &()),
                ("Float", true, true) => registry.field::<Vec<f64>>(p.name(), &()),
                ("ID", false, false) => registry.field::<Option<ID>>(p.name(), &()),
                ("ID", false, true) => registry.field::<Option<Vec<ID>>>(p.name(), &()),
                ("ID", true, false) => registry.field::<ID>(p.name(), &()),
                ("ID", true, true) => registry.field::<Vec<ID>>(p.name(), &()),
                ("Int", false, false) => registry.field::<Option<i32>>(p.name(), &()),
                ("Int", false, true) => registry.field::<Option<Vec<i32>>>(p.name(), &()),
                ("Int", true, false) => registry.field::<i32>(p.name(), &()),
                ("Int", true, true) => registry.field::<Vec<i32>>(p.name(), &()),
                ("String", false, false) => registry.field::<Option<String>>(p.name(), &()),
                ("String", false, true) => registry.field::<Option<Vec<String>>>(p.name(), &()),
                ("String", true, false) => registry.field::<String>(p.name(), &()),
                ("String", true, true) => registry.field::<Vec<String>>(p.name(), &()),
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
}

impl<RequestCtx> GraphQLValue for Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Context = GraphQLContext<RequestCtx>;
    type TypeInfo = Info;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name())
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_def_by_name(info.name())
            .unwrap_or_else(|e| {
                error!("Rel::concrete_type_name panicking on type: {}", info.name());
                panic!("{}", e)
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
}

impl<RequestCtx> GraphQLValueAsync for Rel<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        args: &'a Arguments,
        executor: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        Box::pin(async move {
            trace!(
                "Rel::resolve_field_async called -- field_name: {}",
                field_name
            );
            let p = info.type_def()?.property(field_name)?;

            let options = if let Some(Value::Map(m)) =
                args.get("options").map(|i: Input<RequestCtx>| i.value)
            {
                Options::new(if let Some(Value::Array(a)) = m.get("sort") {
                    a.iter()
                        .map(|sort| {
                            if let Value::Map(sort_map) = sort {
                                Ok(Sort::new(
                                    sort_map.get("direction").map(|d| d.to_string()),
                                    sort_map.get("orderBy").map(|ob| ob.to_string()).ok_or(
                                        Error::InputItemNotFound {
                                            name: "orderBy".to_string(),
                                        },
                                    )?,
                                ))
                            } else {
                                Err(Error::TypeNotExpected {
                                    details: Some("Expected sort to be a Value::Map".to_string()),
                                })
                            }
                        })
                        .collect::<Result<Vec<Sort>, Error>>()?
                } else {
                    Vec::new()
                })
            } else {
                Options::default()
            };
            trace!("Node::resolve_field_async -- options: {:#?}", options);

            let mut resolver = Resolver::new();

            match (p.kind(), &field_name) {
                (PropertyKind::DynamicScalar, _) => {
                    resolver
                        .resolve_custom_field(
                            info,
                            field_name,
                            p.resolver(),
                            Object::Rel(self),
                            args,
                            executor,
                        )
                        .await
                }
                (PropertyKind::Object, &"src") => match &self.src_ref {
                    NodeRef::Identifier(id) => {
                        let mut comparison = HashMap::new();
                        comparison.insert("EQ".to_string(), id.clone());
                        let mut hm = HashMap::new();
                        hm.insert("id".to_string(), Value::Map(comparison));
                        resolver
                            .resolve_node_read_query(
                                field_name,
                                info,
                                Some(Value::Map(hm)),
                                options,
                                executor,
                            )
                            .await
                    }
                    NodeRef::Node(n) => {
                        executor
                            .resolve_async(&Info::new(n.type_name().clone(), info.type_defs()), &n)
                            .await
                    }
                },
                (PropertyKind::Object, _) => Err(Error::ResponseItemNotFound {
                    name: field_name.to_string(),
                }
                .into()),
                (PropertyKind::Scalar, _) => {
                    resolver
                        .resolve_scalar_field(info, field_name, &self.fields, executor)
                        .await
                }
                (PropertyKind::Union, _) => match &self.dst_ref {
                    NodeRef::Identifier(id) => {
                        let mut comparison = HashMap::new();
                        comparison.insert("EQ".to_string(), id.clone());
                        let mut hm = HashMap::new();
                        hm.insert("id".to_string(), Value::Map(comparison));
                        resolver
                            .resolve_node_read_query(
                                field_name,
                                info,
                                Some(Value::Map(hm)),
                                options,
                                executor,
                            )
                            .await
                    }
                    NodeRef::Node(n) => {
                        executor
                            .resolve_async(&Info::new(n.type_name().clone(), info.type_defs()), &n)
                            .await
                    }
                },
                (_, _) => Err((Error::TypeNotExpected {
                    details: Some("Unexpected PropertyKind".to_string()),
                })
                .into()),
            }
        })
    }
}
