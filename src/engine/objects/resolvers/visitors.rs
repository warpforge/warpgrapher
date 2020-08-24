use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::Transaction;
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::{Info, PropertyKind};
use crate::engine::validators::Validators;
use crate::engine::value::Value;
use crate::error::Error;
use log::trace;
use std::collections::HashMap;

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
        "_".to_string() + &self.seed.to_string()
    }
}

pub(super) fn visit_node_create_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Node<GlobalCtx, RequestCtx>, Error>
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

        let node = transaction.create_node::<GlobalCtx, RequestCtx>(
            label,
            partition_key_opt,
            props,
            info,
        )?;
        let ids = vec![node.id()?.clone()];

        inputs.into_iter().try_for_each(|(k, v)| {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => Ok(()), // Handled earlier
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        input_array.into_iter().try_for_each(|val| {
                            visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                                label,
                                ids.clone(),
                                p.name(),
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                validators,
                                None,
                                transaction,
                            )
                            .map(|_| ())
                        })
                    } else {
                        visit_rel_create_mutation_input::<T, GlobalCtx, RequestCtx>(
                            label,
                            ids.clone(),
                            p.name(),
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            validators,
                            None,
                            transaction,
                        )
                        .map(|_| ())
                    }
                }
                _ => Err(Error::TypeNotExpected),
            }
        })?;

        Ok(node)
    } else {
        Err(Error::TypeNotExpected)
    }
}

pub(super) fn visit_node_delete_input<T, GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
    label: &str,
    var_suffix: &str,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, Error>
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

        let results = transaction.read_nodes(&query, partition_key_opt, Some(params), info)?;
        let ids = results
            .iter()
            .map(|n: &Node<GlobalCtx, RequestCtx>| Ok(n.id()?.clone()))
            .collect::<Result<Vec<Value>, Error>>()?;

        visit_node_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
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
        Err(Error::TypeNotExpected)
    }
}

