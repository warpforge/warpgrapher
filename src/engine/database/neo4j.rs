use super::{env_string, DatabaseEndpoint, DatabasePool, QueryResult};
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::Error;
use juniper::FieldError;
use log::{debug, trace};
use r2d2_cypher::CypherConnectionManager;
use rusted_cypher::cypher::result::CypherResult;
use rusted_cypher::cypher::transaction::{Started, Transaction};
use rusted_cypher::Statement;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

pub struct Neo4jEndpoint {
    db_url: String,
}

impl Neo4jEndpoint {
    pub fn from_env() -> Result<Neo4jEndpoint, Error> {
        Ok(Neo4jEndpoint {
            db_url: env_string("WG_NEO4J_URL")?,
        })
    }
}

impl DatabaseEndpoint for Neo4jEndpoint {
    fn pool(&self) -> Result<DatabasePool, Error> {
        let manager = CypherConnectionManager {
            url: self.db_url.to_owned(),
        };

        Ok(DatabasePool::Neo4j(
            r2d2::Pool::builder()
                .max_size(num_cpus::get().try_into().unwrap_or(8))
                .build(manager)?, // .map_err(|e| Error::new(ErrorKind::CouldNotBuildNeo4jPool(e), None))?,
        ))
    }
}

pub struct Neo4jTransaction<'t> {
    transaction: Option<Transaction<'t, Started>>,
}

impl<'t> Neo4jTransaction<'t> {
    pub fn new(transaction: Transaction<'t, Started>) -> Neo4jTransaction {
        Neo4jTransaction {
            transaction: Some(transaction),
        }
    }
}

impl<'t> super::Transaction for Neo4jTransaction<'t> {
    type ImplQueryResult = Neo4jQueryResult;

    fn begin(&self) -> Result<(), FieldError> {
        debug!("transaction::begin called");
        Ok(())
    }

    fn commit(&mut self) -> Result<(), FieldError> {
        debug!("transaction::commit called");
        if let Some(t) = self.transaction.take() {
            t.commit().map(|_| Ok(()))?
        } else {
            Err(Error::TransactionFinished.into())
        }
    }

    fn create_node<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        let query = String::from("CREATE (n:")
            + label
            + " { id: randomUUID() })\n"
            + "SET n += $props\n"
            + "RETURN n, labels(n) as n_label\n";
        let mut params = HashMap::new();
        params.insert("props".to_owned(), props.into());

