use super::config::WarpgrapherValidators;
use super::schema::{Info, PropertyKind};
use crate::error::{Error, ErrorKind};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::database::{QueryResult, Transaction};
use crate::server::objects::Rel;
use juniper::FieldError;
use log::{debug, trace};
use serde_json::{Map, Value};
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
    input: &Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<T::ImplQueryResult, FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_node_create_mutation_input called -- label: {}, info.name: {}, input: {:#?}",
        label,
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    let mut props = HashMap::new();
    if let Value::Object(ref m) = input {
        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator
                        .as_ref()
                        .map_or(Ok(()), |v_name| validate_input(validators, &v_name, &input))?;

                    props.insert(k.to_owned(), v.clone());
                }
                PropertyKind::Input => {} // Handle these later
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(itd.type_name.clone() + "::" + k),
                        None,
                    )
                    .into())
                }
            }
        }

        let query = String::from("CREATE (n:")
            + label
            + " { id: randomUUID() })\n"
            + "SET n += $props\n"
            + "RETURN n\n";
        let mut params = HashMap::new();
        params.insert("props".to_owned(), props);

        debug!(
            "visit_node_create_mutation_input Query statement query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, Some(&params));
        debug!(
            "visit_node_create_mutation_input Raw results: {:#?}",
            raw_results
        );
        let results = raw_results?;
        let ids = results.get_ids("n")?;

        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Handled earlier
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            let _ = visit_rel_create_mutation_input(
                                label,
                                &ids,
                                &p.name,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                                val,
                                validators,
                                transaction,
                            )?;
                        }
                    } else {
                        let _ = visit_rel_create_mutation_input(
                            label,
                            &ids,
                            &p.name,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
    input: &Value,
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

    if let Value::Object(ref m) = input {
        let mut params: HashMap<String, Value> = HashMap::new();

        let query = visit_node_query_input(
            label,
            var_suffix,
            false,
            true,
            "",
            &mut params,
            sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("match"),
        )?;

        debug!(
            "visit_node_delete_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, Some(&params));
        debug!("visit_node_delete_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.get_ids(&(String::from(label) + var_suffix))?;
        trace!("visit_node_delete_input IDs for deletion: {:#?}", ids);

        visit_node_delete_mutation_input(
            label,
            &ids,
            &Info::new(
                itd.get_prop("delete")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            Some(m.get("delete").ok_or_else(|| {
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
    ids: &[String],
    info: &Info,
    input: Option<&Value>,
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
    let mut force = false;

    if let Some(Value::Object(ref m)) = input {
        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar => {
                    if k == "force" && *v == Value::Bool(true) {
                        force = true
                    }
                }
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_delete_input(
                                label,
                                Some(ids),
                                k,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                                val,
                                transaction,
                            )?;
                        }
                    } else {
                        visit_rel_delete_input(
                            label,
                            Some(ids),
                            k,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                            v,
                            transaction,
                        )?;
                    }
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + k),
                        None,
                    )
                    .into())
                }
            }
        }
    }

    let query = String::from("MATCH (n:")
        + label
        + ")\n"
        + "WHERE n.id IN $ids\n"
        + if force { "DETACH " } else { "" }
        + "DELETE n\n"
        + "RETURN count(*) as count\n";
    let mut params = HashMap::new();
    params.insert("ids".to_owned(), &ids);

    debug!(
        "visit_node_delete_mutation_input query, params: {:#?}, {:#?}",
        query, params
    );
    let results = transaction.exec(&query, Some(&params))?;
    debug!(
        "visit_node_delete_mutation_input Query results: {:#?}",
        results
    );

    Ok(results)
}

fn visit_node_input<T>(
    label: &str,
    info: &Info,
    input: &Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<Vec<String>, FieldError>
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

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(
                ErrorKind::MissingProperty(String::from(&info.name) + "::NEW or ::EXISTING", None),
                None,
            )
        })?;

        let p = itd.get_prop(k)?;

        match k.as_ref() {
            "NEW" => visit_node_create_mutation_input(
                label,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
                    "",
                    &mut params,
                    &mut sg,
                    &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                    Some(v),
                )?;

                debug!(
                    "visit_node_input query, params: {:#?}, {:#?}",
                    query, params
                );
                let results = transaction.exec(&query, Some(&params))?;
                debug!("visit_node_input Query results: {:#?}", results);
                results.get_ids(&(label.to_owned() + &var_suffix))
            }

            _ => Err(Error::new(
                ErrorKind::InvalidProperty(String::from(&info.name) + "::" + k),
                None,
            )
            .into()),
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn visit_node_query_input(
    label: &str,
    var_suffix: &str,
    union_type: bool,
    return_node: bool,
    query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    input: Option<&Value>,
) -> Result<String, FieldError> {
    trace!(
             "visit_node_query_input called -- label: {}, var_suffix: {}, union_type: {}, return_node: {}, info.name: {}, input: {:#?}",
             label, var_suffix, union_type, return_node, info.name, input,
         );
    let mut qs = String::from(query);
    let itd = info.get_type_def()?;
    let param_suffix = sg.get_suffix();

    let mut props = Map::new();
    if let Some(Value::Object(ref m)) = input {
        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar => {
                    props.insert(k.to_owned(), v.clone());
                }
                PropertyKind::Input => {
                    qs.push_str(&visit_rel_query_input(
                        label,
                        var_suffix,
                        None,
                        k,
                        "dst",
                        &sg.get_suffix(),
                        false,
                        &qs,
                        params,
                        sg,
                        &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                        Some(v),
                    )?);
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + k),
                        None,
                    )
                    .into())
                }
            }
        }
    }

    if union_type {
        qs.push_str(&(String::from("MATCH (") + label + var_suffix + ")\n"));
    } else {
        qs.push_str(&(String::from("MATCH (") + label + var_suffix + ":" + label + ")\n"));
    }

    let mut wc = None;
    for k in props.keys() {
        match wc {
            None => {
                wc = Some(
                    String::from("WHERE ")
                        + label
                        + var_suffix
                        + "."
                        + &k
                        + "=$"
                        + label
                        + &param_suffix
                        + "."
                        + &k,
                )
            }
            Some(wcs) => {
                wc =
                    Some(wcs + " AND " + label + "." + &k + "=$" + label + &param_suffix + "." + &k)
            }
        }
    }
    if let Some(wcs) = wc {
        qs.push_str(&(String::from(&wcs) + "\n"));
    }

    params.insert(String::from(label) + &param_suffix, props.into());

    if return_node {
        qs.push_str(&(String::from("RETURN ") + label + var_suffix + "\n"));
    }

    Ok(qs)
}