fn visit_node_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<i32, Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_node_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name(),
        input
    );

    let itd = info.type_def()?;

    if let Some(Value::Map(m)) = input {
        m.into_iter().try_for_each(|(k, v)| {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        input_array.into_iter().try_for_each(|val| {
                            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                                label,
                                Some(ids.clone()),
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                transaction,
                            )
                            .map(|_| ())
                        })
                    } else {
                        visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
                            label,
                            Some(ids.clone()),
                            &k,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            transaction,
                        )
                        .map(|_| ())
                    }
                }
                _ => Err(Error::TypeNotExpected),
            }
        })?;
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
) -> Result<Vec<Value>, Error>
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
            "NEW" => Ok(vec![visit_node_create_mutation_input::<
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
            .id()?
            .clone()]),
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

                let results =
                    transaction.read_nodes(&query, partition_key_opt, Some(params), info)?;
                results
                    .iter()
                    .map(|n: &Node<GlobalCtx, RequestCtx>| Ok(n.id()?.clone()))
                    .collect::<Result<Vec<Value>, Error>>()
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
) -> Result<(String, HashMap<String, Value>), Error>
where
    T: Transaction,
{
    trace!(
             "visit_node_query_input called -- label: {}, var_suffix: {}, union_type: {}, return_node: {}, info.name: {}, input: {:#?}",
             label, suffix, union_type, return_node, info.name(), input,
         );
    let itd = info.type_def()?;
    let param_suffix = sg.suffix();

    let mut props = HashMap::new();
    if let Some(Value::Map(m)) = input {
        let (rel_query_fragments, local_params) =
            m.into_iter()
                .try_fold((Vec::new(), params), |(mut rqf, params), (k, v)| {
                    itd.property(&k).map_err(|e| e).and_then(|p| {
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
                            _ => Err(Error::TypeNotExpected),
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
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let mut sg = SuffixGenerator::new();
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

        let results = transaction.read_nodes(&query, partition_key_opt, Some(params), info)?;
        let ids = results
            .iter()
            .map(|n: &Node<GlobalCtx, RequestCtx>| Ok(n.id()?.clone()))
            .collect::<Result<Vec<Value>, Error>>()?;
        trace!("visit_node_update_input IDs for update: {:#?}", ids);

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
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
        Err(Error::TypeNotExpected)
    }
}

fn visit_node_update_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
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

        let results = transaction.update_nodes::<GlobalCtx, RequestCtx>(
            label,
            ids,
            props,
            partition_key_opt,
            info,
        )?;

        inputs.into_iter().try_for_each(|(k, v)| {
            let p = itd.property(&k)?;

            match p.kind() {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => Ok(()), // Properties handled above
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        input_array.into_iter().try_for_each(|val| {
                            visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                                label,
                                results
                                    .iter()
                                    .map(|n| Ok(n.id()?.clone()))
                                    .collect::<Result<Vec<Value>, Error>>()?,
                                &k,
                                &Info::new(p.type_name().to_owned(), info.type_defs()),
                                partition_key_opt,
                                val,
                                validators,
                                transaction,
                            )
                            .map(|_| ())
                        })
                    } else {
                        visit_rel_change_input::<T, GlobalCtx, RequestCtx>(
                            label,
                            results
                                .iter()
                                .map(|n| Ok(n.id()?.clone()))
                                .collect::<Result<Vec<Value>, Error>>()?,
                            &k,
                            &Info::new(p.type_name().to_owned(), info.type_defs()),
                            partition_key_opt,
                            v,
                            validators,
                            transaction,
                        )
                        .map(|_| ())
                    }
                }
                _ => Err(Error::TypeNotExpected),
            }
        })?;

        Ok(results)
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_change_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Vec<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<(), Error>
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
            visit_rel_delete_input::<T, GlobalCtx, RequestCtx>(
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
            });
        }
    } else {
        return Err(Error::TypeNotExpected);
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
) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let mut sg = SuffixGenerator::new();
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

        let results = transaction.read_nodes(&query, partition_key_opt, Some(params), info)?;
        let ids = results
            .iter()
            .map(|n: &Node<GlobalCtx, RequestCtx>| Ok(n.id()?.clone()))
            .collect::<Result<Vec<Value>, Error>>()?;

        if ids.is_empty() {
            Ok(Vec::new())
        } else {
            let create_input = m.remove("create").ok_or_else(|| {
                // Using remove to take ownership
                Error::InputItemNotFound {
                    name: "input::create".to_string(),
                }
            })?;

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
                Value::Array(create_input_array) => create_input_array.into_iter().try_fold(
                    Vec::new(),
                    |mut results, create_input_value| {
                        results.append(&mut visit_rel_create_mutation_input::<
                            T,
                            GlobalCtx,
                            RequestCtx,
                        >(
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
                        )?);
                        Ok(results)
                    },
                ),
                _ => Err(Error::TypeNotExpected),
            }
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_create_mutation_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Vec<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(mut m) = input {
        let dst_prop = info.type_def()?.property("dst")?;
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

        let props = match m.remove("props") {
            None => HashMap::new(),
            Some(Value::Map(hm)) => hm,
            Some(_) => return Err(Error::TypeNotExpected),
        };

        transaction.create_rels::<GlobalCtx, RequestCtx>(
            src_label,
            src_ids,
            &dst_label,
            dst_ids,
            rel_name,
            props,
            props_type_name,
            partition_key_opt,
            info,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

pub(super) fn visit_rel_delete_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids_opt: Option<Vec<Value>>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
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

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;
        let mut sg = SuffixGenerator::new();
        let src_suffix = sg.suffix();
        let dst_suffix = sg.suffix();

        let (read_query, params) = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids_opt,
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
            m.remove("match"), // remove rather than get to take ownership
            transaction,
        )?;

        let read_results =
            transaction.read_rels(&read_query, None, partition_key_opt, Some(params))?;
        let rel_ids = read_results
            .iter()
            .map(|r: &Rel<GlobalCtx, RequestCtx>| r.id().clone())
            .collect();

        let del_results =
            transaction.delete_rels(src_label, rel_name, rel_ids, partition_key_opt)?;

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                read_results.iter().map(|r| r.id().clone()).collect(),
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
            visit_rel_dst_delete_mutation_input::<T, GlobalCtx, RequestCtx>(
                read_results.iter().map(|r| r.id().clone()).collect(),
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
        Err(Error::TypeNotExpected)
    }
}

