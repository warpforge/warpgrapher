use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{QueryResult, Transaction};
use crate::engine::objects::Node;
use crate::engine::schema::{Info, PropertyKind};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use juniper::{graphql_value, FieldError};
use log::{debug, trace};
use std::collections::HashMap;

/// Genererates unique suffixes for the variable names used in Cypher queries
#[derive(Default)]
pub(super) struct SuffixGenerator {
    seed: i32,
}

impl SuffixGenerator {
    pub(super) fn new() -> SuffixGenerator {
        SuffixGenerator { seed: -1 }
    }

    pub(super) fn suffix(&mut self) -> String {
        self.seed += 1;
        String::from("_") + &self.seed.to_string()
    }
}

pub(super) fn visit_node_create_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Node<GlobalCtx, RequestCtx>, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_create_mutation_input called -- label: {}, info.name: {}",
        label,
        info.name(),
    );

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        for k in m.keys() {
            let p = itd.property(k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator()
                        .clone()
                        .map_or(Ok(()), |v_name| validate_input(validators, &v_name, &input))?;
                }
                _ => {} // No validation action to take
            }
        }
    }

    if let Value::Map(m) = input {
        let mut props: HashMap<String, Value> = HashMap::new();
        let mut inputs: HashMap<String, Value> = HashMap::new();
        for (k, v) in m.into_iter() {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    props.insert(k, v);
                }
                PropertyKind::Input => {
                    inputs.insert(k, v);
                }
                _ => return Err(Error::TypeNotExpected.into()),
            }
        }

        let results = transaction.create_node(label, partition_key_opt, props, info)?;
        let ids = Value::Array(vec![results.fields.get("id").unwrap().clone()]);

        for (k, v) in inputs.into_iter() {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Handled earlier
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            let _ = visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                label,
                                ids.clone(),
                                p.name(),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                validators,
                                None,
                                transaction,
                            )?;
                        }
                    } else {
                        let _ = visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                            label,
                            ids.clone(),
                            p.name(),
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            validators,
                            None,
                            transaction,
                        )?;
                    }
                }
                _ => return Err(Error::TypeNotExpected.into()),
            }
        }

        Ok(results)
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

pub(super) fn visit_node_delete_input<T>(
    label: &str,
    var_suffix: &str,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_input called -- info.name: {}, label: {}, input: {:#?}",
        info.name(),
        label,
        input
    );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        let (query, params) = visit_node_query_input(
            label,
            var_suffix,
            false,
            true,
            HashMap::new(),
            sg,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!(
            "visit_node_delete_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, None, partition_key_opt, Some(params));
        debug!("visit_node_delete_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.ids(&(String::from(label) + var_suffix))?;
        trace!("visit_node_delete_input IDs for deletion: {:#?}", ids);

        visit_node_delete_mutation_input(
            label,
            ids,
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
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn visit_node_delete_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<i32, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
    );

    let itd = info.type_def()?;

    if let Some(Value::Map(m)) = input {
        for (k, v) in m.into_iter() {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_delete_input(
                                label,
                                Some(ids.clone()),
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                transaction,
                            )?;
                        }
                    } else {
                        visit_rel_delete_input(
                            label,
                            Some(ids.clone()),
                            &k,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            transaction,
                        )?;
                    }
                }
                _ => {
                    return Err(Error::TypeNotExpected.into());
                }
            }
        }
    }

    transaction.delete_nodes(label, ids, partition_key_opt)
}

fn visit_node_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Value, FieldError>
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

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string() + "::NEW or ::EXISTING",
            })?;

        let p = itd.property(&k)?;

        match k.as_ref() {
            "NEW" => Ok(Value::Array(vec![visit_node_create_mutation_input::<
                T,
                GlobalCtx,
                RequestCtx,
            >(
                label,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                v,
                validators,
                transaction,
            )?
            .fields
            .remove("id")
            .ok_or_else(|| Error::ResponseItemNotFound {
                name: "id".to_string(),
            })?])),
            "EXISTING" => {
                let mut sg = SuffixGenerator::new();
                let var_suffix = sg.suffix();
                let (query, params) = visit_node_query_input(
                    label,
                    &var_suffix,
                    false,
                    true,
                    HashMap::new(),
                    &mut sg,
                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                    partition_key_opt,
                    Some(v),
                    transaction,
                )?;

                debug!(
                    "visit_node_input query, params: {:#?}, {:#?}",
                    query, params
                );
                let results = transaction.exec(&query, None, partition_key_opt, Some(params))?;
                debug!("visit_node_input Query results: {:#?}", results);
                results.ids(&(label.to_owned() + &var_suffix))
            }

            _ => Err(Error::SchemaItemNotFound {
                name: info.name().to_string() + "::" + &k,
            }
            .into()),
        }
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub(super) fn visit_node_query_input<T>(
    label: &str,
    suffix: &str,
    union_type: bool,
    return_node: bool,
    params: HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), FieldError>
