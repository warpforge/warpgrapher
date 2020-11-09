use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{ClauseType, NodeQueryVar, RelQueryVar, Transaction};
use crate::engine::objects::resolvers::SuffixGenerator;
use crate::engine::schema::{Info, PropertyKind};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use log::trace;
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_create_mutation_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_create_mutation_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, clause, info.name(), partition_key_opt
    );

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        m.keys().try_for_each(|k| {
            let p = itd.property(k)?;
            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => p
                    .validator()
                    .map_or(Ok(()), |v_name| validate_input(validators, &v_name, &input)),
                _ => Ok(()), // No validation action to take
            }
        })?
    }

    if let Value::Map(m) = input {
        let (props, inputs) = m.into_iter().try_fold(
            (HashMap::new(), HashMap::new()),
            |(mut props, mut inputs), (k, v)| {
                match itd.property(&k)?.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        props.insert(k, v);
                    }
                    PropertyKind::Input => {
                        inputs.insert(k, v);
                    }
                    _ => return Err(Error::TypeNotExpected),
                }
                Ok((props, inputs))
            },
        )?;

        let (rel_create_fragments, params) = inputs.into_iter().try_fold(
            (Vec::new(), params),
            |(mut rel_create_fragments, params),
             (k, v)|
             -> Result<(Vec<String>, HashMap<String, Value>), Error> {
                let p = itd.property(&k)?;

                let (rel_create_fragments, params) = match p.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        Ok((rel_create_fragments, params))
                    } // Handled earlier
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            input_array.into_iter().try_fold(
                                (rel_create_fragments, params),
                                |(mut rel_create_fragments, params), val| {
                                    let (fragment, params) = visit_rel_create_mutation_input::<
                                        T,
                                        GlobalCtx,
                                        RequestCtx,
                                    >(
                                        params,
                                        &RelQueryVar::new(
                                            p.name().to_string(),
                                            sg.suffix(),
                                            node_var.clone(),
                                            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                        ),
                                        None,
                                        val,
                                        ClauseType::SubQuery,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        sg,
                                        transaction,
                                        validators,
                                    )?;
                                    rel_create_fragments.push(fragment);
                                    Ok((rel_create_fragments, params))
                                },
                            )
                        } else {
                            let (fragment, params) =
                                visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                    params,
                                    &RelQueryVar::new(
                                        p.name().to_string(),
                                        sg.suffix(),
                                        node_var.clone(),
                                        NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                    ),
                                    None,
                                    v,
                                    ClauseType::SubQuery,
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    sg,
                                    transaction,
                                    validators,
                                )?;
                            rel_create_fragments.push(fragment);
                            Ok((rel_create_fragments, params))
                        }
                    }
                    _ => Err(Error::TypeNotExpected),
                }?;

                Ok((rel_create_fragments, params))
            },
        )?;

        transaction.node_create_query::<GlobalCtx, RequestCtx>(
            rel_create_fragments,
            params,
            node_var,
            props,
            clause,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

pub(super) fn visit_node_delete_input<T, GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, info.name(), partition_key_opt
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            node_var,
            m.remove("$MATCH"), // Remove used to take ownership
            ClauseType::SubQuery,
            &Info::new(
                itd.property("$MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let (match_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            &node_var,
            ClauseType::Parameter,
        )?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            &node_var,
            Some(m.remove("$DELETE").ok_or_else(|| {
                // remove used to take ownership
                Error::InputItemNotFound {
                    name: "input::$DELETE".to_string(),
                }
            })?),
            ClauseType::Query,
            &Info::new(
                itd.property("$DELETE")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Option<Value>,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    let itd = info.type_def()?;

    let (rel_delete_fragments, params) = if let Some(Value::Map(m)) = input {
        m.into_iter()
            .try_fold((Vec::new(), params), |(mut queries, params), (k, v)| {
                let p = itd.property(&k)?;

                match p.kind() {
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            input_array.into_iter().try_fold(
                                (queries, params),
                                |(mut queries, params), val| {
                                    let rel_var = RelQueryVar::new(k.to_string(), sg.suffix(), node_var.clone(), NodeQueryVar::new(None, "dst".to_string(), sg.suffix()));
                                    let (query, params) = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                        params,
                                        &rel_var,
                                        val,
                                        ClauseType::SubQuery,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        sg,
                                        transaction,
                                    )?;
                                    queries.push(query);
                                    Ok((queries, params))
                                },
                            )
                        } else {
                                    let rel_var = RelQueryVar::new(k.to_string(), sg.suffix(), node_var.clone(), NodeQueryVar::new(None, "dst".to_string(), sg.suffix()));
                            let (query, params) = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                params,
                                &rel_var,
                                v,
                                ClauseType::SubQuery,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                sg,
                                transaction,
                            )?;

                            queries.push(query);
                            Ok((queries, params))
                        }
                    }
                    _ => Err(Error::TypeNotExpected),
                }
            })?
    } else {
        (Vec::new(), params)
    };

    transaction.node_delete_query(
        match_query,
        rel_delete_fragments,
        params,
        node_var,
        clause,
        sg,
    )
}

