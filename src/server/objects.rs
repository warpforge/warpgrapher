use super::context::GraphQLContext;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use super::resolvers::{
    resolve_custom_endpoint, resolve_node_create_mutation, resolve_node_delete_mutation,
    resolve_node_update_mutation, resolve_object_field, resolve_rel_create_mutation,
    resolve_rel_delete_mutation, resolve_rel_field, resolve_rel_update_mutation,
    resolve_static_version_query,
};
use super::resolvers::{
    resolve_custom_field, resolve_rel_props, resolve_scalar_field, resolve_union_field,
};
use super::schema::{ArgumentKind, Info, NodeType, Property, PropertyKind, TypeKind};
use crate::error::{Error, ErrorKind};
use crate::server::context::WarpgrapherRequestContext;
#[cfg(feature = "graphson2")]
use crate::server::database::graphson2::Graphson2Transaction;
#[cfg(feature = "neo4j")]
use crate::server::database::neo4j::Neo4jTransaction;
use crate::server::database::DatabasePool;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use crate::server::database::Transaction;
use crate::server::value::Value;
use juniper::meta::MetaType;
use juniper::{
    Arguments, DefaultScalarValue, ExecutionResult, Executor, FromInputValue, GraphQLType,
    InputValue, Registry, Selection, ID,
};
use log::{error, trace};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Input<GlobalCtx, ReqCtx> {
    pub value: Value,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<ReqCtx>,
}

impl<GlobalCtx, ReqCtx> Input<GlobalCtx, ReqCtx> {
    pub fn new(value: Value) -> Input<GlobalCtx, ReqCtx> {
        Input {
            value,
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, ReqCtx> FromInputValue for Input<GlobalCtx, ReqCtx>
where
    ReqCtx: WarpgrapherRequestContext,
{
    fn from_input_value(v: &InputValue) -> Option<Self> {
        serde_json::to_value(v)
            .ok()
            .and_then(|val| val.try_into().ok())
            .map(Input::new)
    }
}

impl<GlobalCtx, ReqCtx> GraphQLType for Input<GlobalCtx, ReqCtx>
where
    ReqCtx: WarpgrapherRequestContext,
{
    type Context = GraphQLContext<GlobalCtx, ReqCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Input::meta called for {}", info.name);

        let nt = info.type_defs.get(&info.name).unwrap_or_else(|| {
            // this path is only reached if there is a bug in the code
            error!(
                "Node::meta expected type '{}' not found in GraphQL schema",
                info.name.to_string()
            );
            panic!(Error::new(
                ErrorKind::MissingSchemaElement(info.name.to_string()),
                None
            ))
        });

        let mut props = nt.props.values().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| &p.name);

        let args = props
            .iter()
            .map(|p| match (p.type_name.as_str(), p.required, p.list) {
                ("Boolean", false, false) => registry.arg::<Option<bool>>(&p.name, &()),
                ("Boolean", false, true) => registry.arg::<Option<Vec<bool>>>(&p.name, &()),
                ("Boolean", true, false) => registry.arg::<bool>(&p.name, &()),
                ("Boolean", true, true) => registry.arg::<Vec<bool>>(&p.name, &()),
                ("Float", false, false) => registry.arg::<Option<f64>>(&p.name, &()),
                ("Float", false, true) => registry.arg::<Option<Vec<f64>>>(&p.name, &()),
                ("Float", true, false) => registry.arg::<f64>(&p.name, &()),
                ("Float", true, true) => registry.arg::<Vec<f64>>(&p.name, &()),
                ("ID", false, false) => registry.arg::<Option<ID>>(&p.name, &()),
                ("ID", false, true) => registry.arg::<Option<Vec<ID>>>(&p.name, &()),
                ("ID", true, false) => registry.arg::<ID>(&p.name, &()),
                ("ID", true, true) => registry.arg::<Vec<ID>>(&p.name, &()),
                ("Int", false, false) => registry.arg::<Option<i32>>(&p.name, &()),
                ("Int", false, true) => registry.arg::<Option<Vec<i32>>>(&p.name, &()),
                ("Int", true, false) => registry.arg::<i32>(&p.name, &()),
                ("Int", true, true) => registry.arg::<Vec<i32>>(&p.name, &()),
                ("String", false, false) => registry.arg::<Option<String>>(&p.name, &()),
                ("String", false, true) => registry.arg::<Option<Vec<String>>>(&p.name, &()),
                ("String", true, false) => registry.arg::<String>(&p.name, &()),
                ("String", true, true) => registry.arg::<Vec<String>>(&p.name, &()),
                (_, false, false) => registry.arg::<Option<Input<GlobalCtx, ReqCtx>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, false, true) => registry.arg::<Option<Vec<Input<GlobalCtx, ReqCtx>>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, true, false) => registry.arg::<Input<GlobalCtx, ReqCtx>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, true, true) => registry.arg::<Vec<Input<GlobalCtx, ReqCtx>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_input_object_type::<Input<GlobalCtx, ReqCtx>>(info, &args)
            .into_meta()
    }
}