        trace!(
            "Neo4jTransaction::create_node query statement query, params: {:#?}, {:#?}",
            query,
            params
        );
        let raw_results = self.exec(&query, partition_key_opt, Some(params));
        trace!(
            "Neo4jTransaction::create_node raw results: {:#?}",
            raw_results
        );
        Ok(raw_results?.nodes(label, info)?.into_iter().next().unwrap())
    }

    fn create_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        src_ids: Value,
        dst_label: &str,
        dst_ids: Value,
        rel_name: &str,
        params: &mut HashMap<String, Value>,
        partition_key_opt: &Option<String>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        let mut props = HashMap::new();
        if let Some(Value::Map(pm)) = params.remove("props") {
            // remove rather than get to take ownership
            for (k, v) in pm.into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        if let (Value::Array(src_id_vec), Value::Array(dst_id_vec)) =
            (src_ids.clone(), dst_ids.clone())
        {
            // TODO remove clone
            let src_td = info.type_def_by_name(src_label)?;
            let src_prop = src_td.property(rel_name)?;

            if !src_prop.list() {
                let check_query = String::from("MATCH (")
                    + src_label
                    + ":"
                    + src_label
                    + ")-["
                    + rel_name
                    + ":"
                    + rel_name
                    + "]->() WHERE "
                    + src_label
                    + ".id IN $aid RETURN COUNT("
                    + rel_name
                    + ") as count";
                let mut check_params: HashMap<String, Value> = HashMap::new();
                check_params.insert("aid".to_owned(), Value::Array(src_id_vec)); // TODO -- remove cloning
                let check_results =
                    self.exec(&check_query, partition_key_opt, Some(check_params))?;
                if check_results.count()? > 0 || dst_id_vec.len() > 1 {
                    return Err(Error::RelDuplicated {
                        rel_name: rel_name.to_string(),
                    }
                    .into());
                }
            }
        }

        let query = String::from("MATCH (")
            + src_label
            + ":"
            + src_label
            + "),(dst:"
            + dst_label
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
            + " as src, "
            + "labels("
            + src_label
            + ") as src_label,"
            + rel_name
            + " as r"
            + ", dst, labels(dst) as dst_label\n";

        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("aid".to_owned(), src_ids);
        params.insert("bid".to_owned(), dst_ids);
        params.insert("props".to_owned(), Value::Map(props));

        debug!(
            "visit_rel_create_mutation_input query, params: {:#?}, {:#?}",
            query, params
        );
        let results = self.exec(&query, partition_key_opt, Some(params))?;
        debug!(
            "visit_rel_create_mutation_input Query results: {:#?}",
            results
        );

        Ok(results.rels(
            src_label,
            "",
            rel_name,
            dst_label,
            "",
            props_type_name,
            info,
        )?)
    }

    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Value,
        partition_key_opt: &Option<String>,
    ) -> Result<i32, FieldError> {
        let query = String::from("MATCH (n:")
            + label
            + ")\n"
            + "WHERE n.id IN $ids\n"
            + "DETACH DELETE n\n"
            + "RETURN count(*) as count\n";
        let mut params = HashMap::new();
        params.insert("ids".to_owned(), ids);

        trace!(
            "Neo4jTransaction::delete_nodes query, params: {:#?}, {:#?}",
            query,
            params
        );
        let results = self.exec(&query, partition_key_opt, Some(params))?;
        trace!(
            "Neo4jTransaction::delete_nodes Query results: {:#?}",
            results
        );

        Ok(results.count()?)
    }

    fn delete_rels(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        _info: &Info,
    ) -> Result<i32, FieldError> {
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

        let mut del_params = HashMap::new();
        del_params.insert("rids".to_owned(), rel_ids);
        debug!(
            "visit_rel_delete_input query, params: {:#?}, {:#?}",
            del_query, del_params
        );
        Ok(self
            .exec(&del_query, partition_key_opt, Some(del_params))?
            .count()?)
    }

    fn exec(
        &mut self,
        query: &str,
        _partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Neo4jQueryResult, FieldError> {
        debug!(
            "transaction::exec called with query, params: {:#?}, {:#?}",
            query, params
        );
        if let Some(transaction) = self.transaction.as_mut() {
            let mut statement = Statement::new(String::from(query));
            if let Some(p) = params {
                for (k, v) in p.into_iter() {
                    statement.add_param::<String, &serde_json::Value>(k, &v.try_into()?)?;
                }
            }
            let result = transaction.exec(statement);
            debug!("transaction::exec result: {:#?}", result);
            Ok(Neo4jQueryResult::new(result?))
        } else {
            Err(Error::TransactionFinished.into())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn node_query_string(
        &mut self,
        // query_string: &str,
        rel_query_fragments: Vec<String>,
        params: &mut HashMap<String, Value>,
        label: &str,
        var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        trace!(
            "transaction::node_query_string called, union_type: {:#?}",
            union_type
        );

        let mut qs = String::new();

        for rqf in rel_query_fragments {
            qs.push_str(&rqf);
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
                            + param_suffix
                            + "."
                            + &k,
                    )
                }
                Some(wcs) => {
                    wc = Some(
                        wcs + " AND " + label + "." + &k + "=$" + label + param_suffix + "." + &k,
                    )
                }
            }
        }
        if let Some(wcs) = wc {
            qs.push_str(&(String::from(&wcs) + "\n"));
        }

        params.insert(String::from(label) + param_suffix, props.into());

        if return_node {
            qs.push_str(
                &(String::from("RETURN ")
                    + label
                    + var_suffix
                    + " as n, labels("
                    + label
                    + var_suffix
                    + ") as n_label\n"),
            );
        }

        Ok(qs)
    }

    fn rel_query_string(
        &mut self,
        // query: &str,
        src_label: &str,
        src_suffix: &str,
        src_ids_opt: Option<Value>,
        src_query_opt: Option<String>,
        rel_name: &str,
        dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
        params: &mut HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        let mut qs = String::new();

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
            let mut id_map = HashMap::new();
            id_map.insert("ids".to_string(), src_ids);

            params.insert(
                String::from(rel_name) + src_suffix + dst_suffix + "_srcids",
                id_map.try_into()?,
            );
        }

        if let Some(wcs) = wc {
            qs.push_str(&(String::from(&wcs) + "\n"));
        }
        params.insert(
            String::from(rel_name) + src_suffix + dst_suffix,
            props.into(),
        );

        if let Some(src_query) = src_query_opt {
            qs.push_str(&src_query);
        }

        if let Some(dst_query) = dst_query_opt {
            qs.push_str(&dst_query);
        }

        if return_rel {
            qs.push_str(
                &(String::from("RETURN ")
                    + src_label
                    + src_suffix
                    + " as src, "
                    + "labels("
                    + src_label
                    + src_suffix
                    + ") as src_label, "
                    + rel_name
                    + src_suffix
                    + dst_suffix
                    + " as r, "
                    + dst_var
                    + dst_suffix
                    + " as dst, "
                    + "labels("
                    + dst_var
                    + dst_suffix
                    + ") as dst_label\n"),
            );
        }

        trace!("visit_rel_query_input -- query_string: {}", qs);
        Ok(qs)
    }

    fn rollback(&mut self) -> Result<(), FieldError> {
        debug!("transaction::rollback called");
        if let Some(t) = self.transaction.take() {
            Ok(t.rollback()?)
        } else {
            Err(Error::TransactionFinished.into())
        }
    }

    fn update_nodes<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        ids: Value,
        props: HashMap<String, Value>,
        partition_key_opt: &Option<String>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("ids".to_owned(), ids);
        params.insert("props".to_owned(), props.into());

        let query = String::from("MATCH (n:")
            + label
            + ")\n"
            + "WHERE n.id IN $ids\n"
            + "SET n += $props\n"
            + "RETURN n, labels(n) as n_label\n";

        trace!("update_nodes query, params: {:#?}, {:#?}", query, params);
        let results = self.exec(&query, partition_key_opt, Some(params));
        trace!("update_nodes Query results: {:#?}", results);

        results?.nodes(label, info)
    }

    fn update_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
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
            + " as src, labels("
            + src_label
            + ") as src_label, "
            + rel_name
            + " as r"
            + ", dst, labels(dst) as dst_label\n";

        let mut params: HashMap<String, Value> = HashMap::new();
        params.insert("rids".to_owned(), rel_ids);
        params.insert("props".to_owned(), props.into());
        debug!(
            "visit_rel_update_mutation_input query, params: {:#?}, {:#?}",
            query, params
        );

        let results = self.exec(&query, partition_key_opt, Some(params))?;
        debug!(
            "visit_rel_update_mutation_input Query results: {:#?}",
            results
        );

        results.rels(src_label, "", rel_name, "", "", props_type_name, info)
    }
}