pub fn visit_node_update_input<T>(
    label: &str,
    info: &Info,
    input: &Value,
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

    if let Value::Object(ref m) = input {
        let var_suffix = sg.get_suffix();
        let mut params: HashMap<String, Value> = HashMap::new();

        let query = visit_node_query_input(
            label,
            &var_suffix,
            false,
            true,
            "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("match"),
        )?;

        debug!(
            "visit_node_update_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, Some(&params));
        debug!("visit_node_update_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = results.get_ids(&(String::from(label) + &var_suffix))?;
        trace!("visit_node_update_input IDs for update: {:#?}", ids);

        visit_node_update_mutation_input(
            label,
            &ids,
            &Info::new(
                itd.get_prop("modify")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("modify").ok_or_else(|| {
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
    ids: &[String],
    info: &Info,
    input: &Value,
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

    let mut props = Map::new();

    if let Value::Object(ref m) = input {
        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {
                    p.validator
                        .as_ref()
                        .map_or(Ok(()), |v| validate_input(validators, &v, &input))?;

                    props.insert(k.to_owned(), v.clone());
                }
                PropertyKind::Input => {} // Handle these rels later
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(String::from(&info.name) + "::" + k),
                        None,
                    )
                    .into())
                }
            }
        }

        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("ids".to_owned(), ids.into());
        params.insert("props".to_owned(), props.into());

        let query = String::from("MATCH (n:")
            + label
            + ")\n"
            + "WHERE n.id IN $ids\n"
            + "SET n += $props\n"
            + "RETURN n\n";

        debug!(
            "visit_node_update_mutation_input query, params: {:#?}, {:#?}",
            query, params
        );
        let raw_results = transaction.exec(&query, Some(&params));
        debug!(
            "visit_node_update_mutation_input Query results: {:#?}",
            raw_results
        );

        let results = raw_results?;

        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Properties handled above
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_change_input(
                                label,
                                &results.get_ids("n")?,
                                k,
                                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                                val,
                                validators,
                                transaction,
                            )?;
                        }
                    } else {
                        visit_rel_change_input(
                            label,
                            &results.get_ids("n")?,
                            k,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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

pub fn visit_rel_change_input<T>(
    src_label: &str,
    src_ids: &[String],
    rel_name: &str,
    info: &Info,
    input: &Value,
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

    if let Value::Object(ref m) = input {
        if let Some(v) = m.get("ADD") {
            visit_rel_create_mutation_input(
                src_label,
                src_ids,
                rel_name,
                &Info::new(
                    itd.get_prop("ADD")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                v,
                validators,
                transaction,
            )
        } else if let Some(v) = m.get("DELETE") {
            visit_rel_delete_input(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.get_prop("DELETE")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                v,
                transaction,
            )
        } else if let Some(v) = m.get("UPDATE") {
            visit_rel_update_input(
                src_label,
                Some(src_ids),
                rel_name,
                &Info::new(
                    itd.get_prop("UPDATE")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
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

pub fn visit_rel_create_input<T, GlobalCtx, ReqCtx>(
    src_label: &str,
    rel_name: &str,
    props_type_name: Option<&str>,
    info: &Info,
    input: &Value,
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

    if let Value::Object(ref m) = input {
        let var_suffix = sg.get_suffix();
        let mut params = HashMap::new();

        let query = visit_node_query_input(
            src_label,
            &var_suffix,
            false,
            true,
            "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("match"),
        )?;

        debug!("Query, params: {:#?}, {:#?}", query, params);
        let raw_results = transaction.exec(&query, Some(&params));
        debug!("Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = &results.get_ids(&(String::from(src_label) + &var_suffix))?;
        trace!("IDs for update: {:#?}", ids);

        let create_input = m.get("create").ok_or_else(|| {
            Error::new(
                ErrorKind::MissingProperty("input::create".to_owned(), None),
                None,
            )
        })?;

        trace!("visit_rel_create_input calling get_rels.");
        match &create_input {
            Value::Object(_) => visit_rel_create_mutation_input(
                src_label,
                &ids,
                rel_name,
                &Info::new(
                    itd.get_prop("create")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                &create_input,
                validators,
                transaction,
            )?
            .get_rels(&src_label, "", rel_name, "dst", "", props_type_name),
            Value::Array(create_input_array) => {
                let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
                for create_input_value in create_input_array {
                    let result = visit_rel_create_mutation_input(
                        src_label,
                        &ids,
                        rel_name,
                        &Info::new(
                            itd.get_prop("create")?.type_name.to_owned(),
                            info.type_defs.clone(),
                        ),
                        create_input_value,
                        validators,
                        transaction,
                    )?;

                    for rel in
                        result.get_rels(src_label, "", rel_name, "dst", "", props_type_name)?
                    {
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

pub fn visit_rel_create_mutation_input<T>(
    src_label: &str,
    src_ids: &[String],
    rel_name: &str,
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let dst = m
            .get("dst")
            .ok_or_else(|| Error::new(ErrorKind::MissingProperty("dst".to_owned(), None), None))?;

        let (dst_label, dst_ids) = visit_rel_nodes_mutation_input_union(
            &Info::new(dst_prop.type_name.to_owned(), info.type_defs.clone()),
            dst,
            validators,
            transaction,
        )?;

        let mut props = Map::new();
        if let Some(Value::Object(pm)) = m.get("props") {
            for (k, v) in pm.iter() {
                props.insert(k.to_owned(), v.clone());
            }
        }

        let query = String::from("MATCH (")
            + src_label
            + ":"
            + src_label
            + "),(dst:"
            + &dst_label
            + ")"
            + "\n"
            + "WHERE "
            + src_label
            + ".id IN $aid AND dst.id IN $bid\n"
            + "CREATE ("
            + src_label
            + ")-["
            + rel_name
            + ":"
            + String::from(rel_name).as_str()
            + " { id: randomUUID() }]->(dst)\n"
            + "SET "
            + rel_name
            + " += $props\n"
            + "RETURN "
            + src_label
            + ", "
            + rel_name
            + ", dst, labels(dst) as dst_label\n";

        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("aid".to_owned(), src_ids.into());
        params.insert("bid".to_owned(), dst_ids.into());
        params.insert("props".to_owned(), props.into());

        debug!(
            "visit_rel_create_mutation_input query, params: {:#?}, {:#?}",
            query, params
        );
        let results = transaction.exec(&query, Some(&params))?;
        debug!(
            "visit_rel_create_mutation_input Query results: {:#?}",
            results
        );

        Ok(results)
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_delete_input<T>(
    src_label: &str,
    src_ids_opt: Option<&[String]>,
    rel_name: &str,
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let read_query = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids_opt,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("match"),
        )?;

        debug!(
            "visit_rel_delete_input query, params: {:#?}, {:#?}",
            read_query, params
        );
        let raw_read_results = transaction.exec(&read_query, Some(&params));
        debug!("visit_rel_delete_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        let del_query = String::from("MATCH (")
            + src_label
            + ":"
            + src_label
            + ")-["
            + rel_name
            + ":"
            + rel_name
            + "]->()\n"
            + "WHERE "
            + rel_name
            + ".id IN $rids\n"
            + "DELETE "
            + rel_name
            + "\n"
            + "RETURN count(*) as count\n";
        params.insert(
            "rids".to_owned(),
            read_results
                .get_ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?
                .into(),
        );
        debug!(
            "visit_rel_delete_input query, params: {:#?}, {:#?}",
            del_query, params
        );
        let raw_del_results = transaction.exec(&del_query, Some(&params));
        debug!("visit_rel_delete_input Raw result: {:#?}", raw_del_results);

        let del_results = raw_del_results?;

        if let Some(src) = m.get("src") {
            visit_rel_src_delete_mutation_input(
                src_label,
                &read_results.get_ids(&(src_label.to_string() + &src_suffix))?,
                &Info::new(
                    itd.get_prop("src")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                src,
                transaction,
            )?;
        }

        if let Some(dst) = m.get("dst") {
            visit_rel_dst_delete_mutation_input(
                &read_results.get_ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.get_prop("dst")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
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
    ids: &[String],
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(k)?;

        visit_node_delete_mutation_input(
            k,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            Some(v),
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

fn visit_rel_dst_query_input(
    label: &str,
    var_suffix: &str,
    query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    input: Option<&Value>,
) -> Result<String, FieldError> {
    trace!(
         "visit_rel_dst_query_input called -- label: {}, var_suffix: {}, info.name: {}, input: {:#?}",
         label,
         var_suffix,
         info.name,
         input
     );

    let itd = info.get_type_def()?;

    if let Some(Value::Object(m)) = input {
        if let Some((k, v)) = m.iter().next() {
            let p = itd.get_prop(k)?;

            visit_node_query_input(
                label,
                var_suffix,
                true,
                false,
                query,
                params,
                sg,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                Some(v),
            )
        } else {
            Ok(query.to_owned())
        }
    } else {
        Ok(query.to_owned())
    }
}

pub fn visit_rel_dst_update_mutation_input<T>(
    ids: &[String],
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(k)?;

        visit_node_update_mutation_input(
            k,
            &ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
    input: &Value,
    validators: &WarpgrapherValidators,
    transaction: &mut T,
) -> Result<(String, Vec<String>), FieldError>
where
    T: Transaction,
{
    trace!(
        "visit_rel_nodes_mutation_input_union called -- info.name: {}, input: {:#?}",
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(k)?;

        let dst_ids = visit_node_input(
            k,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
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
pub fn visit_rel_query_input(
    src_label: &str,
    src_suffix: &str,
    src_ids_opt: Option<&[String]>,
    rel_name: &str,
    dst_var: &str,
    dst_suffix: &str,
    return_rel: bool,
    query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    input_opt: Option<&Value>,
) -> Result<String, FieldError> {
    trace!(
        "visit_rel_query_input called -- src_label: {}, src_suffix: {}, rel_name: {}, dst_var: {}, dst_suffix: {}, return_rel: {:#?}, query: {}, info.name: {}, input: {:#?}",
        src_label,
        src_suffix,
        rel_name,
        dst_var,
        dst_suffix,
        return_rel,
        query,
        info.name,
        input_opt,
    );

    let mut qs = query.to_string();

    let itd = info.get_type_def()?;
    let src_prop = itd.get_prop("src")?;
    let dst_prop = itd.get_prop("dst")?;

    let mut props = Map::new();
    if let Some(Value::Object(ref m)) = input_opt {
        if let Some(id) = m.get("id") {
            props.insert("id".to_owned(), id.clone());
        }

        if let Some(Value::Object(rel_props)) = m.get("props") {
            for (k, v) in rel_props.iter() {
                props.insert(k.to_owned(), v.clone());
            }
        }
    }

    qs.push_str(
        &(String::from("MATCH (")
            + src_label
            + src_suffix
            + ":"
            + src_label
            + ")-["
            + rel_name
            + src_suffix
            + dst_suffix
            + ":"
            + String::from(rel_name).as_str()
            + "]->("
            + dst_var
            + dst_suffix
            + ")\n"),
    );

    let mut wc = None;
    for k in props.keys() {
        match wc {
            None => {
                wc = Some(
                    String::from("WHERE ")
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k
                        + " = $"
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k,
                )
            }
            Some(wcs) => {
                wc = Some(
                    wcs + " AND "
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k
                        + " = $"
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k,
                )
            }
        }
    }

    if let Some(src_ids) = src_ids_opt {
        match wc {
            None => {
                wc = Some(
                    String::from("WHERE ")
                        + src_label
                        + src_suffix
                        + ".id IN $"
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "_srcids"
                        + "."
                        + "ids",
                )
            }
            Some(wcs) => {
                wc = Some(
                    wcs + " AND "
                        + src_label
                        + src_suffix
                        + ".id IN $"
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "_srcids"
                        + "."
                        + "ids",
                )
            }
        }
        let mut id_map = Map::new();
        id_map.insert(
            "ids".to_string(),
            src_ids
                .iter()
                .map(|s| Value::String(s.to_owned()))
                .collect(),
        );

        params.insert(
            String::from(rel_name) + src_suffix + dst_suffix + "_srcids",
            id_map.into(),
        );
    }

    if let Some(wcs) = wc {
        qs.push_str(&(String::from(&wcs) + "\n"));
    }
    params.insert(
        String::from(rel_name) + src_suffix + dst_suffix,
        props.into(),
    );

    if let Some(Value::Object(ref m)) = input_opt {
        if let Some(src) = m.get("src") {
            qs.push_str(&visit_rel_src_query_input(
                src_label,
                src_suffix,
                &query,
                params,
                sg,
                &Info::new(src_prop.type_name.to_owned(), info.type_defs.clone()),
                Some(src),
            )?);
        }

        if let Some(dst) = m.get("dst") {
            qs.push_str(&visit_rel_dst_query_input(
                dst_var,
                dst_suffix,
                &query,
                params,
                sg,
                &Info::new(dst_prop.type_name.to_owned(), info.type_defs.clone()),
                Some(dst),
            )?);
        }
    }

    if return_rel {
        qs.push_str(
            &(String::from("RETURN ")
                + src_label
                + src_suffix
                + ", "
                + rel_name
                + src_suffix
                + dst_suffix
                + ", "
                + dst_var
                + dst_suffix
                + ", "
                + "labels("
                + dst_var
                + dst_suffix
                + ") as "
                + dst_var
                + dst_suffix
                + "_label\n"),
        );
    }

    Ok(qs)
}

pub fn visit_rel_src_delete_mutation_input<T>(
    label: &str,
    ids: &[String],
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(k)?;

        visit_node_delete_mutation_input(
            label,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            Some(v),
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_src_update_mutation_input<T>(
    label: &str,
    ids: &[String],
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let (k, v) = m.iter().next().ok_or_else(|| {
            Error::new(ErrorKind::MissingProperty(info.name.to_owned(), None), None)
        })?;

        let p = itd.get_prop(k)?;

        visit_node_update_mutation_input(
            label,
            ids,
            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
            v,
            validators,
            transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

fn visit_rel_src_query_input(
    label: &str,
    label_suffix: &str,
    query: &str,
    params: &mut HashMap<String, Value>,
    sg: &mut SuffixGenerator,
    info: &Info,
    input: Option<&Value>,
) -> Result<String, FieldError> {
    trace!(
         "visit_rel_src_query_input called -- label: {}, label_suffix: {}, info.name: {}, input: {:#?}",
         label,
         label_suffix,
         info.name,
         input
     );

    let itd = info.get_type_def()?;

    if let Some(Value::Object(m)) = input {
        if let Some((k, v)) = m.iter().next() {
            let p = itd.get_prop(k)?;

            visit_node_query_input(
                label,
                label_suffix,
                false,
                false,
                query,
                params,
                sg,
                &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                Some(v),
            )
        } else {
            Ok(query.to_owned())
        }
    } else {
        Ok(query.to_owned())
    }
}

pub fn visit_rel_update_input<T>(
    src_label: &str,
    src_ids: Option<&[String]>,
    rel_name: &str,
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let read_query = visit_rel_query_input(
            src_label,
            &src_suffix,
            src_ids,
            rel_name,
            "dst",
            &dst_suffix,
            true,
            "",
            &mut params,
            &mut sg,
            &Info::new(
                itd.get_prop("match")?.type_name.to_owned(),
                info.type_defs.clone(),
            ),
            m.get("match"),
        )?;

        debug!(
            "visit_rel_update_input query, params: {:#?}, {:#?}",
            read_query, params
        );
        let raw_read_results = transaction.exec(&read_query, Some(&params));
        debug!("visit_rel_update_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        if let Some(update) = m.get("update") {
            visit_rel_update_mutation_input(
                src_label,
                &read_results.get_ids(&(String::from(src_label) + &src_suffix))?,
                rel_name,
                &read_results.get_ids(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
                &read_results.get_ids(&(String::from("dst") + &dst_suffix))?,
                &Info::new(
                    itd.get_prop("update")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
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
    src_ids: &[String],
    rel_name: &str,
    rel_ids: &[String],
    dst_ids: &[String],
    info: &Info,
    input: &Value,
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

    if let Value::Object(m) = input {
        let mut props = Map::new();
        if let Some(Value::Object(pm)) = m.get("props") {
            for (k, v) in pm.iter() {
                props.insert(k.to_owned(), v.clone());
            }
        }

        let query = String::from("MATCH (")
            + src_label
            + ":"
            + src_label
            + ")-["
            + rel_name
            + ":"
            + String::from(rel_name).as_str()
            + "]->(dst)\n"
            + "WHERE "
            + rel_name
            + ".id IN $rids\n"
            + "SET "
            + rel_name
            + " += $props\n"
            + "RETURN "
            + src_label
            + ", "
            + rel_name
            + ", dst, labels(dst) as dst_label\n";

        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("rids".to_owned(), rel_ids.into());
        params.insert("props".to_owned(), props.into());
        debug!(
            "visit_rel_update_mutation_input query, params: {:#?}, {:#?}",
            query, params
        );
        let results = transaction.exec(&query, Some(&params))?;
        debug!(
            "visit_rel_update_mutation_input Query results: {:#?}",
            results
        );

        if let Some(src) = m.get("src") {
            visit_rel_src_update_mutation_input(
                src_label,
                src_ids,
                &Info::new(
                    itd.get_prop("src")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
                src,
                validators,
                transaction,
            )?;
        }

        if let Some(dst) = m.get("dst") {
            visit_rel_dst_update_mutation_input(
                dst_ids,
                &Info::new(
                    itd.get_prop("dst")?.type_name.to_owned(),
                    info.type_defs.clone(),
                ),
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
        "validate_input Calling input validator function {validator_name} for input value {input_value}",
        validator_name = v,
        input_value = input
    );

    func(input).or_else(|e| match e.kind {
        ErrorKind::ValidationError(v) => Err(FieldError::new(
            v,
            juniper::graphql_value!({ "internal_error": "Input validation failed" }),
        )),
        _ => Err(FieldError::from(e)),
    })
}
