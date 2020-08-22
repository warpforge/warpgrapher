use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{ClauseType, Transaction};
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
    node_var: &str,
    label: &str,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_create_mutation_input called -- params: {:#?}, label: {}, info.name: {}",
        params,
        label,
        info.name(),
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
                                    let rel_var = "rel".to_string() + &sg.suffix();
                                    let dst_var = "dst".to_string() + &sg.suffix();
                                    let (fragment, params) = visit_rel_create_mutation_input::<
                                        T,
                                        GlobalCtx,
                                        RequestCtx,
                                    >(
                                        String::new(),
                                        params,
                                        label,
                                        &node_var,
                                        &rel_var,
                                        p.name(),
                                        &dst_var,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        val,
                                        validators,
                                        None,
                                        ClauseType::SubQuery(rel_var.clone()),
                                        // clause.clone(),
                                        transaction,
                                        sg,
                                    )?;
                                    rel_create_fragments.push(fragment);
                                    Ok((rel_create_fragments, params))
                                },
                            )
                        } else {
                            let rel_var = "rel".to_string() + &sg.suffix();
                            let dst_var = "dst".to_string() + &sg.suffix();
                            let (fragment, params) =
                                visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                    String::new(),
                                    params,
                                    label,
                                    &node_var,
                                    &rel_var,
                                    p.name(),
                                    &dst_var,
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    v,
                                    validators,
                                    None,
                                    ClauseType::SubQuery(rel_var.to_string()),
                                    // clause.clone(),
                                    transaction,
                                    sg,
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
            &node_var,
            label,
            props,
            clause,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_delete_input<T, GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
    params: HashMap<String, Value>,
    label: &str,
    node_var: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_input called -- info.name: {}, label: {}, input: {:#?}",
        info.name(),
        label,
        input
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            label,
            node_var,
            true,
            false,
            ClauseType::SubQuery(node_var.to_string()),
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            sg,
        )?;

        let (match_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            label,
            node_var,
            true,
            false,
            ClauseType::Parameter(node_var.to_string()),
            &sg.suffix(),
            HashMap::new(),
        )?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            node_var,
            label,
            &Info::new(
                itd.property("delete")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            Some(m.remove("delete").ok_or_else(|| {
                // remove used to take ownership
                Error::InputItemNotFound {
                    name: "input::delete".to_string(),
                }
            })?),
            transaction,
            sg,
            true,
            ClauseType::Query,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
    top_level_query: bool,
    clause: ClauseType,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_node_delete_mutation_input called -- params: {:#?}, node_var: {}, label: {}, info.name: {}, input: {:#?}",
        params,
        node_var, label,
        info.name(),
        input
    );

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
                                    let (query, params) = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                        params,
                                        node_var,
                                        label,
                                        &k,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        val,
                                        transaction,
                                        sg,
                                        false
                                    )?;
                                    queries.push(query);
                                    Ok((queries, params))
                                },
                            )
                        } else {
                            let (query, params) = visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                params,
                                node_var,
                                label,
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                v,
                                transaction,
                                sg,
                                false
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
        label,
        partition_key_opt,
        sg,
        top_level_query,
        clause,
    )
}