#[derive(Debug)]
pub struct Neo4jQueryResult {
    result: CypherResult,
}

impl Neo4jQueryResult {
    fn new(result: CypherResult) -> Neo4jQueryResult {
        Neo4jQueryResult { result }
    }
}

impl QueryResult for Neo4jQueryResult {
    fn nodes<GlobalCtx, ReqCtx>(
        self,
        _name: &str,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        ReqCtx: RequestContext,
    {
        trace!("Neo4jQueryResult::nodes called, result: {:#?}", self.result);

        let mut v = Vec::new();
        for row in self.result.rows() {
            let m: HashMap<String, serde_json::Value> = row.get("n")?;
            let mut label_list: Vec<String> = row.get("n_label")?;
            let label = label_list
                .pop()
                .ok_or_else(|| Error::ResponseItemNotFound {
                    name: "label".to_string(),
                })?;
            let mut fields = HashMap::new();
            let type_def = info.type_def_by_name(&label)?;
            for (k, v) in m.into_iter() {
                let prop_def = type_def.property(&k)?;
                if prop_def.list() {
                    if let serde_json::Value::Array(_) = v {
                        fields.insert(k, v.try_into()?);
                    } else {
                        let mut val = Vec::new();
                        val.push(v.try_into()?);
                        fields.insert(k, Value::Array(val));
                    }
                } else {
                    fields.insert(k, v.try_into()?);
                }
            }
            v.push(Node::new(label.to_owned(), fields));
        }
        trace!("Neo4jQueryResults::nodes results: {:#?}", v);
        Ok(v)
    }

    fn rels<GlobalCtx, ReqCtx>(
        self,
        src_name: &str,
        src_suffix: &str,
        rel_name: &str,
        dst_name: &str,
        dst_suffix: &str,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        ReqCtx: RequestContext,
    {
        trace!("Neo4jQueryResult::rels called, src_name, src_suffix, rel_name, dst_name, dst_suffix, props_type_name: {:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?}", src_name, src_suffix, rel_name, dst_name, dst_suffix, props_type_name);

        let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();

        for row in self.result.rows() {
            if let serde_json::Value::Array(src_labels) = row.get("src_label")? {
                if let serde_json::Value::String(src_type) = &src_labels[0] {
                    let src_map: HashMap<String, serde_json::Value> = row.get("src")?;
                    let mut src_label_list: Vec<String> = row.get("src_label")?;
                    let src_label =
                        src_label_list
                            .pop()
                            .ok_or_else(|| Error::ResponseItemNotFound {
                                name: "label".to_string(),
                            })?;
                    let mut src_fields = HashMap::new();
                    let type_def = info.type_def_by_name(&src_label)?;
                    for (k, v) in src_map.into_iter() {
                        let prop_def = type_def.property(&k)?;
                        if prop_def.list() {
                            if let serde_json::Value::Array(_) = v {
                                src_fields.insert(k, v.try_into()?);
                            } else {
                                let mut val = Vec::new();
                                val.push(v.try_into()?);
                                src_fields.insert(k, Value::Array(val));
                            }
                        } else {
                            src_fields.insert(k, v.try_into()?);
                        }
                    }

                    if let serde_json::Value::Array(dst_labels) = row.get("dst_label")? {
                        if let serde_json::Value::String(dst_type) = &dst_labels[0] {
                            let dst_map: HashMap<String, serde_json::Value> = row.get("dst")?;
                            let mut dst_label_list: Vec<String> = row.get("dst_label")?;
                            let dst_label = dst_label_list.pop().ok_or_else(|| {
                                Error::ResponseItemNotFound {
                                    name: "label".to_string(),
                                }
                            })?;
                            let mut dst_fields = HashMap::new();
                            let type_def = info.type_def_by_name(&dst_label)?;
                            for (k, v) in dst_map.into_iter() {
                                let prop_def = type_def.property(&k)?;
                                if prop_def.list() {
                                    if let serde_json::Value::Array(_) = v {
                                        dst_fields.insert(k, v.try_into()?);
                                    } else {
                                        let mut val = Vec::new();
                                        val.push(v.try_into()?);
                                        dst_fields.insert(k, Value::Array(val));
                                    }
                                } else {
                                    dst_fields.insert(k, v.try_into()?);
                                }
                            }

                            v.push(Rel::new(
                                row.get::<serde_json::Value>("r")?
                                    .get("id")
                                    .ok_or_else(|| Error::ResponseItemNotFound {
                                        name: "id".to_string(),
                                    })?
                                    .clone()
                                    .try_into()?,
                                match props_type_name {
                                    Some(p_type_name) => {
                                        let map: HashMap<String, serde_json::Value> =
                                            row.get::<HashMap<String, serde_json::Value>>("r")?;
                                        let mut wg_map = HashMap::new();
                                        for (k, v) in map.into_iter() {
                                            wg_map.insert(k, v.try_into()?);
                                        }

                                        Some(Node::new(p_type_name.to_string(), wg_map))
                                    }
                                    None => None,
                                },
                                Node::new(src_type.to_owned(), src_fields),
                                Node::new(dst_type.to_owned(), dst_fields),
                            ))
                        } else {
                            return Err(Error::TypeNotExpected.into());
                        }
                    } else {
                        return Err(Error::TypeNotExpected.into());
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            } else {
                return Err(Error::TypeNotExpected.into());
            };
        }
        trace!("Neo4jQueryResults::rels results: {:#?}", v);
        Ok(v)
    }

    fn ids(&self, _name: &str) -> Result<Value, FieldError> {
        trace!("Neo4jQueryResult::ids called");

        let mut v = Vec::new();
        for row in self.result.rows() {
            if let Ok(n) = row.get::<serde_json::Map<String, serde_json::Value>>("n") {
                if let serde_json::Value::String(id) =
                    n.get("id").ok_or_else(|| Error::ResponseItemNotFound {
                        name: "id".to_string(),
                    })?
                {
                    v.push(Value::String(id.to_owned()));
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            } else if let Ok(r) = row.get::<serde_json::Map<String, serde_json::Value>>("r") {
                if let serde_json::Value::String(id) =
                    r.get("id").ok_or_else(|| Error::ResponseItemNotFound {
                        name: "id".to_string(),
                    })?
                {
                    v.push(Value::String(id.to_owned()));
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            } else {
                return Err(Error::ResponseItemNotFound {
                    name: "n or r".to_string(),
                }
                .into());
            }
        }

        trace!("ids result: {:#?}", v);
        Ok(Value::Array(v))
    }

    fn count(&self) -> Result<i32, FieldError> {
        trace!("Neo4jQueryResult::count called");

        let ret_row = self
            .result
            .rows()
            .next()
            .ok_or_else(|| Error::ResponseSetNotFound)?;
        let ret_val = ret_row.get("count")?;

        if let serde_json::Value::Number(n) = ret_val {
            if let Some(i_val) = n.as_i64() {
                Ok(i_val as i32)
            } else {
                Err(Error::TypeNotExpected.into())
            }
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }

    fn len(&self) -> i32 {
        trace!("Neo4jQueryResult::len called");
        0
    }

    fn is_empty(&self) -> bool {
        trace!("Neo4jQueryResult::is_empty called");
        self.len() == 0
    }
}