where
    T: Transaction,
{
    trace!(
             "visit_node_query_input called -- label: {}, var_suffix: {}, union_type: {}, return_node: {}, info.name: {}, input: {:#?}",
             label, suffix, union_type, return_node, info.name(), input,
         );
    // let mut qs = String::from(query);
    let itd = info.type_def()?;
    let param_suffix = sg.suffix();

    let mut props = HashMap::new();
    if let Some(Value::Map(m)) = input {
        let (rel_query_fragments, local_params) =
            m.into_iter()
                .try_fold((Vec::new(), params), |(mut rqf, params), (k, v)| {
                    itd.property(&k).map_err(|e| e.into()).and_then(|p| {
                        match p.kind() {
                            PropertyKind::Scalar => {
                                props.insert(k, v);
                                Ok((rqf, params))
                            }
                            PropertyKind::Input => {
                                visit_rel_query_input(
                                    label,
                                    suffix,
                                    None,
                                    &k,
                                    "dst",
                                    &sg.suffix(),
                                    false,
                                    // &qs,
                                    params,
                                    sg,
                                    &Info::new(p.type_name().to_owned(), info.type_defs()),
                                    partition_key_opt,
                                    Some(v),
                                    transaction,
                                )
                                .map(|(qs, params)| {
                                    rqf.push(qs);
                                    (rqf, params)
                                })
                            }
                            _ => Err(FieldError::from(Error::TypeNotExpected)),
                        }
                    })
                })?;

        transaction.node_query(
            rel_query_fragments,
            local_params,
            label,
            suffix,
            union_type,
            return_node,
            &param_suffix,
            props,
        )
    } else {
        transaction.node_query(
            Vec::new(),
            params,
            label,
            suffix,
            union_type,
            return_node,
            &param_suffix,
            props,
        )
    }
}