#[allow(clippy::too_many_arguments)]
fn visit_node_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_input Called -- params: {:#?}, node_var: {:#?}, input: {:#?}, clause: {:#?}, into.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, clause, info.name(), partition_key_opt
    );

    if let Value::Map(m) = input {
        let itd = info.type_def()?;

        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string() + "::$NEW or ::$EXISTING",
            })?;

        let p = itd.property(&k)?;

        match k.as_ref() {
            "$NEW" => visit_node_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                params,
                node_var,
                v,
                clause,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
                validators,
            ),
            "$EXISTING" => {
                let (match_fragment, where_fragment, params) = visit_node_query_input(
                    params,
                    node_var,
                    Some(v),
                    clause,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    partition_key_opt,
                    sg,
                    transaction,
                )?;

                transaction.node_read_query(
                    &match_fragment,
                    &where_fragment,
                    params,
                    &node_var,
                    ClauseType::SubQuery,
                )
            }
            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + &k,
            }),
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_query_input<T>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Option<Value>,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!("visit_node_query_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
    params, node_var, input, clause, info.name(), partition_key_opt);

    let itd = info.type_def()?;
    let dst_var = NodeQueryVar::new(None, "dst".to_string(), sg.suffix());

    let mut props = HashMap::new();
    if let Some(Value::Map(m)) = input {
        let (rqfs, params) =
            m.into_iter()
                .try_fold((Vec::new(), params), |(mut rqfs, params), (k, v)| {
                    itd.property(&k)
                        .map_err(|e| e)
                        .and_then(|p| match p.kind() {
                            PropertyKind::Scalar => {
                                props.insert(k, v);
                                Ok((rqfs, params))
                            }
                            PropertyKind::Input => {
                                let (match_fragment, where_fragment, params) =
                                    visit_rel_query_input(
                                        params,
                                        &RelQueryVar::new(
                                            k.to_string(),
                                            sg.suffix(),
                                            node_var.clone(),
                                            dst_var.clone(),
                                        ),
                                        Some(v),
                                        ClauseType::Parameter,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        sg,
                                        transaction,
                                    )?;
                                rqfs.push((match_fragment, where_fragment));
                                Ok((rqfs, params))
                            }
                            _ => Err(Error::TypeNotExpected),
                        })
                })?;

        transaction.node_read_fragment(rqfs, params, &node_var, props, clause, sg)
    } else {
        transaction.node_read_fragment(Vec::new(), params, &node_var, props, clause, sg)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_update_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, info.name(), partition_key_opt
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            node_var,
            m.remove("$MATCH"), // Remove used to take ownership
            ClauseType::SubQuery,
            &Info::new(
                itd.property("$MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let (match_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            &node_var,
            ClauseType::Parameter,
        )?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            node_var,
            m.remove("$SET").ok_or_else(|| {
                // remove() used here to take ownership of the "set" value, not borrow it
                Error::InputItemNotFound {
                    name: "input::$SET".to_string(),
                }
            })?,
            ClauseType::Query,
            &Info::new(
                itd.property("$SET")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
            validators,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_mutation_input called -- match_query: {}, params: {:#?}, node_var: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        match_query, params, node_var, input, clause, info.name(), partition_key_opt,
    );

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        m.keys().try_for_each(|k| {
            let p = itd.property(k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => p
                    .validator()
                    .map_or(Ok(()), |v_name| validate_input(validators, &v_name, &input)),
                _ => Ok(()), // No validation action to take
            }
        })?;
    }

    if let Value::Map(m) = input {
        let (props, inputs) = m.into_iter().try_fold(
            (HashMap::new(), HashMap::new()),
            |(mut props, mut inputs), (k, v)| {
                match itd.property(&k)?.kind() {
                    PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                        props.insert(k, v);
                    }
                    PropertyKind::Input => {
                        inputs.insert(k, v);
                    }
                    _ => return Err(Error::TypeNotExpected),
                }
                Ok((props, inputs))
            },
        )?;

        let (change_queries, params) =
            inputs.into_iter().try_fold(
                (Vec::new(), params),
                |(mut queries, params), (k, v)| {
                    let p = itd.property(&k)?;

                    match p.kind() {
                        PropertyKind::Scalar | PropertyKind::DynamicScalar => Ok((queries, params)), // Properties handled above
                        PropertyKind::Input => {
                            if let Value::Array(input_array) = v {
                                let (mut qs, params) =
                                    input_array.into_iter().try_fold(
                                        (Vec::new(), params),
                                        |(mut queries, params),
                                         val|
                                         -> Result<
                                            (Vec<String>, HashMap<String, Value>),
                                            Error,
                                        > {
                                            let (query, params) =
                                                visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                                                    params,
                                                    &RelQueryVar::new(
                                                        k.clone(),
                                                        sg.suffix(),
                                                        node_var.clone(),
                                                        NodeQueryVar::new(
                                                            None,
                                                            "dst".to_string(),
                                                            sg.suffix(),
                                                        ),
                                                    ),
                                                    val,
                                                    &Info::new(
                                                        p.type_name().to_owned(),
                                                        info.type_defs(),
                                                    ),
                                                    partition_key_opt,
                                                    sg,
                                                    transaction,
                                                    validators,
                                                )?;

                                            queries.push(query);
                                            Ok((queries, params))
                                        },
                                    )?;
                                queries.append(&mut qs);

                                Ok((queries, params))
                            } else {
                                let (query, params) =
                                    visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                                        params,
                                        &RelQueryVar::new(
                                            k,
                                            sg.suffix(),
                                            node_var.clone(),
                                            NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                                        ),
                                        v,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        sg,
                                        transaction,
                                        validators,
                                    )?;

                                queries.push(query);
                                Ok((queries, params))
                            }
                        }
                        _ => Err(Error::TypeNotExpected),
                    }
                },
            )?;

        transaction.node_update_query::<GlobalCtx, RequestCtx>(
            match_query,
            change_queries,
            params,
            node_var,
            props,
            clause,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_change_input called -- params: {:#?}, rel_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, rel_var, input, info.name(), partition_key_opt
    );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("$ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                params,
                rel_var,
                None,
                v,
                ClauseType::SubQuery,
                &Info::new(
                    itd.property("$ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                validators,
            )
        } else if let Some(v) = m.remove("$DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                params,
                rel_var,
                v,
                ClauseType::SubQuery,
                &Info::new(
                    itd.property("$DELETE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
            )
        } else if let Some(v) = m.remove("$UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
                params,
                rel_var,
                None,
                v,
                ClauseType::SubQuery,
                &Info::new(
                    itd.property("$UPDATE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                validators,
            )
        } else {
            Err(Error::InputItemNotFound {
                name: itd.type_name().to_string() + "::$ADD|$DELETE|$UPDATE",
            })
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(super) fn visit_rel_create_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    src_var: &NodeQueryVar,
    rel_name: &str,
    props_type_name: Option<&str>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_create_input called -- params: {:#?}, src_var: {:#?}, rel_name: {}, input: {:#?}, info.name: {}, partition_key_opt {:#?}",
        params, src_var, rel_name, input, info.name(), partition_key_opt
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            src_var,
            m.remove("$MATCH"), // Remove used to take ownership
            ClauseType::SubQuery,
            &Info::new(
                itd.property("$MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let (src_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            src_var,
            ClauseType::Parameter,
        )?;

        let create_input = m.remove("$CREATE").ok_or_else(|| {
            // Using remove to take ownership
            Error::InputItemNotFound {
                name: "input::$CREATE".to_string(),
            }
        })?;

        match create_input {
            Value::Map(_) => {
                let rel_var = RelQueryVar::new(
                    rel_name.to_string(),
                    sg.suffix(),
                    src_var.clone(),
                    NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                );
                let (cf, params) = visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                    params,
                    &rel_var,
                    props_type_name,
                    create_input,
                    ClauseType::SubQuery,
                    &Info::new(
                        itd.property("$CREATE")?.type_name().to_owned(),
                        info.type_defs(),
                    ),
                    partition_key_opt,
                    sg,
                    transaction,
                    validators,
                )?;

                transaction.rel_create_query::<GlobalCtx, RequestCtx>(
                    Some(src_query),
                    vec![cf],
                    params,
                    vec![rel_var],
                    ClauseType::Query,
                )
            }
            Value::Array(create_input_array) => {
                let (rcfs, params, rel_vars) =
                    create_input_array.into_iter().try_fold(
                        (Vec::new(), params, Vec::new()),
                        |(mut rcfs, params, mut rel_vars),
                         create_input_value|
                         -> Result<
                            (Vec<String>, HashMap<String, Value>, Vec<RelQueryVar>),
                            Error,
                        > {
                            let rel_var = RelQueryVar::new(
                                rel_name.to_string(),
                                sg.suffix(),
                                src_var.clone(),
                                NodeQueryVar::new(None, "dst".to_string(), sg.suffix()),
                            );
                            let (query, params) =
                                visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                    params,
                                    &rel_var,
                                    props_type_name,
                                    create_input_value,
                                    ClauseType::SubQuery,
                                    &Info::new(
                                        itd.property("$CREATE")?.type_name().to_owned(),
                                        info.type_defs(),
                                    ),
                                    partition_key_opt,
                                    sg,
                                    transaction,
                                    validators,
                                )?;

                            rcfs.push(query);
                            rel_vars.push(rel_var);
                            Ok((rcfs, params, rel_vars))
                        },
                    )?;

                transaction.rel_create_query::<GlobalCtx, RequestCtx>(
                    Some(src_query),
                    rcfs,
                    params,
                    rel_vars,
                    ClauseType::Query,
                )
            }
            _ => Err(Error::TypeNotExpected),
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_create_mutation_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_create_mutation_input called -- params: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
            params, rel_var, props_type_name, input, clause, info.name(), partition_key_opt,
        );

    if let Value::Map(mut m) = input {
        let dst_prop = info.type_def()?.property("dst")?;
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::InputItemNotFound {
                name: "dst".to_string(),
            })?;
        let (dst_query, params) = visit_rel_nodes_mutation_input_union::<T, GlobalCtx, RequestCtx>(
            params,
            rel_var.dst(),
            dst,
            ClauseType::SubQuery,
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            validators,
        )?;

        let props = match m.remove("props") {
            None => HashMap::new(),
            Some(Value::Map(hm)) => hm,
            Some(_) => return Err(Error::TypeNotExpected),
        };

        transaction.rel_create_fragment::<GlobalCtx, RequestCtx>(
            &dst_query, params, &rel_var, props, clause, sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_delete_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!("visit_rel_delete_input called -- params: {:#?}, rel_var: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
    params, rel_var, input, clause, info.name(), partition_key_opt);

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let (match_fragment, where_fragment, params) = visit_rel_query_input(
            params,
            rel_var,
            m.remove("$MATCH"), // remove rather than get to take ownership
            ClauseType::SubQuery,
            &Info::new(
                itd.property("$MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let (match_query, params) = transaction.rel_read_query(
            &match_fragment,
            &where_fragment,
            params,
            &rel_var,
            if let ClauseType::Query = clause {
                ClauseType::FirstSubQuery
            } else {
                ClauseType::SubQuery
            },
        )?;

        let (src_delete_query_opt, params) = if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            let (query, params) = visit_rel_src_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query.clone(),
                params,
                rel_var.src(),
                src,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
            )?;
            (Some(query), params)
        } else {
            (None, params)
        };

        let (dst_delete_query_opt, params) = if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            let (query, params) = visit_rel_dst_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query.clone(),
                params,
                rel_var.dst(),
                dst,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
            )?;
            (Some(query), params)
        } else {
            (None, params)
        };

        transaction.rel_delete_query(
            match_query,
            src_delete_query_opt,
            dst_delete_query_opt,
            params,
            &rel_var,
            clause,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_rel_dst_delete_mutation_input called -- match_query: {}, params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt, {:#?}",
        match_query, params, node_var, input, info.name(), partition_key_opt
    );

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            node_var,
            Some(v),
            ClauseType::SubQuery,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::type_complexity)]
fn visit_rel_dst_query_input<T>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(Option<(String, String)>, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!("visit_rel_dst_query_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, info.name(), partition_key_opt);

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (match_fragment, where_fragment, params) = visit_node_query_input(
                params,
                node_var,
                Some(v),
                ClauseType::Parameter,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?;

            Ok((Some((match_fragment, where_fragment)), params))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_dst_update_mutation_input called -- match_query: {}, params: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        match_query, params, input, info.name(), partition_key_opt);

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            &NodeQueryVar::new(Some(k), "dst".to_string(), sg.suffix()),
            v,
            ClauseType::SubQuery,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            validators,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_nodes_mutation_input_union<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!("visit_rel_nodes_mutation_input_union called -- params: {:#?}, node_var: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, clause, info.name(), partition_key_opt);

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_input::<T, GlobalCtx, RequestCtx>(
            params,
            &NodeQueryVar::new(
                Some(k.clone()),
                node_var.base().to_string(),
                node_var.suffix().to_string(),
            ),
            v,
            clause,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            validators,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_query_input<T>(
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    input_opt: Option<Value>,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!("visit_rel_query_input called -- params: {:#?}, rel_var: {:#?}, input_opt: {:#?}, clause: {:#?}, info.name(): {}, partition_key_opt: {:#?}",
        params, rel_var, input_opt, clause, info.name(), partition_key_opt);

    let itd = info.type_def()?;
    let src_prop = itd.property("src")?;
    let dst_prop = itd.property("dst")?;

    if let Some(Value::Map(mut m)) = input_opt {
        let mut props = if let Some(Value::Map(rel_props)) = m.remove("props") {
            rel_props
        } else {
            HashMap::new()
        };

        // uses remove in order to take ownership
        if let Some(id) = m.remove("id") {
            props.insert("id".to_owned(), id);
        }

        // Remove used to take ownership
        let (src_query_opt, params) = if let Some(src) = m.remove("src") {
            visit_rel_src_query_input(
                params,
                rel_var.src(),
                Some(src),
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?
        } else {
            (None, params)
        };

        // Remove used to take ownership
        let (dst_query_opt, params) = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                params,
                rel_var.dst(),
                Some(dst),
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?
        } else {
            (None, params)
        };

        transaction.rel_read_fragment(src_query_opt, dst_query_opt, params, rel_var, props, sg)
    } else {
        transaction.rel_read_fragment(None, None, params, rel_var, HashMap::new(), sg)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- match_query: {}, params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        match_query, params, node_var, input, info.name(), partition_key_opt);

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            node_var,
            Some(v),
            ClauseType::SubQuery,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_src_update_mutation_input called -- match_query: {}, params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        match_query, params, node_var, input, info.name(), partition_key_opt);

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            node_var,
            v,
            ClauseType::SubQuery,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            sg,
            transaction,
            validators,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn visit_rel_src_query_input<T>(
    params: HashMap<String, Value>,
    node_var: &NodeQueryVar,
    input: Option<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
) -> Result<(Option<(String, String)>, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_rel_src_query_input called -- params: {:#?}, node_var: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
        params, node_var, input, info.name(), partition_key_opt);

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (match_fragment, where_fragment, params) = visit_node_query_input(
                params,
                node_var,
                Some(v),
                ClauseType::Parameter,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                sg,
                transaction,
            )?;

            Ok((Some((match_fragment, where_fragment)), params))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_update_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_input called -- params: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, info.name: {}, partition_key_opt: {:#?}",
         params, rel_var, props_type_name, input, info.name(), partition_key_opt);

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let (match_fragment, where_fragment, params) = visit_rel_query_input(
            params,
            &rel_var,
            m.remove("$MATCH"), // uses remove to take ownership
            ClauseType::Parameter,
            &Info::new(
                itd.property("$MATCH")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            sg,
            transaction,
        )?;

        let (match_query, params) = transaction.rel_read_query(
            &match_fragment,
            &where_fragment,
            params,
            &rel_var,
            if let ClauseType::Query = clause {
                ClauseType::FirstSubQuery
            } else {
                ClauseType::SubQuery
            },
        )?;

        trace!(
            "visit_rel_update_input -- match_query: {}, params: {:#?}",
            match_query,
            params
        );

        if let Some(update) = m.remove("$SET") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query,
                params,
                &rel_var,
                props_type_name,
                update,
                clause,
                &Info::new(
                    itd.property("$SET")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                validators,
            )
        } else {
            Err(Error::InputItemNotFound {
                name: "input::$SET".to_string(),
            })
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    rel_var: &RelQueryVar,
    props_type_name: Option<&str>,
    input: Value,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    sg: &mut SuffixGenerator,
    transaction: &mut T,
    validators: &Validators,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_mutation_input called -- match_query: {}, params: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, input: {:#?}, clause: {:#?}, info.name: {}, partition_key_opt: {:#?}",
         match_query, params, rel_var, props_type_name, input, clause, info.name(), partition_key_opt);

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let props = if let Some(Value::Map(props)) = m.remove("props") {
            props
        } else {
            HashMap::new()
        };

        let results = transaction.rel_update_query::<GlobalCtx, RequestCtx>(
            match_query.clone(),
            params.clone(),
            rel_var,
            props,
            clause,
            sg,
        )?;

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query.clone(),
                params.clone(),
                rel_var.src(),
                src,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                validators,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query,
                params,
                dst,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                sg,
                transaction,
                validators,
            )?;
        }

        Ok(results)
    } else {
        Err(Error::TypeNotExpected)
    }
}

fn validate_input(validators: &Validators, v: &str, input: &Value) -> Result<(), Error> {
    let func = validators.get(v).ok_or_else(|| Error::ValidatorNotFound {
        name: v.to_string(),
    })?;

    trace!(
        "validate_input Calling input validator function {} for input value {:#?}",
        v,
        input
    );

    func(input)
}
