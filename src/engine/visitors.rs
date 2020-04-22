use super::config::Validators;
use super::schema::{Info, PropertyKind};
use crate::error::{Error, ErrorKind};
use juniper::FieldError;
use log::{debug, trace};
use rusted_cypher::cypher::result::CypherResult;
use rusted_cypher::cypher::transaction::{Started, Transaction};
use rusted_cypher::Statement;
use serde_json::Value;
use std::collections::BTreeMap;

/// Genererates unique suffixes for the variable names used in Cypher queries
#[derive(Default)]
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

fn extract_ids(results: &CypherResult, name: &str) -> Result<Vec<String>, FieldError> {
    trace!("extract_ids called -- name: {}", name);

    let mut v = Vec::new();
    for row in results.rows() {
        let n: Value = row.get(name)?;
        if let Value::String(id) = n
            .get("id")
            .ok_or_else(|| Error::new(ErrorKind::MissingProperty("id".to_owned(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None))?
        {
            v.push(id.to_owned());
        } else {
            return Err(Error::new(ErrorKind::InvalidPropertyType("id".to_owned()), None).into());
        }
    }

    trace!("extract_ids ids: {:#?}", v);
    Ok(v)
}

pub fn visit_node_create_mutation_input(
    label: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
    trace!(
        "visit_node_create_mutation_input called -- label: {}, info.name: {}, input: {:#?}",
        label,
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    let mut props = BTreeMap::new();
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

        let statement = Statement::new(
            String::from("CREATE (n:")
                + label
                + " { id: randomUUID() })\n"
                + "SET n += $props\n"
                + "RETURN n\n",
        )
        .with_param("props".to_owned(), &props)?;

        debug!(
            "visit_node_create_mutation_input Query statement: {:#?}",
            statement
        );
        let raw_results = transaction.exec(statement);
        debug!(
            "visit_node_create_mutation_input Raw results: {:#?}",
            raw_results
        );
        let results = raw_results?;
        let ids = extract_ids(&results, "n")?;

        for (k, v) in m.iter() {
            let p = itd.get_prop(k)?;

            match p.kind {
                PropertyKind::Scalar | PropertyKind::DynamicScalar => {} // Handled earlier
                PropertyKind::Input => {
                    if let Value::Array(input_array) = v {
                        for val in input_array {
                            visit_rel_create_mutation_input(
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
                        visit_rel_create_mutation_input(
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

pub fn visit_node_delete_input(
    label: &str,
    var_suffix: &str,
    sg: &mut SuffixGenerator,
    info: &Info,
    input: &Value,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
    trace!(
        "visit_node_delete_input called -- info.name: {}, label: {}, input: {:#?}",
        info.name,
        label,
        input
    );

    let itd = info.get_type_def()?;

    if let Value::Object(ref m) = input {
        let mut params = BTreeMap::new();

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

        let mut statement = Statement::new(query);
        statement.set_parameters(&params)?;
        debug!("visit_node_delete_input Query: {:#?}", statement);
        let raw_results = transaction.exec(statement);
        debug!("visit_node_delete_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = extract_ids(&results, &(String::from(label) + var_suffix))?;
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

pub fn visit_node_delete_mutation_input(
    label: &str,
    ids: &[String],
    info: &Info,
    input: Option<&Value>,
    mut transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
                                &mut transaction,
                            )?;
                        }
                    } else {
                        visit_rel_delete_input(
                            label,
                            Some(ids),
                            k,
                            &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                            v,
                            &mut transaction,
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

    let statement = Statement::new(
        String::from("MATCH (n:")
            + label
            + ")\n"
            + "WHERE n.id IN $ids\n"
            + if force { "DETACH " } else { "" }
            + "DELETE n\n"
            + "RETURN count(*) as count\n",
    )
    .with_param("ids", &ids)?;

    debug!(
        "visit_node_delete_mutation_input Query statement: {:#?}",
        statement
    );
    let results = transaction.exec(statement)?;
    debug!(
        "visit_node_delete_mutation_input Query results: {:#?}",
        results
    );

    Ok(results)
}

fn visit_node_input(
    label: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<Vec<String>, FieldError> {
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
            "NEW" => Ok(extract_ids(
                &visit_node_create_mutation_input(
                    label,
                    &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
                    v,
                    validators,
                    transaction,
                )?,
                "n",
            )?),
            "EXISTING" => {
                let mut sg = SuffixGenerator::new();
                let mut params = BTreeMap::new();
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

                let mut statement = Statement::new(query);
                statement.set_parameters(&params)?;
                debug!("visit_node_input Query statement: {:#?}", statement);
                let results = transaction.exec(statement)?;
                debug!("visit_node_input Query results: {:#?}", results);
                extract_ids(&results, &(label.to_owned() + &var_suffix))
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
    params: &mut BTreeMap<String, BTreeMap<String, Value>>,
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

    let mut props = BTreeMap::new();
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

    params.insert(String::from(label) + &param_suffix, props);

    if return_node {
        qs.push_str(&(String::from("RETURN ") + label + var_suffix + "\n"));
    }

    Ok(qs)
}

pub fn visit_node_update_input(
    label: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    mut transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
        let mut params = BTreeMap::new();

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

        let mut statement = Statement::new(query);
        statement.set_parameters(&params)?;
        debug!("visit_node_update_input Query: {:#?}", statement);
        let raw_results = transaction.exec(statement);
        debug!("visit_node_update_input Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = extract_ids(&results, &(String::from(label) + &var_suffix))?;
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
            &mut transaction,
        )
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_node_update_mutation_input(
    label: &str,
    ids: &[String],
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
    trace!(
        "visit_node_update_mutation_input called -- label: {}, ids: {:#?}, info.name: {}, input: {:#?}",
        label,
        ids,
        info.name,
        input
    );

    let itd = info.get_type_def()?;

    let mut props = BTreeMap::new();

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

        let statement = Statement::new(
            String::from("MATCH (n:")
                + label
                + ")\n"
                + "WHERE n.id IN $ids\n"
                + "SET n += $props\n"
                + "RETURN n\n",
        )
        .with_param("ids", &ids)?
        .with_param("props", &props)?;

        debug!(
            "visit_node_update_mutation_input Query statement: {:#?}",
            statement
        );
        let raw_results = transaction.exec(statement);
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
                                &extract_ids(&results, "n")?,
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
                            &extract_ids(&results, "n")?,
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

pub fn visit_rel_change_input(
    src_label: &str,
    src_ids: &[String],
    rel_name: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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

pub fn visit_rel_create_input(
    src_label: &str,
    rel_name: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    mut transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
    trace!(
        "visit_node_create_input called -- info.name: {}, rel_name {}, input: {:#?}",
        info.name,
        rel_name,
        input
    );

    let mut sg = SuffixGenerator::new();
    let itd = info.get_type_def()?;

    if let Value::Object(ref m) = input {
        let var_suffix = sg.get_suffix();
        let mut params = BTreeMap::new();

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

        let mut statement = Statement::new(query);
        statement.set_parameters(&params)?;
        debug!("Query: {:#?}", statement);
        let raw_results = transaction.exec(statement);
        debug!("Raw result: {:#?}", raw_results);
        let results = raw_results?;
        let ids = extract_ids(&results, &(String::from(src_label) + &var_suffix))?;
        trace!("IDs for update: {:#?}", ids);

        let create_input = m.get("create").ok_or_else(|| {
            Error::new(
                ErrorKind::MissingProperty("input::create".to_owned(), None),
                None,
            )
        })?;

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
                &mut transaction,
            ),
            Value::Array(create_input_array) => {
                let mut results = CypherResult {
                    columns: vec![
                        "a".to_string(),
                        "r".to_string(),
                        "b".to_string(),
                        "b_label".to_string(),
                    ],
                    data: vec![],
                };
                for create_input_value in create_input_array {
                    let r = visit_rel_create_mutation_input(
                        src_label,
                        &ids,
                        rel_name,
                        &Info::new(
                            itd.get_prop("create")?.type_name.to_owned(),
                            info.type_defs.clone(),
                        ),
                        create_input_value,
                        validators,
                        &mut transaction,
                    );

                    let data = r?.data;
                    results.data.extend(data);
                }
                Ok(results)
            }
            _ => Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into()),
        }
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_create_mutation_input(
    src_label: &str,
    src_ids: &[String],
    rel_name: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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

        let mut props = BTreeMap::new();
        if let Some(Value::Object(pm)) = m.get("props") {
            for (k, v) in pm.iter() {
                props.insert(k.to_owned(), v.clone());
            }
        }

        let statement = Statement::new(
            String::from("MATCH (a:")
                + src_label
                + "),(b:"
                + &dst_label
                + ")"
                + "\n"
                + "WHERE a.id IN $aid AND b.id IN $bid\n"
                + "CREATE (a)-[r:"
                + String::from(rel_name).as_str()
                + " { id: randomUUID() }]->(b)\n"
                + "SET r += $props\n"
                + "RETURN a, r, b, labels(b) as b_label\n",
        )
        .with_param("aid", &src_ids)?
        .with_param("bid", &dst_ids)?
        .with_param("props", &props)?;

        debug!(
            "visit_rel_create_mutation_input Query statement: {:#?}",
            statement
        );
        let results = transaction.exec(statement)?;
        debug!(
            "visit_rel_create_mutation_input Query results: {:#?}",
            results
        );

        Ok(results)
    } else {
        Err(Error::new(ErrorKind::InputTypeMismatch(info.name.to_owned()), None).into())
    }
}

pub fn visit_rel_delete_input(
    src_label: &str,
    src_ids_opt: Option<&[String]>,
    rel_name: &str,
    info: &Info,
    input: &Value,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
    let mut params = BTreeMap::new();

    if let Value::Object(m) = input {
        let query = visit_rel_query_input(
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

        let mut read_statement = Statement::new(query);
        read_statement.set_parameters(&params)?;
        debug!("visit_rel_delete_input Query {:#?}", read_statement);
        let raw_read_results = transaction.exec(read_statement);
        debug!("visit_rel_delete_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        let mut del_statement = Statement::new(
            String::from("MATCH (a:")
                + src_label
                + ")-[r:"
                + rel_name
                + "]->()\n"
                + "WHERE r.id IN $rids\n"
                + "DELETE r\n"
                + "RETURN count(*) as count\n",
        );
        del_statement.set_parameters(&params)?;
        del_statement.add_param(
            "rids",
            &extract_ids(
                &read_results,
                &(String::from(rel_name) + &src_suffix + &dst_suffix),
            )?,
        )?;
        debug!("visit_rel_delete_input Query {:#?}", del_statement);
        let raw_del_results = transaction.exec(del_statement);
        debug!("visit_rel_delete_input Raw result: {:#?}", raw_del_results);

        let del_results = raw_del_results?;

        if let Some(src) = m.get("src") {
            visit_rel_src_delete_mutation_input(
                src_label,
                &extract_ids(&read_results, &(src_label.to_string() + &src_suffix))?,
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
                &extract_ids(&read_results, &(String::from("dst") + &dst_suffix))?,
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

pub fn visit_rel_dst_delete_mutation_input(
    ids: &[String],
    info: &Info,
    input: &Value,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
    params: &mut BTreeMap<String, BTreeMap<String, Value>>,
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

pub fn visit_rel_dst_update_mutation_input(
    ids: &[String],
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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

fn visit_rel_nodes_mutation_input_union(
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<(String, Vec<String>), FieldError> {
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
    params: &mut BTreeMap<String, BTreeMap<String, Value>>,
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

    let mut props = BTreeMap::new();
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
        let mut id_map = BTreeMap::new();
        id_map.insert(
            "ids".to_string(),
            src_ids
                .iter()
                .map(|s| Value::String(s.to_owned()))
                .collect(),
        );

        params.insert(
            String::from(rel_name) + src_suffix + dst_suffix + "_srcids",
            id_map,
        );
    }

    if let Some(wcs) = wc {
        qs.push_str(&(String::from(&wcs) + "\n"));
    }
    params.insert(String::from(rel_name) + src_suffix + dst_suffix, props);

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

pub fn visit_rel_src_delete_mutation_input(
    label: &str,
    ids: &[String],
    info: &Info,
    input: &Value,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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

pub fn visit_rel_src_update_mutation_input(
    label: &str,
    ids: &[String],
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
    params: &mut BTreeMap<String, BTreeMap<String, Value>>,
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

pub fn visit_rel_update_input(
    src_label: &str,
    src_ids: Option<&[String]>,
    rel_name: &str,
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
    let mut params = BTreeMap::new();
    let src_suffix = sg.get_suffix();
    let dst_suffix = sg.get_suffix();

    if let Value::Object(m) = input {
        let query = visit_rel_query_input(
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

        let mut read_statement = Statement::new(query);
        read_statement.set_parameters(&params)?;
        debug!("visit_rel_update_input Query {:#?}", read_statement);
        let raw_read_results = transaction.exec(read_statement);
        debug!("visit_rel_update_input Raw result: {:#?}", raw_read_results);

        let read_results = raw_read_results?;

        if let Some(update) = m.get("update") {
            visit_rel_update_mutation_input(
                src_label,
                &extract_ids(&read_results, &(String::from(src_label) + &src_suffix))?,
                rel_name,
                &extract_ids(
                    &read_results,
                    &(String::from(rel_name) + &src_suffix + &dst_suffix),
                )?,
                &extract_ids(&read_results, &(String::from("dst") + &dst_suffix))?,
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
fn visit_rel_update_mutation_input(
    src_label: &str,
    src_ids: &[String],
    rel_name: &str,
    rel_ids: &[String],
    dst_ids: &[String],
    info: &Info,
    input: &Value,
    validators: &Validators,
    transaction: &mut Transaction<Started>,
) -> Result<CypherResult, FieldError> {
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
        let mut props = BTreeMap::new();
        if let Some(Value::Object(pm)) = m.get("props") {
            for (k, v) in pm.iter() {
                props.insert(k.to_owned(), v.clone());
            }
        }

        let statement = Statement::new(
            String::from("MATCH (a:")
                + src_label
                + ")-[r:"
                + String::from(rel_name).as_str()
                + "]->(b)\n"
                + "WHERE r.id IN $rids\n"
                + "SET r += $props\n"
                + "RETURN a, r, b, labels(b) as b_label\n",
        )
        .with_param("rids", rel_ids)?
        .with_param("props", &props)?;
        debug!(
            "visit_rel_update_mutation_input Query statement: {:#?}",
            statement
        );
        let results = transaction.exec(statement)?;
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
    validators: &Validators,
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
