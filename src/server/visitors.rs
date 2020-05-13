use super::config::WarpgrapherValidators;
use super::schema::{Info, PropertyKind};
use crate::error::{Error, ErrorKind};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::database::{QueryResult, Transaction};
use crate::server::objects::Rel;
use crate::server::value::Value;
use juniper::FieldError;
use log::{debug, trace};
use std::collections::HashMap;
use std::fmt::Debug;

/// Genererates unique suffixes for the variable names used in Cypher queries
pub struct SuffixGenerator {
    seed: i32,
}

impl SuffixGenerator {
    pub fn new() -> SuffixGenerator {
        SuffixGenerator { seed: -1 }
    }

    pub fn get_suffix(&mut self) -> String {
        self.seed += 1;
        String::from("_") + &self.seed.to_string()
    }
}

pub fn visit_node_create_mutation_input<T>(
    label: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_create_mutation_input called -- label: {}, info.name: {}",
        label,
        info.name,
    );

    let itd = info.get_type_def()?;

    if let Value::Map(ref m) = input {
        for k in m.keys() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator
                        .as_ref()
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
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    props.insert(k, v);
                }
                PropertyKind::Input => {
                    inputs.insert(k, v);
                } // Handle these later
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(itd.type_name.clone() + "::" + &k),
                        None,
                    )
                    .into())
                }
            }
        }

        let results = transaction.create_node(label, partition_key_opt, props)?;
        let ids = results.get_ids("n")?;

        for (k, v) in inputs.into_iter() {
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Handled earlier
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            let _ = visit_rel_create_mutation_input(
                                label,
                                ids.clone(),
                                &p.name,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                                partition_key_opt,
                                val,
                                validators,
                                transaction,
                            )?;
                        }
                    } else {
                        let _ = visit_rel_create_mutation_input(
                            label,
                            ids.clone(),
                            &p.name,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                            partition_key_opt,
                            v,
                            validators,
                            transaction,
                        )?;
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(p.type_name.to_owned()),
                        None,
                    )
                    .into())
                }
            }
        }

        Ok(results)
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_node_delete_input<T>(
    label: &str,
    var_suffix: &str,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_input called -- info.name: {}, label: {}, input: {:#?}",
        info.name,
        label,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(mut m) = input {
        let mut params: HashMap<String, Value> = HashMap::new();

        let query = visit_node_query_input(
            label,
            var_suffix,
            false,
            true,
            // "",
            &mut params,
            sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!(
            "visit_node_delete_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, partition_key_opt, Some(params));
        debug!("visit_node_delete_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.get_ids(&(String::from(label) + var_suffix))?;
        trace!("visit_node_delete_input IDs for deletion: {:#?}", ids);

        visit_node_delete_mutation_input(
            label,
            ids,
            &Info::new(
                itd.get_prop("delete")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            Some(m.remove("delete").ok_or_else(|| {
                // remove used to take ownership
                Error::new(
                    ErrorKind::MissingProperty("input::delete".to_owned(), None),
                    None,
                )
            })?),
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_node_delete_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Some(Value::Map(m)) = input {
        for (k, v) in m.into_iter() {
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_delete_input(
                                label,
                                Some(ids.clone()),
                                &k,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                            partition_key_opt,
                            v,
                            transaction,
                        )?;
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + &k),
                        None,
                    )
                    .into())
                }
            }
        }
    }

    transaction.delete_nodes(label, ids, partition_key_opt)
}

fn visit_node_input<T>(
    label: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<Value, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_input Called -- label: {}, info.name: {}, input: {:#?}",
        label,
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(
                ErrorKind::MissingProperty(String::from(&info.name) + "::NEW or ::EXISTING", None),
                None,
            )
        })?;

        let p = itd.get_prop(&k)?;

        match k.as_ref() {
            "NEW" => visit_node_create_mutation_input(
                label,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                partition_key_opt,
                v,
                validators,
                transaction,
            )?
            .get_ids("n"),
            "EXISTING" => {
                let mut sg = SuffixGenerator::new();
                let mut params: HashMap<String, Value> = HashMap::new();
                let var_suffix = sg.get_suffix();
                let query = visit_node_query_input(
                    label,
                    &var_suffix,
                    false,
                    true,
                    // "",
                    &mut params,
                    &mut sg,
                    &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                    partition_key_opt,
                    Some(v),
                    transaction,
                )?;

                debug!(
                    "visit_node_input query, params: {:#?}, {:#?}",
                    query, params
                );
                let results = transaction.exec(&query, partition_key_opt, Some(params))?;
                debug!("visit_node_input Query results: {:#?}", results);
                results.get_ids(&(label.to_owned() + &var_suffix))
            }

            _ => Err(Error::new(
                ErrorKind::InvalidProperty(String::from(&info.name) + "::" + &k),
                None,
            )
            .into()),
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_node_query_input<T>(
    label: &str,
    var_suffix: &str,
    union_type: bool,
    return_node: bool,
    // query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<String, FieldError>
where
    T: Transaction,
{
    trace!(
             "visit_node_query_input called -- label: {}, var_suffix: {}, union_type: {}, return_node: {}, info.name: {}, input: {:#?}",
             label, var_suffix, union_type, return_node, info.name, input,
         );
    // let mut qs = String::from(query);
    let itd = info.get_type_def()?;
    let param_suffix = sg.get_suffix();

    let mut rel_query_fragments = Vec::new();
    let mut props = HashMap::new();
    if let Some(Value::Map(m)) = input {
        for (k, v) in m.into_iter() {
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Scalar => {
                    props.insert(k, v);
                }
                PropertyKind::Input => {
                    rel_query_fragments.push(visit_rel_query_input(
                        label,
                        var_suffix,
                        None,
                        &k,
                        "dst",
                        &sg.get_suffix(),
                        false,
                        // &qs,
                        params,
                        sg,
                        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                        partition_key_opt,
                        Some(v),
                        transaction,
                    )?);
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + &k),
                        None,
                    )
                    .into())
                }
            }
        }
    }

    transaction.node_query_string(
        // &qs,
        rel_query_fragments,
        params,
        label,
        var_suffix,
        union_type,
        return_node,
        &param_suffix,
        props,
    )
}