#[derive(Debug)]
pub struct Node<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    pub concrete_typename: String,
    fields: HashMap<String, Value>,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<ReqCtx>,
}

impl<GlobalCtx, ReqCtx> Node<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    pub fn new(
        concrete_typename: String,
        fields: HashMap<String, Value>,
    ) -> Node<GlobalCtx, ReqCtx> {
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
        let types = match &nt.union_types {
            None => panic!("Missing union_types on NodeType of type Union"),
            Some(union_types) => union_types
                .iter()
                .map(|ut| {
                    registry.get_type::<Node<GlobalCtx, ReqCtx>>(&Info::new(
                        ut.to_string(),
                        info.type_defs.clone(),
                    ))
                })
                .collect::<Vec<_>>(),
        };
        registry
            .build_union_type::<Node<GlobalCtx, ReqCtx>>(info, &types)
            .into_meta()
    }

    fn object_meta<'r>(nt: &NodeType, info: &Info, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Node::object_meta called for {}.", nt.type_name);
        let mut props = nt.props.values().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| &p.name);

        let fields = props
            .iter()
            .map(|p| {
                let mut f = match (p.type_name.as_str(), p.required, p.list, &p.kind) {
                    ("Boolean", false, false, _) => registry.field::<Option<bool>>(&p.name, &()),
                    ("Boolean", false, true, _) => {
                        registry.field::<Option<Vec<bool>>>(&p.name, &())
                    }
                    ("Boolean", true, false, _) => registry.field::<bool>(&p.name, &()),
                    ("Boolean", true, true, _) => registry.field::<Vec<bool>>(&p.name, &()),
                    ("Float", false, false, _) => registry.field::<Option<f64>>(&p.name, &()),
                    ("Float", false, true, _) => registry.field::<Option<Vec<f64>>>(&p.name, &()),
                    ("Float", true, false, _) => registry.field::<f64>(&p.name, &()),
                    ("Float", true, true, _) => registry.field::<Vec<f64>>(&p.name, &()),
                    ("ID", false, false, _) => registry.field::<Option<ID>>(&p.name, &()),
                    ("ID", false, true, _) => registry.field::<Option<Vec<ID>>>(&p.name, &()),
                    ("ID", true, false, _) => registry.field::<ID>(&p.name, &()),
                    ("ID", true, true, _) => registry.field::<Vec<ID>>(&p.name, &()),
                    ("Int", false, false, _) => registry.field::<Option<i32>>(&p.name, &()),
                    ("Int", false, true, _) => registry.field::<Option<Vec<i32>>>(&p.name, &()),
                    ("Int", true, false, _) => registry.field::<i32>(&p.name, &()),
                    ("Int", true, true, _) => registry.field::<Vec<i32>>(&p.name, &()),
                    ("String", false, false, _) => registry.field::<Option<String>>(&p.name, &()),
                    ("String", false, true, _) => {
                        registry.field::<Option<Vec<String>>>(&p.name, &())
                    }
                    ("String", true, false, _) => registry.field::<String>(&p.name, &()),
                    ("String", true, true, _) => registry.field::<Vec<String>>(&p.name, &()),
                    (_, false, false, PropertyKind::Rel(_)) => {
                        registry.field::<Option<Rel<GlobalCtx, ReqCtx>>>(
                            &p.name,
                            &Info::new(p.type_name.clone(), info.type_defs.clone()),
                        )
                    }
                    (_, false, false, _) => registry.field::<Option<Node<GlobalCtx, ReqCtx>>>(
                        &p.name,
                        &Info::new(p.type_name.clone(), info.type_defs.clone()),
                    ),
                    (_, false, true, PropertyKind::Rel(_)) => {
                        registry.field::<Option<Vec<&Rel<GlobalCtx, ReqCtx>>>>(
                            &p.name,
                            &Info::new(p.type_name.clone(), info.type_defs.clone()),
                        )
                    }
                    (_, false, true, _) => registry.field::<Option<Vec<&Node<GlobalCtx, ReqCtx>>>>(
                        &p.name,
                        &Info::new(p.type_name.clone(), info.type_defs.clone()),
                    ),
                    (_, true, false, PropertyKind::Rel(_)) => registry
                        .field::<Rel<GlobalCtx, ReqCtx>>(
                            &p.name,
                            &Info::new(p.type_name.clone(), info.type_defs.clone()),
                        ),
                    (_, true, false, _) => registry.field::<Node<GlobalCtx, ReqCtx>>(
                        &p.name,
                        &Info::new(p.type_name.clone(), info.type_defs.clone()),
                    ),
                    (_, true, true, PropertyKind::Rel(_)) => registry
                        .field::<Vec<&Rel<GlobalCtx, ReqCtx>>>(
                            &p.name,
                            &Info::new(p.type_name.clone(), info.type_defs.clone()),
                        ),
                    (_, true, true, _) => registry.field::<Vec<&Node<GlobalCtx, ReqCtx>>>(
                        &p.name,
                        &Info::new(p.type_name.clone(), info.type_defs.clone()),
                    ),
                };

                for arg in p.arguments.values() {
                    f = match (arg.name.as_str(), arg.type_name.as_str(), &arg.kind) {
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
                            f.argument(registry.arg::<Option<Input<GlobalCtx, ReqCtx>>>(
                                "input",
                                &Info::new(type_name.to_string(), info.type_defs.clone()),
                            ))
                        }
                        ("input", type_name, ArgumentKind::Required) => {
                            f.argument(registry.arg::<Input<GlobalCtx, ReqCtx>>(
                                "input",
                                &Info::new(type_name.to_string(), info.type_defs.clone()),
                            ))
                        }
                        (name, type_name, required) => panic!(Error::new(
                            ErrorKind::UnexpectedSchemaArgument(
                                name.to_string(),
                                type_name.to_string(),
                                format!("{:#?}", required)
                            ),
                            None
                        )),
                    };
                }

                f
            })
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Node<GlobalCtx, ReqCtx>>(info, &fields)
            .into_meta()
    }

    #[cfg(any(feature = "graphson2", feature = "neo4j"))]
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
        let sn = Self::name(info).ok_or_else(|| {
            Error::new(ErrorKind::MissingSchemaElement(info.name.to_owned()), None)
        })?;
        trace!(
            "Node::resolve_field_with_transaction called -- sn: {}, field_name: {}",
            sn,
            field_name,
        );

        let td = info.get_type_def()?;
        let p = td.get_prop(field_name)?;
        let input_opt: Option<Input<GlobalCtx, ReqCtx>> = args.get("input");
        let partition_key_opt: &Option<String> = &args.get("partitionKey");

        let r = match &p.kind {
            PropertyKind::CustomResolver => {
                resolve_custom_endpoint(info, field_name, args, executor)
            }
            PropertyKind::DynamicScalar => {
                resolve_custom_field(info, field_name, &p.resolver, args, executor)
            }
            PropertyKind::Input => Err(Error::new(
                ErrorKind::InvalidPropertyType("PropertyKind::Input".to_owned()),
                None,
            )
            .into()),
            PropertyKind::NodeCreateMutation => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_node_create_mutation(
                    field_name,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::NodeDeleteMutation(deltype) => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_node_delete_mutation(
                    field_name,
                    &deltype,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::NodeUpdateMutation => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_node_update_mutation(
                    field_name,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::Object => resolve_object_field(
                field_name,
                self.fields.get("id"),
                info,
                partition_key_opt,
                input_opt,
                executor,
                transaction,
            ),
            PropertyKind::Rel(rel_name) => resolve_rel_field(
                field_name,
                self.fields.get("id").cloned(),
                rel_name,
                info,
                partition_key_opt,
                input_opt,
                executor,
                transaction,
            ),
            PropertyKind::RelCreateMutation(src_label, rel_name) => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_rel_create_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::RelDeleteMutation(src_label, rel_name) => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_rel_delete_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::RelUpdateMutation(src_label, rel_name) => {
                let input = input_opt.ok_or_else(|| {
                    Error::new(ErrorKind::MissingArgument("input".to_owned()), None)
                })?;
                resolve_rel_update_mutation(
                    field_name,
                    src_label,
                    rel_name,
                    info,
                    partition_key_opt,
                    input,
                    executor,
                    transaction,
                )
            }
            PropertyKind::Scalar => resolve_scalar_field(info, field_name, &self.fields, executor),
            PropertyKind::Union => Err(Error::new(
                ErrorKind::InvalidPropertyType("PropertyKind::Union".to_owned()),
                None,
            )
            .into()),
            PropertyKind::VersionQuery => resolve_static_version_query(info, args, executor),
        };
        trace!("Node::resolve_field_with_transaction Response: {:#?}", r);
        r
    }
}

impl<GlobalCtx, ReqCtx> GraphQLType for Node<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    type Context = GraphQLContext<GlobalCtx, ReqCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name)
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_defs
            .get(&info.name)
            .unwrap_or_else(|| {
                error!("Node::concrete_type_name panicking on type: {}", info.name);
                panic!(Error::new(
                    ErrorKind::MissingSchemaElement(info.name.to_owned()),
                    None
                ))
            })
            .type_name
            .to_owned();
        trace!(
            "Node::concrete_type_name called on {:#?}, returning {:#?}",
            info.name,
            tn
        );

        tn
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Node::meta called for {}", info.name);
        let nt = info.type_defs.get(&info.name).unwrap_or_else(|| {
            error!("Node::meta panicking on type: {}", info.name.to_string());
            panic!(Error::new(
                ErrorKind::MissingSchemaElement(info.name.to_string()),
                None
            ))
        });

        match nt.type_kind {
            TypeKind::Union => Node::<GlobalCtx, ReqCtx>::union_meta(nt, info, registry),
            _ => Node::<GlobalCtx, ReqCtx>::object_meta(nt, info, registry),
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
        let sn = Self::name(info).ok_or_else(|| {
            Error::new(ErrorKind::MissingSchemaElement(info.name.to_owned()), None)
        })?;
        trace!(
            "Node::resolve_field called -- sn: {}, field_name: {}",
            sn,
            field_name,
        );

        match &executor.context().pool {
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
            #[cfg(feature = "graphson2")]
            DatabasePool::Graphson2(c) => {
                let mut transaction = Graphson2Transaction::new(c.clone());
                self.resolve_field_with_transaction(
                    info,
                    field_name,
                    args,
                    executor,
                    &mut transaction,
                )
            }
            DatabasePool::NoDatabase => Err(Error::new(
                ErrorKind::UnsupportedDatabase("no database".to_owned()),
                None,
            )
            .into()),
        }
    }

    fn resolve_into_type(
        &self,
        info: &Self::TypeInfo,
        type_name: &str,
        _selection_set: Option<&[Selection]>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        let sn = Self::name(info).ok_or_else(|| {
            Error::new(ErrorKind::MissingSchemaElement(info.name.to_owned()), None)
        })?;

        trace!(
            "Node::resolve_into_type called -- sn: {}, ti.name: {}, type_name: {}, self.concrete_typename: {}",
            sn,
            info.name,
            type_name,
            self.concrete_typename
        );

        executor.resolve(
            &Info::new(self.concrete_typename.to_owned(), info.type_defs.clone()),
            &Some(self),
        )
    }
}

