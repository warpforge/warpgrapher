#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use super::visitors::{
    visit_node_create_mutation_input, visit_node_delete_input, visit_node_query_input,
    visit_node_update_input, visit_rel_create_input, visit_rel_delete_input, visit_rel_query_input,
    visit_rel_update_input, SuffixGenerator,
};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use super::Input;
use super::Node;
use crate::engine::context::{GraphQLContext, RequestContext};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use crate::engine::database::{QueryResult, Transaction};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::{Error, ErrorKind};
use core::hash::BuildHasher;
use juniper::{Arguments, ExecutionResult, Executor};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use log::debug;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
pub(super) fn resolve_custom_endpoint<GlobalCtx, ReqCtx>(
    info: &Info,
    field_name: &str,
    args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    ReqCtx: RequestContext,
{
    trace!(
        "resolve_custom_endpoint called -- field_name: {}, info.name: {:#?}",
        field_name,
        info.name(),
    );

    let func = executor.context().resolver(field_name)?;
    func(info, args, executor)
}

pub(super) fn resolve_custom_field<GlobalCtx, ReqCtx>(
    info: &Info,
    field_name: &str,
    resolver_name: &Option<String>,
    args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalCtx, ReqCtx>>,
) -> ExecutionResult
where
    ReqCtx: RequestContext,
{
    trace!(
        "resolve_custom_field called -- field_name: {:#?}, info.name: {:#?}",
        field_name,
        info.name(),
    );

    let resolver_name = resolver_name.as_ref().ok_or_else(|| {
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

    let func = executor.context().resolver(resolver_name)?;

    func(info, args, executor)
}

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
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