pub(super) fn visit_node_update_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_input called -- info.name: {}, label {}, input: {:#?}",
        info.name(),
        label,
        input
    );

    let mut sg = SuffixGenerator::new();
    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        let var_suffix = sg.suffix();

        let (query, params) = visit_node_query_input(
            label,
            &var_suffix,
            false,
            true,
            HashMap::new(),
            &mut sg,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!(
            "visit_node_update_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, None, partition_key_opt, Some(params));
        debug!("visit_node_update_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.ids(&(String::from(label) + &var_suffix))?;
        trace!("visit_node_update_input IDs for update: {:#?}", ids);

        visit_node_update_mutation_input(
            label,
            ids,
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
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn visit_node_update_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_node_update_mutation_input called -- label: {}, ids: {:#?}, info.name: {}, input: {:#?}",
        label,
        ids,
        info.name(),
        input
    );

    let itd = info.type_def()?;

    if let Value::Map(ref m) = input {
        for k in m.keys() {
            let p = itd.property(k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator()
                        .clone()
                        .map_or(Ok(()), |v_name| validate_input(validators, &v_name, &input))?;
                }
                _ => {} // No validation action to take
            }
        }
    }

    if let Value::Map(m) = input {
        let mut props = HashMap::new();
        let mut inputs = HashMap::new();
        for (k, v) in m.into_iter() {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    props.insert(k.to_owned(), v);
                }
                PropertyKind::Input => {
                    inputs.insert(k, v);
                }
                _ => return Err(Error::TypeNotExpected.into()),
            }
        }

        let results = transaction.update_nodes(label, ids, props, partition_key_opt, info)?;

        for (k, v) in inputs.into_iter() {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Properties handled above
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                                label,
                                Value::Array(
                                    results
                                        .clone()
                                        .into_iter()
                                        .map(|n| n.fields.get("id").unwrap().clone())
                                        .collect(),
                                ),
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                validators,
                                transaction,
                            )?;
                        }
                    } else {
                        visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                            label,
                            Value::Array(
                                results
                                    .clone()
                                    .into_iter()
                                    .map(|n| n.fields.get("id").unwrap().clone())
                                    .collect(),
                            ),
                            &k,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            validators,
                            transaction,
                        )?;
                    }
                }
                _ => {}
            }
        }

        Ok(results)
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<(), FieldError>
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
                src_label,
                src_ids,
                rel_name,
                &Info::new(
                    itd.property("ADD")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                validators,
                None,
                transaction,
            )?;
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.property("DELETE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                transaction,
            )?;
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.property("UPDATE")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                v,
                validators,
                None,
                transaction,
            )?;
        } else {
            return Err(Error::InputItemNotFound {
                name: itd.type_name().to_string() + "::ADD|DELETE|UPDATE",
            }
            .into());
        }
    } else {
        return Err(Error::TypeNotExpected.into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_create_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    rel_name: &str,
    props_type_name: Option<&str>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_create_input called -- info.name: {}, rel_name {}, input: {:#?}",
        info.name(),
        rel_name,
        input
    );

    let mut sg = SuffixGenerator::new();
    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        let var_suffix = sg.suffix();

        let (query, params) = visit_node_query_input(
            src_label,
            &var_suffix,
            false,
            true,
            HashMap::new(),
            &mut sg,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!("Query, params: {:#?}, {:#?}", query, params);
        let raw_results =
            transaction.exec(&query, props_type_name, partition_key_opt, Some(params));
        debug!("Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.ids(&(String::from(src_label) + &var_suffix))?;
        trace!("IDs for update: {:#?}", ids);

        let create_input = m.remove("create").ok_or_else(|| {
            // Using remove to take ownership
            Error::InputItemNotFound {
                name: "input::create".to_string(),
            }
        })?;

        trace!("visit_rel_create_input calling rels.");
        match create_input {
            Value::Map(_) => Ok(visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                ids,
                rel_name,
                &Info::new(
                    itd.property("create")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                create_input,
                validators,
                props_type_name,
                transaction,
            )?),
            Value::Array(create_input_array) => {
                let mut result = None;
                for create_input_value in create_input_array {
                    match result.as_mut() {
                        None => {
                            result =
                                Some(visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                    src_label,
                                    ids.clone(),
                                    rel_name,
                                    &Info::new(
                                        itd.property("create")?.type_name().to_owned(),
                                        info.type_defs(),
                                    ),
                                    partition_key_opt,
                                    create_input_value,
                                    validators,
                                    props_type_name,
                                    transaction,
                                )?)
                        }
                        Some(r) => {
                            r.merge(visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                src_label,
                                ids.clone(),
                                rel_name,
                                &Info::new(
                                    itd.property("create")?.type_name().to_owned(),
                                    info.type_defs(),
                                ),
                                partition_key_opt,
                                create_input_value,
                                validators,
                                props_type_name,
                                transaction,
                            )?)
                        }
                    }
                }
                Ok(result.unwrap())
            }
            _ => Err(Error::TypeNotExpected.into()),
        }
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_create_mutation_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
            "visit_rel_create_mutation_input called -- src_label: {}, src_ids: {:#?}, rel_name: {}, info.name: {}, input: {:#?}",
            src_label,
            src_ids,
            rel_name,
            info.name(),
            input
        );

    let itd = info.type_def()?;
    let dst_prop = itd.property("dst")?;

    if let Value::Map(mut m) = input {
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::InputItemNotFound {
                name: "dst".to_string(),
            })?;
        let (dst_label, dst_ids) = visit_rel_nodes_mutation_input_union::<T, GlobalCtx, RequestCtx>(
            &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            dst,
            validators,
            transaction,
        )?;

        let result = transaction.create_rels::<GlobalCtx, RequestCtx>(
            src_label,
            src_ids,
            &dst_label,
            dst_ids,
            rel_name,
            &mut m,
            partition_key_opt,
            props_type_name,
            info,
        );

        trace!("visit_rel_create_mutation_input -- result: {:#?}", result);

        result
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

pub(super) fn visit_rel_delete_input<T>(
    src_label: &str,
    src_ids_opt: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_delete_input called -- src_label {}, src_ids_opt {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids_opt,
         rel_name,
         info.name(),
         input
     );

    let mut sg = SuffixGenerator::new();
    let itd = info.type_def()?;
    let src_suffix = sg.suffix();
    let dst_suffix = sg.suffix();

    if let Value::Map(mut m) = input {
        let (read_query, params) = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids_opt,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            // "",
            HashMap::new(),
            &mut sg,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // remove rather than get to take ownership
            transaction,
        )?;

        let read_results = transaction.exec(&read_query, None, partition_key_opt, Some(params))?;
        let rel_ids = read_results.ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?;

        let del_results =
            transaction.delete_rels(src_label, rel_name, rel_ids, partition_key_opt, info)?;

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input(
                src_label,
                read_results.ids(&(src_label.to_string() + &src_suffix))?,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                src,
                transaction,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            visit_rel_dst_delete_mutation_input(
                read_results.ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                dst,
                transaction,
            )?;
        }

        Ok(del_results)
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn visit_rel_dst_delete_mutation_input<T>(
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_dst_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
    );

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = itd.property(&k)?;

        visit_node_delete_mutation_input(
            &k,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_query_input<T>(
    label: &str,
    var_suffix: &str,
    // query: &str,
    params: HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<(Option<String>, HashMap<String, Value>), FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_dst_query_input called -- label: {}, var_suffix: {}, info.name: {}, input: {:#?}",
         label,
         var_suffix,
         info.name(),
         input
     );

    let itd = info.type_def()?;

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = itd.property(&k)?;

            let (t1, t2) = visit_node_query_input(
                label,
                var_suffix,
                true,
                false,
                // query,
                params,
                sg,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
            )?;

            Ok((Some(t1), t2))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

fn visit_rel_dst_update_mutation_input<T, GlobalCtx, RequestCtx>(
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
        "visit_rel_dst_update_mutation_input called -- info.name: {}, ids: {:#?}, input: {:#?}",
        info.name(),
        ids,
        input
    );

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = itd.property(&k)?;

        visit_node_update_mutation_input(
            &k,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn visit_rel_nodes_mutation_input_union<T, GlobalCtx, RequestCtx>(
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<(String, Value), FieldError>
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

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = itd.property(&k)?;

        let dst_ids = visit_node_input::<T, GlobalCtx, RequestCtx>(
            &k,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )?;

        Ok((k.to_owned(), dst_ids))
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::implicit_hasher, clippy::too_many_arguments)]
pub(super) fn visit_rel_query_input<T>(
    src_label: &str,
    src_suffix: &str,
    src_ids_opt: Option<Value>,
    rel_name: &str,
    dst_var: &str,
    dst_suffix: &str,
    return_rel: bool,
    params: HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input_opt: Option<Value>,
    transaction: &mut T,
) -> Result<(String, HashMap<String, Value>), FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_query_input called -- src_label: {}, src_suffix: {}, src_ids_opt: {:#?}, rel_name: {}, dst_var: {}, dst_suffix: {}, return_rel: {:#?}, info.name: {}, input: {:#?}",
        src_label,
        src_suffix,
        src_ids_opt,
        rel_name,
        dst_var,
        dst_suffix,
        return_rel,
        info.name(),
        input_opt,
    );

    let itd = info.type_def()?;
    let src_prop = itd.property("src")?;
    let dst_prop = itd.property("dst")?;

    let mut props = HashMap::new();
    let local_params = params;
    if let Some(Value::Map(mut m)) = input_opt {
        // uses remove in order to take ownership
        if let Some(id) = m.remove("id") {
            props.insert("id".to_owned(), id);
        }

        // uses remove to take ownership
        if let Some(Value::Map(rel_props)) = m.remove("props") {
            for (k, v) in (rel_props).into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        // Remove used to take ownership
        let (src_query_opt, local_params) = if let Some(src) = m.remove("src") {
            visit_rel_src_query_input(
                src_label,
                src_suffix,
                // &query,
                local_params,
                sg,
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(src),
                transaction,
            )?
        } else {
            (None, local_params)
        };

        // Remove used to take ownership
        let (dst_query_opt, local_params) = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                dst_var,
                dst_suffix,
                // &query,
                local_params,
                sg,
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(dst),
                transaction,
            )?
        } else {
            (None, local_params)
        };

        transaction.rel_query_string(
            // query,
            src_label,
            src_suffix,
            src_ids_opt,
            src_query_opt,
            rel_name,
            dst_var,
            dst_suffix,
            dst_query_opt,
            return_rel,
            props,
            local_params,
        )
    } else {
        transaction.rel_query_string(
            // query,
            src_label,
            src_suffix,
            src_ids_opt,
            None,
            rel_name,
            dst_var,
            dst_suffix,
            None,
            return_rel,
            props,
            local_params,
        )
    }
}

fn visit_rel_src_delete_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- info.name: {}, ids: {:#?}, input: {:#?}",
        info.name(),
        ids,
        input
    );

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = itd.property(&k)?;

        visit_node_delete_mutation_input(
            label,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn visit_rel_src_update_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_src_update_mutation_input called -- info.name: {}, label: {}, ids: {:#?}, input: {:#?}",
         info.name(),
         label,
         ids,
         input
     );

    let itd = info.type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = itd.property(&k)?;

        visit_node_update_mutation_input(
            label,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_query_input<T>(
    label: &str,
    label_suffix: &str,
    params: HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<(Option<String>, HashMap<String, Value>), FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_src_query_input called -- label: {}, label_suffix: {}, info.name: {}, input: {:#?}",
         label,
         label_suffix,
         info.name(),
         input
     );

    let itd = info.type_def()?;

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = itd.property(&k)?;

            let (t1, t2) = visit_node_query_input(
                label,
                label_suffix,
                false,
                false,
                // query,
                params,
                sg,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
            )?;

            Ok((Some(t1), t2))
        } else {
            Ok((None, params))
        }
    } else {
        Ok((None, params))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_update_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
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

    let mut sg = SuffixGenerator::new();
    let itd = info.type_def()?;
    let src_suffix = sg.suffix();
    let dst_suffix = sg.suffix();

    if let Value::Map(mut m) = input {
        let (read_query, params) = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            HashMap::new(),
            &mut sg,
            &Info::new(
                itd.property("match")?.type_name().to_owned(),
                info.type_defs(),
            ),
            partition_key_opt,
            m.remove("match"), // uses remove to take ownership
            transaction,
        )?;

        debug!(
            "visit_rel_update_input query, params: {:#?}, {:#?}",
            read_query, params
        );
        let raw_read_results = transaction.exec(
            &read_query,
            props_type_name,
            partition_key_opt,
            Some(params),
        );
        debug!("visit_rel_update_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        if let Some(update) = m.remove("update") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                read_results.ids(&(String::from(src_label) + &src_suffix))?,
                rel_name,
                read_results.ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
                read_results.ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.property("update")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                update,
                validators,
                props_type_name,
                transaction,
            )
        } else {
            Err(Error::InputItemNotFound {
                name: "update".to_string(),
            }
            .into())
        }
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_update_mutation_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    rel_ids: Value,
    dst_ids: Value,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    trace!(
         "visit_rel_update_mutation_input called -- info.name: {}, src_label: {}, src_ids: {:#?}, rel_name: {}, rel_ids: {:#?}, dst_ids: {:#?}, props_type_name: {:#?}, input: {:#?}",
         info.name(),
         src_label,
         src_ids,
         rel_name,
         rel_ids,
         dst_ids,
         props_type_name,
         input
     );

    let itd = info.type_def()?;

    if let Value::Map(mut m) = input {
        let mut props = HashMap::new();
        if let Some(Value::Map(pm)) = m.remove("props") {
            // remove used to take ownership
            for (k, v) in pm.into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        let results = transaction.update_rels::<GlobalCtx, RequestCtx>(
            src_label,
            rel_name,
            rel_ids,
            partition_key_opt,
            props,
            props_type_name,
            info,
        )?;

        trace!("visit_rel_update_mutation_input results: {:#?}", results);

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                src_ids,
                &Info::new(
                    itd.property("src")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                src,
                validators,
                transaction,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                dst_ids,
                &Info::new(
                    itd.property("dst")?.type_name().to_owned(),
                    info.type_defs(),
                ),
                partition_key_opt,
                dst,
                validators,
                transaction,
            )?;
        }

        Ok(results)
    } else {
        Err(Error::TypeNotExpected.into())
    }
}

fn validate_input(validators: &Validators, v: &str, input: &Value) -> Result<(), FieldError> {
    let func = validators.get(v).ok_or_else(|| Error::ValidatorNotFound {
        name: v.to_string(),
    })?;

    trace!(
        "validate_input Calling input validator function {} for input value {:#?}",
        v,
        input
    );

    func(input).or_else(|e| match e {
        Error::ValidationFailed { message } => Err(FieldError::new(
            message,
            juniper::graphql_value!({ "internal_error": "Input validation failed" }),
        )),
        _ => Err(FieldError::from(e)),
    })
}
