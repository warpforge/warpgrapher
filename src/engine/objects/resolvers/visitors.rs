use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{ReturnClause, Transaction};
use crate::engine::objects::resolvers::SuffixGenerator;
use crate::engine::schema::{Info, PropertyKind};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use log::trace;
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_create_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    return_clause: ReturnClause,
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
        "visit_node_create_mutation_input called -- query: {}, params: {:#?}, label: {}, info.name: {}",
        query, params,
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
                                    let (fragment, params) = visit_rel_create_mutation_input::<
                                        T,
                                        GlobalCtx,
                                        RequestCtx,
                                    >(
                                        String::new(),
                                        params,
                                        label,
                                        &node_var,
                                        p.name(),
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        val,
                                        validators,
                                        None,
                                        ReturnClause::SubQuery("rel".to_string() + &sg.suffix()),
                                        transaction,
                                        sg,
                                    )?;
                                    rel_create_fragments.push(fragment);
                                    Ok((rel_create_fragments, params))
                                },
                            )
                        } else {
                            let (fragment, params) =
                                visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                    String::new(),
                                    params,
                                    label,
                                    &node_var,
                                    p.name(),
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    v,
                                    validators,
                                    None,
                                    ReturnClause::None,
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
            query,
            rel_create_fragments,
            params,
            &node_var,
            label,
            return_clause,
            partition_key_opt,
            props,
            info,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_delete_input<T, GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
    query: String,
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
        let (query, params) = visit_node_query_input(
            query,
            params,
            label,
            node_var,
            true,
            false,
            ReturnClause::SubQuery(node_var.to_string()),
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            sg,
        )?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
            query,
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
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_node_delete_mutation_input called -- query: {}, params: {:#?}, node_var: {}, label: {}, info.name: {}, input: {:#?}",
        query,
        params,
        node_var, label,
        info.name(),
        input
    );

    let itd = info.type_def()?;

    let (query, params) = if let Some(Value::Map(m)) = input {
        m.into_iter()
            .try_fold((query, params), |(query, params), (k, v)| {
                let p = itd.property(&k)?;

                match p.kind() {
                    PropertyKind::Input => {
                        if let Value::Array(input_array) = v {
                            input_array.into_iter().try_fold(
                                (query, params),
                                |(query, params), val| {
                                    visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                        query,
                                        params,
                                        label,
                                        &k,
                                        &Info::new(p.type_name().to_owned(), info.type_defs()),
                                        partition_key_opt,
                                        val,
                                        transaction,
                                        sg,
                                    )
                                },
                            )
                        } else {
                            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                query,
                                params,
                                label,
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                v,
                                transaction,
                                sg,
                            )
                        }
                    }
                    _ => Err(Error::TypeNotExpected),
                }
            })?
    } else {
        (query, params)
    };

    transaction.node_delete_query(
        query,
        params,
        node_var,
        label,
        partition_key_opt,
        sg,
    )
}