#[derive(Debug)]
pub struct Rel<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    id: Value,
    props: Option<Node<GlobalCtx, ReqCtx>>,
    src: Node<GlobalCtx, ReqCtx>,
    dst: Node<GlobalCtx, ReqCtx>,
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<ReqCtx>,
}

impl<GlobalCtx: Debug, ReqCtx> Rel<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    pub fn new(
        id: Value,
        props: Option<Node<GlobalCtx, ReqCtx>>,
        src: Node<GlobalCtx, ReqCtx>,
        dst: Node<GlobalCtx, ReqCtx>,
    ) -> Rel<GlobalCtx, ReqCtx> {
        Rel {
            id,
            props,
            src,
            dst,
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, ReqCtx> GraphQLType for Rel<GlobalCtx, ReqCtx>
where
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    type Context = GraphQLContext<GlobalCtx, ReqCtx>;
    type TypeInfo = Info;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name)
    }

    fn concrete_type_name(&self, _context: &Self::Context, info: &Self::TypeInfo) -> String {
        let tn = info
            .type_defs
            .get(&info.name)
            .unwrap_or_else(|| {
                error!("Rel::concrete_type_name panicking on type: {}", info.name);
                panic!(Error::new(
                    ErrorKind::MissingSchemaElement(info.name.to_owned()),
                    None
                ))
            })
            .type_name
            .to_owned();
        trace!(
            "Rel::concrete_type_name called on {:#?}, returning {:#?}",
            info.name,
            tn
        );

        tn
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r>
    where
        DefaultScalarValue: 'r,
    {
        trace!("Rel::meta called for {}", info.name);
        let nt = info.type_defs.get(&info.name).unwrap_or_else(|| {
            error!("Rel::meta panicking on type: {}", info.name.to_string());
            panic!(Error::new(
                ErrorKind::MissingSchemaElement(info.name.to_string()),
                None
            ))
        });

        let mut props = nt.props.values().collect::<Vec<&Property>>();
        props.sort_by_key(|&p| &p.name);

        let fields = props
            .iter()
            .map(|p| match (p.type_name.as_str(), p.required, p.list) {
                ("ID", false, false) => registry.field::<Option<ID>>(&p.name, &()),
                ("ID", false, true) => registry.field::<Option<Vec<ID>>>(&p.name, &()),
                ("ID", true, false) => registry.field::<ID>(&p.name, &()),
                ("ID", true, true) => registry.field::<Vec<ID>>(&p.name, &()),
                (_, false, false) => registry.field::<Option<Node<GlobalCtx, ReqCtx>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, false, true) => registry.field::<Option<Vec<&Node<GlobalCtx, ReqCtx>>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, true, false) => registry.field::<Node<GlobalCtx, ReqCtx>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
                (_, true, true) => registry.field::<Vec<&Node<GlobalCtx, ReqCtx>>>(
                    &p.name,
                    &Info::new(p.type_name.clone(), info.type_defs.clone()),
                ),
            })
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Rel<GlobalCtx, ReqCtx>>(info, &fields)
            .into_meta()
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        let sn = Self::name(info).ok_or_else(|| {
            Error::new(ErrorKind::MissingSchemaElement(info.name.to_owned()), None)
        })?;
        trace!(
            "Rel::resolve_field called -- sn: {}, field_name: {}",
            sn,
            field_name,
        );

        let td = info.get_type_def()?;
        let p = td.get_prop(field_name)?;

        let r = match (&p.kind, &field_name) {
            (PropertyKind::DynamicScalar, _) => {
                resolve_custom_field(info, field_name, &p.resolver, args, executor)
            }
            (PropertyKind::Object, &"props") => match &self.props {
                Some(p) => resolve_rel_props(info, field_name, p, executor),
                None => Err(Error::new(
                    ErrorKind::InvalidPropertyType(format!("{:#?}", p.kind)),
                    None,
                )
                .into()),
            },
            (PropertyKind::Object, &"src") => executor.resolve(
                &Info::new(
                    self.src.concrete_typename.to_owned(),
                    info.type_defs.clone(),
                ),
                &self.src,
            ),
            (PropertyKind::Object, _) => Err(Error::new(
                ErrorKind::MissingProperty(field_name.to_owned(), None),
                None,
            )
            .into()),
            (PropertyKind::Scalar, _) => {
                let mut m = HashMap::new();
                m.insert("id".to_string(), self.id.clone());
                resolve_scalar_field(info, field_name, &m, executor)
            }
            (PropertyKind::Union, _) => {
                resolve_union_field(info, field_name, &self.src, &self.dst, executor)
            }
            (_, _) => Err(Error::new(
                ErrorKind::InvalidPropertyType(format!("{:#?}", p.kind)),
                None,
            )
            .into()),
        };
        trace!("Rel::resolve_field Response: {:#?}", r);
        r
    }
}