#[allow(clippy::too_many_arguments)]
fn visit_node_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_input Called -- label: {}, info.name: {}, input: {:#?}",
        label,
        info.name(),
        input
    );

    if let Value::Map(m) = input {
        let itd = info.type_def()?;

        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string() + "::NEW or ::EXISTING",
            })?;

        let p = itd.property(&k)?;

        match k.as_ref() {
            "NEW" => visit_node_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                params,
                node_var,
                label,
                // ClauseType::SubQuery(node_var.to_string()),
                clause,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                v,
                validators,
                transaction,
                sg,
            ),
            "EXISTING" => {
                let _node_suffix = sg.suffix();
                let (match_fragment, where_fragment, params) = visit_node_query_input(
                    params,
                    label,
                    node_var,
                    true,
                    false,
                    // ClauseType::SubQuery(node_var.to_string()),
                    clause,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    partition_key_opt,
                    Some(v),
                    transaction,
                    sg,
                )?;

                transaction.node_read_query(
                    &match_fragment,
                    &where_fragment,
                    params,
                    label,
                    node_var,
                    true,
                    false,
                    ClauseType::SubQuery(node_var.to_string()),
                    //clause,
                    &sg.suffix(),
                    HashMap::new(),
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
    label: &str,
    node_var: &str,
    name_node: bool,
    union_type: bool,
    clause: ClauseType,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_node_query_input called -- params: {:#?}, label: {}, node_var: {}, union_type: {}, clause: {:#?}, info.name: {}, input: {:#?}",
        params,
        label,
        node_var,
        union_type,
        clause,
        info.name(),
        input,
    );
    let itd = info.type_def()?;
    let param_suffix = sg.suffix();
    let dst_suffix = sg.suffix();
    let rel_suffix = sg.suffix();

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
                                        label,
                                        node_var,
                                        &k,
                                        &rel_suffix,
                                        &("dst".to_string() + &dst_suffix),
                                        &dst_suffix,
                                        ClauseType::Parameter(node_var.to_string()),
                                        false,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        Some(v),
                                        transaction,
                                        sg,
                                    )?;
                                rqfs.push((match_fragment, where_fragment));
                                Ok((rqfs, params))
                            }
                            _ => Err(Error::TypeNotExpected),
                        })
                })?;

        transaction.node_read_fragment(
            rqfs,
            params,
            label,
            node_var,
            name_node,
            union_type,
            &param_suffix,
            props,
            clause,
        )
    } else {
        transaction.node_read_fragment(
            Vec::new(),
            params,
            label,
            node_var,
            name_node,
            union_type,
            &param_suffix,
            props,
            clause,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_update_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_input called -- params: {:#?}, label: {}, info.name: {}, input: {:#?}",
        params,
        label,
        info.name(),
        input,
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let node_suffix = sg.suffix();

        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            label,
            &node_var,
            true,
            false,
            ClauseType::SubQuery(node_var.to_string()),
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            sg,
        )?;

        let (match_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            label,
            node_var,
            true,
            false,
            ClauseType::Parameter(node_var.to_string()),
            &sg.suffix(),
            HashMap::new(),
        )?;

        trace!(
            "visit_node_update_input -- match_fragment: {}, where_fragment: {}",
            match_fragment,
            where_fragment
        );

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            &node_var,
            &node_suffix,
            label,
            &Info::new(
                itd.property("modify")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("modify").ok_or_else(|| {
                // remove() used here to take ownership of the "modify" value, not borrow it
                Error::InputItemNotFound {
                    name: "input::modify".to_string(),
                }
            })?,
            validators,
            transaction,
            sg,
            ClauseType::Query,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    node_suffix: &str,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
    clause: ClauseType,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_mutation_input called -- match_query: {}, params: {:#?}, node_suffix: {}, label: {}, info.name: {}, input: {:#?}",
        match_query,
        params,
        node_suffix,
        label,
        info.name(),
        input
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
                                                    label,
                                                    node_var,
                                                    &k,
                                                    &Info::new(
                                                        p.type_name().to_owned(),
                                                        info.type_defs(),
                                                    ),
                                                    partition_key_opt,
                                                    val,
                                                    validators,
                                                    transaction,
                                                    sg,
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
                                        label,
                                        node_var,
                                        &k,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        v,
                                        validators,
                                        transaction,
                                        sg,
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
            label,
            node_var,
            props,
            partition_key_opt,
            info,
            sg,
            clause,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    src_label: &str,
    src_var: &str,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_change_input called -- src_label {}, src_var {}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_var,
         rel_name,
         info.name(),
         input
     );

    let itd = info.type_def()?;

    let rel_var = "rel".to_string() + &sg.suffix();
    let dst_var = "dst".to_string() + &sg.suffix();

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                String::new(),
                params,
                src_label,
                src_var,
                &rel_var,
                rel_name,
                &dst_var,
                &Info::new(
                    itd.property("ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                validators,
                None,
                // ClauseType::None,
                ClauseType::SubQuery(rel_var.clone()),
                transaction,
                sg,
            )
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                params,
                src_var,
                src_label,
                rel_name,
                &Info::new(
                    itd.property("DELETE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                transaction,
                sg,
                false,
            )
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
                params,
                src_label,
                src_var,
                rel_name,
                false,
                &Info::new(
                    itd.property("UPDATE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                validators,
                None,
                transaction,
                sg,
            )
        } else {
            Err(Error::InputItemNotFound {
                name: itd.type_name().to_string() + "::ADD|DELETE|UPDATE",
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
    src_label: &str,
    rel_name: &str,
    props_type_name: Option<&str>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_create_input called -- params: {:#?}, info.name: {}, rel_name {}, input: {:#?}",
        params,
        info.name(),
        rel_name,
        input
    );

    let src_var = "src".to_string() + &sg.suffix();

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let _src_suffix = sg.suffix();

        let (match_fragment, where_fragment, params) = visit_node_query_input(
            params,
            src_label,
            &src_var,
            true,
            false,
            ClauseType::SubQuery(src_var.clone()),
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            sg,
        )?;

        trace!(
            "visit_rel_create_input -- match_fragment: {}, where_fragment: {}",
            match_fragment,
            where_fragment
        );

        let (src_query, params) = transaction.node_read_query(
            &match_fragment,
            &where_fragment,
            params,
            src_label,
            &src_var,
            true,
            false,
            // ClauseType::SubQuery(src_var.clone()),
            ClauseType::Parameter(src_var.clone()),
            &sg.suffix(),
            HashMap::new(),
        )?;

        let create_input = m.remove("create").ok_or_else(|| {
            // Using remove to take ownership
            Error::InputItemNotFound {
                name: "input::create".to_string(),
            }
        })?;

        match create_input {
            Value::Map(_) => {
                let rel_var = "rel".to_string() + &sg.suffix();
                let dst_var = "dst".to_string() + &sg.suffix();

                let (cf, params) = visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                    String::new(),
                    params,
                    src_label,
                    &src_var,
                    &rel_var,
                    rel_name,
                    &dst_var,
                    &Info::new(
                        itd.property("create")?.type_name().to_owned(),
                        info.type_defs(),
                    ),
                    partition_key_opt,
                    create_input,
                    validators,
                    props_type_name,
                    ClauseType::SubQuery(rel_var.to_string()),
                    transaction,
                    sg,
                )?;

                transaction.rel_create_query::<GlobalCtx, RequestCtx>(
                    Some(src_query),
                    vec![cf],
                    &src_var,
                    src_label,
                    vec![rel_var],
                    vec![dst_var],
                    params,
                    sg,
                    ClauseType::Query,
                )
            }
            Value::Array(create_input_array) => {
                let (rcfs, rel_vars, dst_vars, params) = create_input_array.into_iter().try_fold(
                    (Vec::new(), Vec::new(), Vec::new(), params),
                    |(mut rcfs, mut rel_vars, mut dst_vars, params),
                     create_input_value|
                     -> Result<
                        (
                            Vec<String>,
                            Vec<String>,
                            Vec<String>,
                            HashMap<String, Value>,
                        ),
                        Error,
                    > {
                        let rel_var = "rel".to_string() + &sg.suffix();
                        let dst_var = "dst".to_string() + &sg.suffix();
                        let (query, params) =
                            visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                String::new(),
                                params,
                                src_label,
                                &src_var,
                                &rel_var,
                                rel_name,
                                &dst_var,
                                &Info::new(
                                    itd.property("create")?.type_name().to_owned(),
                                    info.type_defs(),
                                ),
                                partition_key_opt,
                                create_input_value,
                                validators,
                                props_type_name,
                                ClauseType::SubQuery(rel_var.clone()),
                                transaction,
                                sg,
                            )?;

                        rcfs.push(query);
                        rel_vars.push(rel_var);
                        dst_vars.push(dst_var);
                        Ok((rcfs, rel_vars, dst_vars, params))
                    },
                )?;

                transaction.rel_create_query::<GlobalCtx, RequestCtx>(
                    Some(src_query),
                    rcfs,
                    &src_var,
                    src_label,
                    rel_vars,
                    dst_vars,
                    params,
                    sg,
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
    src_query: String,
    params: HashMap<String, Value>,
    src_label: &str,
    src_var: &str,
    rel_var: &str,
    rel_name: &str,
    dst_var: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    clause: ClauseType,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
            "visit_rel_create_mutation_input called -- src_query: {}, params: {:#?}, src_label: {}, src_var: {:#?}, rel_name: {}, info.name: {}, input: {:#?}",
            src_query,
            params,
            src_label,
            src_var,
            rel_name,
            info.name(),
            input
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
            dst_var,
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            dst,
            validators,
            transaction,
            sg,
            ClauseType::SubQuery(dst_var.to_string()),
        )?;

        let props = match m.remove("props") {
            None => HashMap::new(),
            Some(Value::Map(hm)) => hm,
            Some(_) => return Err(Error::TypeNotExpected),
        };

        transaction.rel_create_fragment::<GlobalCtx, RequestCtx>(
            Some(src_query),
            params,
            src_var,
            &dst_query,
            src_label,
            "dst_label",
            dst_var,
            rel_var,
            rel_name,
            props,
            props_type_name,
            clause,
            partition_key_opt,
            info,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_delete_input<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    src_var: &str,
    src_label: &str,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
    top_level_query: bool,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
         "visit_rel_delete_input called -- params: {:#?}, src_var: {}, src_label: {}, rel_name: {}, info.name: {}, input: {:#?}",
         params,
         src_var,
         src_label,
         rel_name,
         info.name(),
         input
     );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let _src_suffix = sg.suffix();
        let rel_suffix = sg.suffix();
        let dst_suffix = sg.suffix();
        let dst_var = "dst".to_string() + &dst_suffix;

        let (match_fragment, where_fragment, params) = visit_rel_query_input(
            params,
            src_label,
            src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            ClauseType::SubQuery(rel_name.to_string() + &rel_suffix),
            false,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // remove rather than get to take ownership
            transaction,
            sg,
        )?;

        let (match_query, params) = transaction.rel_read_query(
            &match_fragment,
            &where_fragment,
            params,
            src_label,
            src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            false,
            if top_level_query {
                ClauseType::FirstSubQuery(rel_name.to_string() + &rel_suffix)
            } else {
                ClauseType::SubQuery(rel_name.to_string() + &rel_suffix)
            },
            HashMap::new(),
            sg,
        )?;

        let (src_delete_query_opt, params) = if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            let (query, params) = visit_rel_src_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query.clone(),
                params,
                src_label,
                src_var,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                src,
                transaction,
                sg,
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
                &("dst".to_string() + &dst_suffix),
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                dst,
                transaction,
                sg,
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
            src_label,
            rel_name,
            &rel_suffix,
            partition_key_opt,
            sg,
            top_level_query,
            if top_level_query {
                ClauseType::Query
            } else {
                ClauseType::SubQuery(rel_name.to_string())
            },
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_rel_dst_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
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
            &k,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
            sg,
            false,
            ClauseType::SubQuery(node_var.to_string()),
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn visit_rel_dst_query_input<T>(
    params: HashMap<String, Value>,
    label: &str,
    var_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(Option<(String, String)>, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_rel_dst_query_input called -- label: {}, var_name: {}, info.name: {}, input: {:#?}",
        label,
        var_name,
        info.name(),
        input
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (match_fragment, where_fragment, params) = visit_node_query_input(
                params,
                label,
                var_name,
                false,
                true,
                ClauseType::Parameter(var_name.to_string()),
                // ClauseType::SubQuery(node_var.to_string()),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
                sg,
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
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_dst_update_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
    );

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        let dst_suffix = sg.suffix();
        let dst_var = "dst".to_string() + &dst_suffix;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            &dst_var,
            &dst_suffix,
            &k,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
            sg,
            ClauseType::SubQuery(dst_var.to_string()),
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_nodes_mutation_input_union<T, GlobalCtx, RequestCtx>(
    params: HashMap<String, Value>,
    node_var: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
    clause: ClauseType,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_nodes_mutation_input_union called -- info.name: {}, input: {:#?}",
        info.name(),
        input
    );

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
            node_var,
            &k,
            // ClauseType::SubQuery(node_var.to_string()),
            // ClauseType::None,
            clause,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_query_input<T>(
    params: HashMap<String, Value>,
    src_label: &str,
    src_var: &str,
    rel_name: &str,
    rel_suffix: &str,
    dst_var: &str,
    dst_suffix: &str,
    clause: ClauseType,
    top_level_query: bool,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input_opt: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_rel_query_input called -- src_label: {}, src_var: {}, rel_name: {}, dst_var: {}, dst_suffix: {}, clause: {:#?}, info.name: {}, input: {:#?}",
        src_label,
        src_var,
        rel_name,
        dst_var,
        dst_suffix,
        clause,
        info.name(),
        input_opt,
    );

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
                src_label,
                src_var,
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(src),
                transaction,
                sg,
            )?
        } else {
            (None, params)
        };

        // Remove used to take ownership
        let (dst_query_opt, params) = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                params,
                "", // dst_label,
                dst_var,
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(dst),
                transaction,
                sg,
            )?
        } else {
            (None, params)
        };

        transaction.rel_read_fragment(
            params,
            src_label,
            src_var,
            src_query_opt,
            rel_name,
            &rel_suffix,
            dst_var,
            dst_suffix,
            dst_query_opt,
            top_level_query,
            props,
            sg,
        )
    } else {
        transaction.rel_read_fragment(
            params,
            src_label,
            src_var,
            None,
            rel_name,
            &rel_suffix,
            dst_var,
            dst_suffix,
            None,
            false,
            HashMap::new(),
            sg,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    label: &str,
    node_var: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
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
            label,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
            sg,
            false,
            ClauseType::SubQuery(node_var.to_string()),
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_src_update_mutation_input called -- info.name: {}, label: {}, input: {:#?}",
        info.name(),
        label,
        input
    );

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        let src_suffix = sg.suffix();
        let src_var = "src".to_string() + &src_suffix;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
            &src_var,
            &src_suffix,
            label,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
            sg,
            ClauseType::SubQuery(src_var.to_string()),
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn visit_rel_src_query_input<T>(
    params: HashMap<String, Value>,
    label: &str,
    var_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(Option<(String, String)>, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_rel_src_query_input called -- label: {}, var_name: {}, info.name: {}, input: {:#?}",
        label,
        var_name,
        info.name(),
        input
    );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (match_fragment, where_fragment, params) = visit_node_query_input(
                params,
                label,
                &var_name,
                false,
                false,
                ClauseType::Parameter(var_name.to_string()),
                // ClauseType::SubQuery(node_var.to_string()),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
                sg,
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
    src_label: &str,
    src_var: &str,
    rel_name: &str,
    top_level_query: bool,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_input called -- src_label {}, src_var {}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_var,
         rel_name,
         info.name(),
         input
     );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let src_suffix = sg.suffix();
        let rel_suffix = sg.suffix();
        let dst_suffix = sg.suffix();
        let dst_var = "dst".to_string() + &dst_suffix;

        let (match_fragment, where_fragment, params) = visit_rel_query_input(
            params,
            src_label,
            &src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            ClauseType::Parameter("rel".to_string() + &rel_suffix),
            false,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // uses remove to take ownership
            transaction,
            sg,
        )?;

        let (match_query, params) = transaction.rel_read_query(
            &match_fragment,
            &where_fragment,
            params,
            &src_label,
            &src_var,
            rel_name,
            &rel_suffix,
            &dst_var,
            &dst_suffix,
            top_level_query,
            if top_level_query {
                ClauseType::FirstSubQuery(rel_name.to_string())
            } else {
                ClauseType::SubQuery(rel_name.to_string())
            },
            HashMap::new(),
            sg,
        )?;

        trace!(
            "visit_rel_update_input -- match_query: {}, params: {:#?}",
            match_query,
            params
        );

        /*
        let (query, params) = transaction.rel_read_query(
            params,
            src_label,
            &("src".to_string() + &src_suffix),
            None,
            rel_name,
            &rel_suffix,
            &("dst".to_string() + &dst_suffix),
            &dst_suffix,
            None,
            false,
            ClauseType::None,
            HashMap::new(),
            &mut sg,
        )?;
        */

        if let Some(update) = m.remove("update") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query,
                params,
                src_var,
                src_label,
                &src_suffix,
                rel_name,
                &rel_suffix,
                &dst_suffix,
                top_level_query,
                &Info::new(
                    itd.property("update")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                update,
                validators,
                props_type_name,
                transaction,
                sg,
                if top_level_query {
                    ClauseType::Query
                } else {
                    ClauseType::SubQuery("rel".to_string() + &rel_suffix)
                },
            )
        } else {
            Err(Error::InputItemNotFound {
                name: "update".to_string(),
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
    src_var: &str,
    src_label: &str,
    src_suffix: &str,
    rel_name: &str,
    rel_suffix: &str,
    dst_suffix: &str,
    top_level_query: bool,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
    clause: ClauseType,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_mutation_input called -- match_query: {}, params: {:#?}, src_var: {}, info.name: {}, src_label: {}, rel_name: {}, props_type_name: {:#?}, input: {:#?}",
         match_query,
         params,
         src_var,
         info.name(),
         src_label,
         rel_name,
         props_type_name,
         input
     );

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
            src_var,
            src_label,
            src_suffix,
            rel_name,
            rel_suffix,
            &("rel".to_string() + rel_suffix),
            dst_suffix,
            top_level_query,
            props,
            props_type_name,
            partition_key_opt,
            sg,
            clause,
        )?;

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query.clone(),
                params.clone(),
                src_label,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                src,
                validators,
                transaction,
                sg,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query,
                params,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                dst,
                validators,
                transaction,
                sg,
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