#[allow(clippy::too_many_arguments)]
fn visit_node_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    _return_clause: ReturnClause,
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
                query,
                params,
                node_var,
                label,
                ReturnClause::SubQuery(node_var.to_string()),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                v,
                validators,
                transaction,
                sg,
            ),
            "EXISTING" => {
                let _node_suffix = sg.suffix();
                visit_node_query_input(
                    query,
                    params,
                    label,
                    node_var,
                    true,
                    false,
                    ReturnClause::SubQuery(node_var.to_string()),
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    partition_key_opt,
                    Some(v),
                    transaction,
                    sg,
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
    query: String,
    params: HashMap<String, Value>,
    label: &str,
    node_var: &str,
    name_node: bool,
    union_type: bool,
    return_clause: ReturnClause,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_node_query_input called -- query: {}, params: {:#?}, label: {}, node_var: {}, union_type: {}, return_clause: {:#?}, info.name: {}, input: {:#?}",
        query,
        params,
        label,
        node_var,
        union_type,
        return_clause,
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
                                let (query, params) = visit_rel_query_input(
                                    String::new(),
                                    params,
                                    label,
                                    node_var,
                                    &k,
                                    &rel_suffix,
                                    &("dst".to_string() + &dst_suffix),
                                    &dst_suffix,
                                    ReturnClause::None,
                                    false,
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    Some(v),
                                    transaction,
                                    sg,
                                )?;
                                rqfs.push(query);
                                Ok((rqfs, params))
                            }
                            _ => Err(Error::TypeNotExpected),
                        })
                })?;

        transaction.node_read_query(
            query,
            rqfs,
            params,
            label,
            node_var,
            name_node,
            union_type,
            return_clause,
            &param_suffix,
            props,
        )
    } else {
        transaction.node_read_query(
            query,
            Vec::new(),
            params,
            label,
            node_var,
            name_node,
            union_type,
            return_clause,
            &param_suffix,
            props,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_node_update_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    node_var: &str,
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
    _sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_input called -- query: {}, params: {:#?}, label: {}, info.name: {}, input: {:#?}",
        query,
        params,
        label,
        info.name(),
        input,
    );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let mut sg = SuffixGenerator::new();
        let node_suffix = sg.suffix();

        let (match_query, params) = visit_node_query_input(
            query,
            params,
            label,
            &node_var,
            true,
            false,
            ReturnClause::SubQuery(node_var.to_string()),
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            &mut sg,
        )?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            match_query,
            params,
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
            &mut sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node_update_mutation_input<T, GlobalCtx, RequestCtx>(
    match_query: String,
    params: HashMap<String, Value>,
    node_suffix: &str,
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
                                                    String::new(),
                                                    params,
                                                    label,
                                                    Vec::new(),
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
                                        String::new(),
                                        params,
                                        label,
                                        Vec::new(),
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
            &("node".to_string() + node_suffix),
            props,
            partition_key_opt,
            info,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    src_label: &str,
    src_ids: Vec<Value>,
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
         "visit_rel_change_input called -- src_label {}, src_ids {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids,
         rel_name,
         info.name(),
         input
     );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                query,
                params,
                src_label,
                "",
                rel_name,
                &Info::new(
                    itd.property("ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                validators,
                None,
                ReturnClause::None,
                transaction,
                sg,
            )
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                query,
                params,
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
            )
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
                query,
                params,
                src_label,
                Some(src_ids),
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
pub(super) fn visit_rel_create_input<T, GlobalCtx, RequestCtx>(
    query: String,
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
        "visit_rel_create_input called -- query: {}, params: {:#?}, info.name: {}, rel_name {}, input: {:#?}",
        query,
        params,
        info.name(),
        rel_name,
        input
    );

    let src_var = "src".to_string() + &sg.suffix();

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let _src_suffix = sg.suffix();

        let (match_query, params) = visit_node_query_input(
            query,
            params,
            src_label,
            &src_var,
            true,
            false,
            ReturnClause::None,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
            sg,
        )?;
        let create_input = m.remove("create").ok_or_else(|| {
            // Using remove to take ownership
            Error::InputItemNotFound {
                name: "input::create".to_string(),
            }
        })?;

        trace!(
            "visit_rel_create_input -- match_query: {}, params: {:#?}",
            match_query,
            params
        );

        match create_input {
            Value::Map(_) => visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                match_query,
                params,
                src_label,
                &src_var,
                rel_name,
                &Info::new(
                    itd.property("create")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                create_input,
                validators,
                props_type_name,
                ReturnClause::Query(rel_name.to_string()),
                transaction,
                sg,
            ),
            Value::Array(create_input_array) => create_input_array.into_iter().try_fold(
                (match_query, params),
                |(query, params), create_input_value| {
                    visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                        query,
                        params,
                        src_label,
                        &src_var,
                        rel_name,
                        &Info::new(
                            itd.property("create")?.type_name().to_owned(),
                            info.type_defs(),
                        ),
                        partition_key_opt,
                        create_input_value,
                        validators,
                        props_type_name,
                        ReturnClause::Query(rel_name.to_string()),
                        transaction,
                        sg,
                    )
                },
            ),
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
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    return_clause: ReturnClause,
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
        let dst_var = "dst".to_string() + &sg.suffix();
        let dst_prop = info.type_def()?.property("dst")?;
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::InputItemNotFound {
                name: "dst".to_string(),
            })?;
        let (dst_query, params) = visit_rel_nodes_mutation_input_union::<T, GlobalCtx, RequestCtx>(
            String::new(), //".V()".to_string(),
            params,
            &dst_var,
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            dst,
            validators,
            transaction,
            sg,
        )?;

        let props = match m.remove("props") {
            None => HashMap::new(),
            Some(Value::Map(hm)) => hm,
            Some(_) => return Err(Error::TypeNotExpected),
        };

        transaction.rel_create_query::<GlobalCtx, RequestCtx>(
            src_query,
            params,
            src_var,
            &dst_query,
            src_label,
            "dst_label",
            &dst_var,
            rel_name,
            props,
            props_type_name,
            return_clause,
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
    query: String,
    params: HashMap<String, Value>,
    src_label: &str,
    rel_name: &str,
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
         "visit_rel_delete_input called -- query: {}, params: {:#?}, src_label {}, rel_name {}, info.name: {}, input: {:#?}",
         query,
         params,
         src_label,
         rel_name,
         info.name(),
         input
     );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let src_suffix = sg.suffix();
        let rel_suffix = sg.suffix();
        let dst_suffix = sg.suffix();

        let (read_query, params) = visit_rel_query_input(
            query.clone(),
            params,
            src_label,
            &("src".to_string() + &src_suffix),
            rel_name,
            &rel_suffix,
            &("dst".to_string() + &dst_suffix),
            &dst_suffix,
            ReturnClause::SubQuery(rel_name.to_string() + &rel_suffix),
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

        /*
        let (query, params) = transaction.rel_read_query(
            query,
            params,
            src_label,
            &("src".to_string() + &src_suffix),
            Some(read_query),
            rel_name,
            &rel_suffix,
            &("dst".to_string() + &dst_suffix),
            &dst_suffix,
            None,
            true,
            HashMap::new(),
            sg,
        )?;
        */
        /*
        let rel_ids = read_results
            .iter()
            .map(|r: &Rel<GlobalCtx, RequestCtx>| r.id().clone())
            .collect();
            */

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                query.clone(),
                params.clone(),
                src_label,
                &("src".to_string() + &src_suffix),
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                src,
                transaction,
                sg,
            )?;
        };

        if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            visit_rel_dst_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                query,
                params.clone(),
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
        };

        transaction.rel_delete_query(
            read_query,
            params,
            src_label,
            rel_name,
            &rel_suffix,
            partition_key_opt,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
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
            query,
            params,
            node_var,
            &k,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_query_input<T>(
    query: String,
    params: HashMap<String, Value>,
    label: &str,
    var_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(Option<String>, HashMap<String, Value>), Error>
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

            let (query, params) = visit_node_query_input(
                query,
                params,
                label,
                var_name,
                false,
                true,
                ReturnClause::None,
                // ReturnClause::SubQuery(node_var.to_string()),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
                sg,
            )?;

            Ok((Some(query), params))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_update_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
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

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            query,
            params,
            &sg.suffix(),
            &k,
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
fn visit_rel_nodes_mutation_input_union<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    node_var: &str,
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
            query,
            params,
            node_var,
            &k,
            ReturnClause::SubQuery(node_var.to_string()),
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
    query: String,
    params: HashMap<String, Value>,
    src_label: &str,
    src_var: &str,
    rel_name: &str,
    rel_suffix: &str,
    dst_var: &str,
    dst_suffix: &str,
    return_clause: ReturnClause,
    top_level_query: bool,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input_opt: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
        "visit_rel_query_input called -- query: {}, src_label: {}, src_var: {}, rel_name: {}, dst_var: {}, dst_suffix: {}, return_clause: {:#?}, info.name: {}, input: {:#?}",
        query, 
        src_label,
        src_var,
        rel_name,
        dst_var,
        dst_suffix,
        return_clause,
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
                "".to_string(),
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
                "".to_string(),
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

        transaction.rel_read_query(
            query,
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
            return_clause,
            props,
            sg,
        )
    } else {
        transaction.rel_read_query(
            query,
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
            return_clause,
            HashMap::new(),
            sg,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
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
            query,
            params,
            node_var,
            label,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
            sg,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_update_mutation_input<T, GlobalCtx, RequestCtx>(
    query: String,
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

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            query,
            params,
            &sg.suffix(),
            label,
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
fn visit_rel_src_query_input<T>(
    query: String,
    params: HashMap<String, Value>,
    label: &str,
    var_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
    sg: &mut SuffixGenerator,
) -> Result<(Option<String>, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
         "visit_rel_src_query_input called -- query: {}, label: {}, var_name: {}, info.name: {}, input: {:#?}",
         query,
         label,
         var_name,
         info.name(),
         input
     );

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (query, params) = visit_node_query_input(
                query,
                params,
                label,
                &var_name,
                false,
                false,
                ReturnClause::None,
                // ReturnClause::SubQuery(node_var.to_string()),
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
                sg,
            )?;

            Ok((Some(query), params))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_update_input<T, GlobalCtx, RequestCtx>(
    query: String,
    params: HashMap<String, Value>,
    src_label: &str,
    src_ids: Option<Vec<Value>>,
    rel_name: &str,
    top_level_query: bool,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
    _sg: &mut SuffixGenerator,
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_input called -- src_label {}, src_ids {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids,
         rel_name,
         info.name(),
         input
     );

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let mut sg = SuffixGenerator::new();
        let src_suffix = sg.suffix();
        let rel_suffix = sg.suffix();
        let dst_suffix = sg.suffix();

        let (match_query, params) = visit_rel_query_input(
            query,
            params,
            src_label,
            &("src".to_string() + &src_suffix),
            rel_name,
            &rel_suffix,
            &("dst".to_string() + &dst_suffix),
            &dst_suffix,
            ReturnClause::None,
            false,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // uses remove to take ownership
            transaction,
            &mut sg,
        )?;

        let (query, params) = transaction.rel_read_query(
            match_query,
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
            ReturnClause::None,
            HashMap::new(),
            &mut sg,
        )?;

        if let Some(update) = m.remove("update") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                query,
                params,
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
                &mut sg,
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
    query: String,
    params: HashMap<String, Value>,
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
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_mutation_input called -- query: {}, params: {:#?}, info.name: {}, src_label: {}, rel_name: {}, props_type_name: {:#?}, input: {:#?}",
         query,
         params,
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
            query.clone(),
            params.clone(),
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
        )?;

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                query.clone(),
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
                query,
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