fn visit_rel_dst_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, Error>
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
            &k,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_query_input<T>(
    label: &str,
    var_suffix: &str,
    params: HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<(Option<String>, HashMap<String, Value>), Error>
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

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (t1, t2) = visit_node_query_input(
                label,
                var_suffix,
                true,
                false,
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
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            &k,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

fn visit_rel_nodes_mutation_input_union<T, GlobalCtx, RequestCtx>(
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<(String, Vec<Value>), Error>
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
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn visit_rel_query_input<T>(
    src_label: &str,
    src_suffix: &str,
    src_ids_opt: Option<Vec<Value>>,
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
) -> Result<(String, HashMap<String, Value>), Error>
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
                src_label,
                src_suffix,
                params,
                sg,
                &Info::new(src_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(src),
                transaction,
            )?
        } else {
            (None, params)
        };

        // Remove used to take ownership
        let (dst_query_opt, params) = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                dst_var,
                dst_suffix,
                params,
                sg,
                &Info::new(dst_prop.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(dst),
                transaction,
            )?
        } else {
            (None, params)
        };

        transaction.rel_query(
            params,
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
        )
    } else {
        transaction.rel_query(
            params,
            src_label,
            src_suffix,
            src_ids_opt,
            None,
            rel_name,
            dst_var,
            dst_suffix,
            None,
            return_rel,
            HashMap::new(),
        )
    }
}

fn visit_rel_src_delete_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    transaction: &mut T,
) -> Result<i32, Error>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
    T: Transaction,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- info.name: {}, ids: {:#?}, input: {:#?}",
        info.name(),
        ids,
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
            label,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
    }
}

fn visit_rel_src_update_mutation_input<T, GlobalCtx, RequestCtx>(
    label: &str,
    ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    transaction: &mut T,
) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(m) = input {
        let (k, v) = m
            .into_iter()
            .next()
            .ok_or_else(|| Error::InputItemNotFound {
                name: info.name().to_string(),
            })?;

        let p = info.type_def()?.property(&k)?;

        visit_node_update_mutation_input::<T, GlobalCtx, RequestCtx>(
            label,
            ids,
            &Info::new(p.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::TypeNotExpected)
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
) -> Result<(Option<String>, HashMap<String, Value>), Error>
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

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = info.type_def()?.property(&k)?;

            let (query, params) = visit_node_query_input(
                label,
                label_suffix,
                false,
                false,
                params,
                sg,
                &Info::new(p.type_name().to_owned(), info.type_defs()),
                partition_key_opt,
                Some(v),
                transaction,
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
    src_label: &str,
    src_ids: Option<Vec<Value>>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
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
        let dst_suffix = sg.suffix();

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

        let results = transaction.read_rels(
            &read_query,
            props_type_name,
            partition_key_opt,
            Some(params),
        )?;

        if let Some(update) = m.remove("update") {
            // remove used to take ownership
            visit_rel_update_mutation_input::<T, GlobalCtx, RequestCtx>(
                src_label,
                results
                    .iter()
                    .map(|r: &Rel<GlobalCtx, RequestCtx>| r.src_id().map(|id| id.clone()))
                    .collect::<Result<Vec<Value>, Error>>()?,
                rel_name,
                results.iter().map(|r| r.id().clone()).collect(),
                results
                    .iter()
                    .map(|r| r.dst_id().map(|id| id.clone()))
                    .collect::<Result<Vec<Value>, Error>>()?,
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
            })
        }
    } else {
        Err(Error::TypeNotExpected)
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_update_mutation_input<T, GlobalCtx, RequestCtx>(
    src_label: &str,
    src_ids: Vec<Value>,
    rel_name: &str,
    rel_ids: Vec<Value>,
    dst_ids: Vec<Value>,
    info: &Info,
    partition_key_opt: Option<&Value>,
    input: Value,
    validators: &Validators,
    props_type_name: Option<&str>,
    transaction: &mut T,
) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
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

    if let Value::Map(mut m) = input {
        let itd = info.type_def()?;

        let props = if let Some(Value::Map(props)) = m.remove("props") {
            props
        } else {
            HashMap::new()
        };

        let results = transaction.update_rels::<GlobalCtx, RequestCtx>(
            src_label,
            rel_name,
            rel_ids,
            props,
            props_type_name,
            partition_key_opt,
        )?;

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