pub fn visit_node_update_input<T>(
    label: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_update_input called -- info.name: {}, label {}, input: {:#?}",
        info.name,
        label,
        input
    );

    let mut sg = SuffixGenerator::new();
    let itd = info.get_type_def()?;

    if let Value::Map(mut m) = input {
        let var_suffix = sg.get_suffix();
        let mut params: HashMap<String, Value> = HashMap::new();

        let query = visit_node_query_input(
            label,
            &var_suffix,
            false,
            true,
            // "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!(
            "visit_node_update_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, partition_key_opt, Some(params));
        debug!("visit_node_update_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.get_ids(&(String::from(label) + &var_suffix))?;
        trace!("visit_node_update_input IDs for update: {:#?}", ids);

        visit_node_update_mutation_input(
            label,
            ids,
            &Info::new(
                itd.get_prop("modify")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("modify").ok_or_else(|| {
                // remove() used here to take ownership of the "modify" value, not borrow it
                Error::new(
                    ErrorKind::MissingProperty("input::modify".to_owned(), None),
                    None,
                )
            })?,
            validators,
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_node_update_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_update_mutation_input called -- label: {}, ids: {:#?}, info.name: {}, input: {:#?}",
        label,
        ids,
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(ref m) = input {
        for k in m.keys() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator
                        .as_ref()
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
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    props.insert(k.to_owned(), v);
                }
                PropertyKind::Input => {
                    inputs.insert(k, v);
                } // Handle these rels later
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + &k),
                        None,
                    )
                    .into())
                }
            }
        }

        let results = transaction.update_nodes(label, ids, props, partition_key_opt)?;

        for (k, v) in inputs.into_iter() {
            let p = itd.get_prop(&k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Properties handled above
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_change_input(
                                label,
                                results.get_ids("n")?,
                                &k,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                                partition_key_opt,
                                val,
                                validators,
                                transaction,
                            )?;
                        }
                    } else {
                        visit_rel_change_input(
                            label,
                            results.get_ids("n")?,
                            &k,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_rel_change_input<T>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_change_input called -- src_label {}, src_ids {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids,
         rel_name,
         info.name,
         input
     );

    let itd = info.get_type_def()?;

    if let Value::Map(mut m) = input {
        if let Some(v) = m.remove("ADD") {
            // Using remove to take ownership
            visit_rel_create_mutation_input(
                src_label,
                src_ids,
                rel_name,
                &Info::new(
                    itd.get_prop("ADD")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                v,
                validators,
                transaction,
            )
        } else if let Some(v) = m.remove("DELETE") {
            // Using remove to take ownership
            visit_rel_delete_input(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.get_prop("DELETE")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                v,
                transaction,
            )
        } else if let Some(v) = m.remove("UPDATE") {
            // Using remove to take ownership
            visit_rel_update_input(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.get_prop("UPDATE")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                v,
                validators,
                transaction,
            )
        } else {
            Err(Error::new(
                ErrorKind::MissingProperty(
                    String::from(&itd.type_name) + "::ADD|DELETE|UPDATE",
                    None,
                ),
                None,
            )
            .into())
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_rel_create_input<T, GlobalCtx, ReqCtx>(
    src_label: &str,
    rel_name: &str,
    props_type_name: Option<&str>,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, FieldError>
where
    T: Transaction,
    GlobalCtx: Debug,
    ReqCtx: Debug + WarpgrapherRequestContext,
{
    trace!(
        "visit_rel_create_input called -- info.name: {}, rel_name {}, input: {:#?}",
        info.name,
        rel_name,
        input
    );

    let mut sg = SuffixGenerator::new();
    let itd = info.get_type_def()?;

    if let Value::Map(mut m) = input {
        let var_suffix = sg.get_suffix();
        let mut params = HashMap::new();

        let query = visit_node_query_input(
            src_label,
            &var_suffix,
            false,
            true,
            // "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("match"), // Remove used to take ownership
            transaction,
        )?;

        debug!("Query, params: {:#?}, {:#?}", query, params);
        let raw_results = transaction.exec(&query, partition_key_opt, Some(params));
        debug!("Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.get_ids(&(String::from(src_label) + &var_suffix))?;
        trace!("IDs for update: {:#?}", ids);

        let create_input = m.remove("create").ok_or_else(|| {
            // Using remove to take ownership
            Error::new(
                ErrorKind::MissingProperty("input::create".to_owned(), None),
                None,
            )
        })?;

        trace!("visit_rel_create_input calling get_rels.");
        match create_input {
            Value::Map(_) => visit_rel_create_mutation_input(
                src_label,
                ids,
                rel_name,
                &Info::new(
                    itd.get_prop("create")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                create_input,
                validators,
                transaction,
            )?
            .get_rels(&src_label, "", rel_name, "dst", "", props_type_name, info),
            Value::Array(create_input_array) => {
                let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
                for create_input_value in create_input_array {
                    let result = visit_rel_create_mutation_input(
                        src_label,
                        ids.clone(),
                        rel_name,
                        &Info::new(
                            itd.get_prop("create")?.type_name.to_owned(),
                            info.type_defs.clone(),
                        ),
                        partition_key_opt,
                        create_input_value,
                        validators,
                        transaction,
                    )?;

                    for rel in result.get_rels(
                        src_label,
                        "",
                        rel_name,
                        "dst",
                        "",
                        props_type_name,
                        info,
                    )? {
                        v.push(rel)
                    }
                }
                Ok(v)
            }
            _ => Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into()),
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_rel_create_mutation_input<T>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
            "visit_rel_create_mutation_input called -- src_label: {}, src_ids: {:#?}, rel_name: {}, info.name: {}, input: {:#?}",
            src_label,
            src_ids,
            rel_name,
            info.name,
            input
        );

    let itd = info.get_type_def()?;
    let dst_prop = itd.get_prop("dst")?;

    if let Value::Map(mut m) = input {
        let dst = m
            .remove("dst") // Using remove to take ownership
            .ok_or_else(|| Error::new(ErrorKind::MissingProperty("dst".to_owned(), None), None))?;

        let (dst_label, dst_ids) = visit_rel_nodes_mutation_input_union(
            &Info::new(dst_prop.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            dst,
            validators,
            transaction,
        )?;

        transaction.create_rels(
            src_label,
            src_ids,
            &dst_label,
            dst_ids,
            rel_name,
            &mut m,
            partition_key_opt,
            info,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_delete_input<T>(
    src_label: &str,
    src_ids_opt: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_delete_input called -- src_label {}, src_ids_opt {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids_opt,
         rel_name,
         info.name,
         input
     );

    let mut sg = SuffixGenerator::new();
    let itd = info.get_type_def()?;
    let src_suffix = sg.get_suffix();
    let dst_suffix = sg.get_suffix();
    let mut params = HashMap::new();

    if let Value::Map(mut m) = input {
        let read_query = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids_opt,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            // "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("match"), // remove rather than get to take ownership
            transaction,
        )?;

        let read_results = transaction.exec(&read_query, partition_key_opt, Some(params))?;
        let rel_ids =
            read_results.get_ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?;

        let del_results =
            transaction.delete_rels(src_label, rel_name, rel_ids, partition_key_opt)?;

        if let Some(src) = m.remove("src") {
            // Uses remove to take ownership
            visit_rel_src_delete_mutation_input(
                src_label,
                read_results.get_ids(&(src_label.to_string() + &src_suffix))?,
                &Info::new(
                    itd.get_prop("src")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                src,
                transaction,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // Uses remove to take ownership
            visit_rel_dst_delete_mutation_input(
                read_results.get_ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.get_prop("dst")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                dst,
                transaction,
            )?;
        }

        Ok(del_results)
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_dst_delete_mutation_input<T>(
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_dst_delete_mutation_input called -- info.name: {}, input: {:#?}",
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(&k)?;

        visit_node_delete_mutation_input(
            &k,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_dst_query_input<T>(
    label: &str,
    var_suffix: &str,
    // query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<Option<String>, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_dst_query_input called -- label: {}, var_suffix: {}, info.name: {}, input: {:#?}",
         label,
         var_suffix,
         info.name,
         input
     );

    let itd = info.get_type_def()?;

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = itd.get_prop(&k)?;

            Ok(Some(visit_node_query_input(
                label,
                var_suffix,
                true,
                false,
                // query,
                params,
                sg,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                partition_key_opt,
                Some(v),
                transaction,
            )?))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn visit_rel_dst_update_mutation_input<T>(
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_dst_update_mutation_input called -- info.name: {}, ids: {:#?}, input: {:#?}",
        info.name,
        ids,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(&k)?;

        visit_node_update_mutation_input(
            &k,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

fn visit_rel_nodes_mutation_input_union<T>(
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<(String, Value), FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_nodes_mutation_input_union called -- info.name: {}, input: {:#?}",
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(&k)?;

        let dst_ids = visit_node_input(
            &k,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )?;

        Ok((k.to_owned(), dst_ids))
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_rel_query_input<T>(
    src_label: &str,
    src_suffix: &str,
    src_ids_opt: Option<Value>,
    rel_name: &str,
    dst_var: &str,
    dst_suffix: &str,
    return_rel: bool,
    // query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: &Option<String>,
    input_opt: Option<Value>,
    transaction: &mut T,
) -> Result<String, FieldError>
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
        info.name,
        input_opt,
    );

    let itd = info.get_type_def()?;
    let src_prop = itd.get_prop("src")?;
    let dst_prop = itd.get_prop("dst")?;

    let mut props = HashMap::new();
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
        let src_query_opt = if let Some(src) = m.remove("src") {
            visit_rel_src_query_input(
                src_label,
                src_suffix,
                // &query,
                params,
                sg,
                &Info::new(src_prop.type_name.to_owned(), info.type_defs.clone()),
                partition_key_opt,
                Some(src),
                transaction,
            )?
        } else {
            None
        };

        // Remove used to take ownership
        let dst_query_opt = if let Some(dst) = m.remove("dst") {
            visit_rel_dst_query_input(
                dst_var,
                dst_suffix,
                // &query,
                params,
                sg,
                &Info::new(dst_prop.type_name.to_owned(), info.type_defs.clone()),
                partition_key_opt,
                Some(dst),
                transaction,
            )?
        } else {
            None
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
            params,
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
            params,
        )
    }
}

pub fn visit_rel_src_delete_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_src_delete_mutation_input called -- info.name: {}, ids: {:#?}, input: {:#?}",
        info.name,
        ids,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(&k)?;

        visit_node_delete_mutation_input(
            label,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            Some(v),
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_src_update_mutation_input<T>(
    label: &str,
    ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_src_update_mutation_input called -- info.name: {}, label: {}, ids: {:#?}, input: {:#?}",
         info.name,
         label,
         ids,
         input
     );

    let itd = info.get_type_def()?;

    if let Value::Map(m) = input {
        let (k, v) = m.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(&k)?;

        visit_node_update_mutation_input(
            label,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            partition_key_opt,
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_src_query_input<T>(
    label: &str,
    label_suffix: &str,
    // query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Option<Value>,
    transaction: &mut T,
) -> Result<Option<String>, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_src_query_input called -- label: {}, label_suffix: {}, info.name: {}, input: {:#?}",
         label,
         label_suffix,
         info.name,
         input
     );

    let itd = info.get_type_def()?;

    if let Some(Value::Map(m)) = input {
        if let Some((k, v)) = m.into_iter().next() {
            let p = itd.get_prop(&k)?;

            Ok(Some(visit_node_query_input(
                label,
                label_suffix,
                false,
                false,
                // query,
                params,
                sg,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                partition_key_opt,
                Some(v),
                transaction,
            )?))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_rel_update_input<T>(
    src_label: &str,
    src_ids: Option<Value>,
    rel_name: &str,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_update_input called -- src_label {}, src_ids {:#?}, rel_name {}, info.name: {}, input: {:#?}",
         src_label,
         src_ids,
         rel_name,
         info.name,
         input
     );

    let mut sg = SuffixGenerator::new();
    let itd = info.get_type_def()?;
    let mut params = HashMap::new();
    let src_suffix = sg.get_suffix();
    let dst_suffix = sg.get_suffix();

    if let Value::Map(mut m) = input {
        let read_query = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            // "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            partition_key_opt,
            m.remove("match"), // uses remove to take ownership
            transaction,
        )?;

        debug!(
            "visit_rel_update_input query, params: {:#?}, {:#?}",
            read_query, params
        );
        let raw_read_results = transaction.exec(&read_query, partition_key_opt, Some(params));
        debug!("visit_rel_update_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        if let Some(update) = m.remove("update") {
            // remove used to take ownership
            visit_rel_update_mutation_input(
                src_label,
                read_results.get_ids(&(String::from(src_label) + &src_suffix))?,
                rel_name,
                read_results.get_ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
                read_results.get_ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.get_prop("update")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                update,
                validators,
                transaction,
            )
        } else {
            Err(Error::new(ErrorKind::MissingProperty("id".to_owned(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None).into())
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_rel_update_mutation_input<T>(
    src_label: &str,
    src_ids: Value,
    rel_name: &str,
    rel_ids: Value,
    dst_ids: Value,
    info: &Info,
    partition_key_opt: &Option<String>,
    input: Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
         "visit_rel_update_mutation_input called -- info.name: {}, src_label: {}, src_ids: {:#?}, rel_name: {}, rel_ids: {:#?}, dst_ids: {:#?}, input: {:#?}",
         info.name,
         src_label,
         src_ids,
         rel_name,
         rel_ids,
         dst_ids,
         input
     );

    let itd = info.get_type_def()?;

    if let Value::Map(mut m) = input {
        let mut props = HashMap::new();
        if let Some(Value::Map(pm)) = m.remove("props") {
            // remove used to take ownership
            for (k, v) in pm.into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        let results =
            transaction.update_rels(src_label, rel_name, rel_ids, partition_key_opt, props)?;

        trace!("visit_rel_update_mutation_input results: {:#?}", results);

        if let Some(src) = m.remove("src") {
            // calling remove to take ownership
            visit_rel_src_update_mutation_input(
                src_label,
                src_ids,
                &Info::new(
                    itd.get_prop("src")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                src,
                validators,
                transaction,
            )?;
        }

        if let Some(dst) = m.remove("dst") {
            // calling remove to take ownership
            visit_rel_dst_update_mutation_input(
                dst_ids,
                &Info::new(
                    itd.get_prop("dst")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                partition_key_opt,
                dst,
                validators,
                transaction,
            )?;
        }

        Ok(results)
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

fn validate_input(
    validators: &WarpgrapherValidators,
    v: &str,
    input: &Value,
) -> Result<(), FieldError> {
    let func = validators.get(v).ok_or_else(|| {
        Error::new(
            ErrorKind::ValidatorNotFound(
                format!(
                    "Could not find custom input validator: {validator_name}.",
                    validator_name = v
                ),
                v.to_owned(),
            ),
            None,
        )
    })?;

    trace!(
        "validate_input Calling input validator function {} for input value {:#?}",
        v,
        input
    );

    func(input).or_else(|e| match e.kind {
        ErrorKind::ValidationError(v) => Err(FieldError::new(
            v,
            juniper::graphql_value!({ "internal_error": "Input validation failed" }),
        )),
        _ => Err(FieldError::from(e)),
    })
}
