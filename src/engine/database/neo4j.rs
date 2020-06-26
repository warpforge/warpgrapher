//! Provides database interface types and functions for Neo4J

use super::{env_string, DatabaseEndpoint, DatabasePool};
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::Info;
use crate::engine::schema::NodeType;
use crate::engine::value::Value;
use crate::Error;
use log::{debug, trace};
use r2d2_cypher::CypherConnectionManager;
use rusted_cypher::cypher::result::CypherResult;
use rusted_cypher::cypher::transaction::{Started, Transaction};
use rusted_cypher::Statement;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

/// A Neo4J endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ne = Neo4jEndpoint::from_env()?;
/// #    Ok(())
/// # }
/// ```
pub struct Neo4jEndpoint {
    db_url: String,
}

impl Neo4jEndpoint {
    /// Reads an variable to construct a [`Neo4jEndpoint`]. The environment variable is
    ///
    /// * WG_NEO4J_URL - the connection URL for the Neo4J DB. For example,
    /// `http://neo4j:testpass@localhost:7474/db/data`
    ///
    /// [`Neo4jEndpoint`]: ./struct.Neo4jEndpoint.html
    ///
    /// # Errors
    ///
    /// * [`EnvironmentVariableNotFound`] - if an environment variable does not exist
    ///
    /// [`EnvironmentVariableNotFound`]: ../../enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ne = Neo4jEndpoint::from_env()?;
    ///     # Ok(())
    /// # }
    /// ```
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
                .build(manager)?,
        ))
    }
}

pub(crate) struct Neo4jTransaction<'t> {
    transaction: Option<Transaction<'t, Started>>,
}

impl<'t> Neo4jTransaction<'t> {
    pub fn new(transaction: Transaction<'t, Started>) -> Neo4jTransaction {
        Neo4jTransaction {
            transaction: Some(transaction),
        }
    }

    fn add_node_return(mut query: String, node_var: &str) -> String {
        query.push_str(
            &("RETURN ".to_string()
                + node_var
                + " as node, labels("
                + node_var
                + ") as node_labels\n"),
        );
        query
    }

    fn add_rel_return(mut query: String, src_var: &str, rel_var: &str, dst_var: &str) -> String {
        query.push_str(
            &("RETURN ".to_string()
                + src_var
                + ".id as src_id, labels("
                + src_var
                + ") as src_labels, "
                + rel_var
                + " as rel, "
                + dst_var
                + ".id as dst_id, labels("
                + dst_var
                + ") as dst_labels\n"),
        );
        query
    }

    fn extract_count(results: CypherResult) -> Result<i32, Error> {
        trace!("Neo4jTransaction::extract_count called");
        if let serde_json::Value::Number(n) = results
            .rows()
            .next()
            .ok_or_else(|| Error::ResponseSetNotFound)?
            .get("count")?
        {
            if let Some(i) = n.as_i64() {
                Ok(i32::try_from(i)?)
            } else {
                Err(Error::TypeConversionFailed {
                    src: format!("{:#?}", n),
                    dst: "i32".to_string(),
                })
            }
        } else {
            Err(Error::ResponseItemNotFound {
                name: "count".to_string(),
            })
        }
    }

    fn extract_node_properties(
        props: HashMap<String, serde_json::Value>,
        type_def: &NodeType,
    ) -> Result<HashMap<String, Value>, Error> {
        props
            .into_iter()
            .map(|(k, v)| {
                if type_def.property(&k)?.list() {
                    if let serde_json::Value::Array(_) = v {
                        Ok((k, v.try_into()?))
                    } else {
                        Ok((k, Value::Array(vec![(v.try_into()?)])))
                    }
                } else {
                    Ok((k, v.try_into()?))
                }
            })
            .collect::<Result<HashMap<String, Value>, Error>>()
    }

    fn nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        results: CypherResult,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::nodes called");
        results
            .rows()
            .map(|row| {
                let label = row
                    .get::<Vec<String>>("node_labels")?
                    .pop()
                    .ok_or_else(|| Error::ResponseItemNotFound {
                        name: "node_labels".to_string(),
                    })?;
                Ok(Node::new(
                    label.to_string(),
                    Neo4jTransaction::extract_node_properties(
                        row.get("node")?,
                        info.type_def_by_name(&label)?,
                    )?,
                ))
            })
            .collect()
    }

    fn rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        results: CypherResult,
        partition_key_opt: Option<&Value>,
        props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::rels called");
        results
            .rows()
            .map(|row| {
                let src_label = row.get::<Vec<String>>("src_labels")?.pop().ok_or_else(|| {
                    Error::ResponseItemNotFound {
                        name: "src_labels".to_string(),
                    }
                })?;
                let dst_label = row.get::<Vec<String>>("dst_labels")?.pop().ok_or_else(|| {
                    Error::ResponseItemNotFound {
                        name: "dst_labels".to_string(),
                    }
                })?;
                let props = row
                    .get::<HashMap<String, serde_json::Value>>("rel")?
                    .into_iter()
                    .map(|(k, v)| Ok((k, v.try_into()?)))
                    .collect::<Result<HashMap<String, Value>, Error>>()?;

                Ok(Rel::new(
                    row.get::<serde_json::Value>("rel")?
                        .get("id")
                        .ok_or_else(|| Error::ResponseItemNotFound {
                            name: "id".to_string(),
                        })?
                        .clone()
                        .try_into()?,
                    partition_key_opt.cloned(),
                    props_type_name.map(|ptn| Node::new(ptn.to_string(), props)),
                    NodeRef::new(
                        row.get::<serde_json::Value>("src_id")?.try_into()?,
                        src_label,
                    ),
                    NodeRef::new(
                        row.get::<serde_json::Value>("dst_id")?.try_into()?,
                        dst_label,
                    ),
                ))
            })
            .collect()
    }

    fn single_rel_check<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        transaction: &mut Transaction<'t, Started>,
        src_label: &str,
        src_ids: Vec<Value>,
        dst_ids: Vec<Value>,
        rel_name: &str,
        _partition_key_opt: Option<&Value>,
    ) -> Result<(), Error> {
        let query = "MATCH (src:".to_string()
            + src_label
            + ")-[rel:"
            + rel_name
            + "]->() WHERE src.id IN $src_ids RETURN COUNT(rel) as count";

        let mut statement = Statement::new(query);
        statement.add_param::<String, &serde_json::Value>(
            "src_ids".to_string(),
            &Value::Array(src_ids).try_into()?,
        )?;

        let results = transaction.exec(statement)?;
        if Neo4jTransaction::extract_count(results)? > 0 || dst_ids.len() > 1 {
            Err(Error::RelDuplicated {
                rel_name: rel_name.to_string(),
            })
        } else {
            Ok(())
        }
    }
}

impl<'t> super::Transaction for Neo4jTransaction<'t> {
    fn begin(&self) -> Result<(), Error> {
        debug!("transaction::begin called");
        Ok(())
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        _partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let mut query = "CREATE (node:".to_string()
                + label
                + " { id: randomUUID() })\n"
                + "SET node += $props\n";
            query = Neo4jTransaction::add_node_return(query, "node");

            let mut params: HashMap<String, Value> = HashMap::new();
            params.insert("props".to_owned(), props.into());

            let mut statement = Statement::new(query);
            params.into_iter().try_for_each(|(k, v)| {
                statement
                    .add_param::<String, &serde_json::Value>(k, &v.try_into()?)
                    .map_err(Error::from)
            })?;

            let result = transaction.exec(statement)?;
            Neo4jTransaction::nodes(result, info)?
                .into_iter()
                .next()
                .ok_or_else(|| Error::ResponseSetNotFound)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_label: &str,
        src_ids: Vec<Value>,
        dst_label: &str,
        dst_ids: Vec<Value>,
        rel_name: &str,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::create_rels called -- src_label: {}, src_ids: {:#?}, dst_label: {}, dst_ids: {:#?}, rel_name: {}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", src_label, src_ids, dst_label, dst_ids, rel_name, props, props_type_name, partition_key_opt);

        if let Some(transaction) = self.transaction.as_mut() {
            let src_td = info.type_def_by_name(src_label)?;
            let src_prop = src_td.property(rel_name)?;

            if !src_prop.list() {
                Neo4jTransaction::single_rel_check::<GlobalCtx, RequestCtx>(
                    transaction,
                    src_label,
                    src_ids.clone(),
                    dst_ids.clone(),
                    rel_name,
                    partition_key_opt,
                )?;
            }

            let mut query = "MATCH (src:".to_string()
                + src_label
                + "),(dst:"
                + dst_label
                + ")\n"
                + "WHERE src.id IN $src_ids AND dst.id IN $dst_ids\n"
                + "CREATE (src)-[rel:"
                + rel_name
                + " { id: randomUUID() }]->(dst)\n"
                + "SET rel += $props\n";
            query = Neo4jTransaction::add_rel_return(query, "src", "rel", "dst");

            trace!("Neo4jTransaction::create_rels -- query: {}, src_ids: {:#?}, dst_ids: {:#?}, props: {:#?}", query, src_ids, dst_ids, props);
            let mut statement = Statement::new(query);
            statement.add_param::<String, &serde_json::Value>(
                "src_ids".to_owned(),
                &Value::Array(src_ids).try_into()?,
            )?;
            statement.add_param::<String, &serde_json::Value>(
                "dst_ids".to_owned(),
                &Value::Array(dst_ids).try_into()?,
            )?;
            statement.add_param::<String, &serde_json::Value>(
                "props".to_owned(),
                &Into::<Value>::into(props).try_into()?,
            )?;

            trace!("statement: {:#?}", statement);
            let results = transaction.exec(statement)?;
            Neo4jTransaction::rels(results, partition_key_opt, props_type_name)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn node_query(
        &mut self,
        rel_query_fragments: Vec<String>,
        mut params: HashMap<String, Value>,
        label: &str,
        var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        let mut query = String::new();

        for rqf in rel_query_fragments {
            query.push_str(&rqf);
        }

        if union_type {
            query.push_str(&("MATCH (".to_string() + label + var_suffix + ")\n"));
        } else {
            query.push_str(&("MATCH (".to_string() + label + var_suffix + ":" + label + ")\n"));
        }

        if !props.is_empty() {
            query = props.keys().enumerate().fold(query, |mut query, (i, k)| {
                if i == 0 {
                    query.push_str("WHERE ");
                } else {
                    query.push_str(" AND ");
                }

                query.push_str(
                    &(label.to_string()
                        + var_suffix
                        + "."
                        + &k
                        + "=$"
                        + label
                        + param_suffix
                        + "."
                        + &k),
                );

                query
            });

            query.push_str("\n");
        }
        params.insert(label.to_string() + param_suffix, props.into());

        if return_node {
            query = Neo4jTransaction::add_node_return(query, &(label.to_string() + var_suffix));
        }

        Ok((query, params))
    }

    fn rel_query(
        &mut self,
        mut params: HashMap<String, Value>,
        src_label: &str,
        src_suffix: &str,
        src_ids_opt: Option<Vec<Value>>,
        src_query_opt: Option<String>,
        rel_name: &str,
        dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        let mut query = "MATCH (".to_string()
            + src_label
            + src_suffix
            + ":"
            + src_label
            + ")-["
            + rel_name
            + src_suffix
            + dst_suffix
            + ":"
            + rel_name
            + "]->("
            + dst_var
            + dst_suffix
            + ")";

        let empty_props = props.is_empty();
        if !empty_props {
            query = props.keys().enumerate().fold(query, |mut query, (i, k)| {
                if i == 0 {
                    query.push_str("\nWHERE ");
                } else {
                    query.push_str(" AND ");
                }

                query.push_str(
                    &(rel_name.to_string()
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k
                        + " = $"
                        + rel_name
                        + src_suffix
                        + dst_suffix
                        + "."
                        + &k),
                );

                query
            });

            params.insert(rel_name.to_string() + src_suffix + dst_suffix, props.into());
        }

        if let Some(src_ids) = src_ids_opt {
            if empty_props {
                query.push_str("\nWHERE ");
            } else {
                query.push_str(" AND ");
            }

            query.push_str(
                &(src_label.to_string()
                    + src_suffix
                    + ".id IN $"
                    + rel_name
                    + src_suffix
                    + dst_suffix
                    + "_srcids"
                    + "."
                    + "ids"),
            );

            let mut id_map = HashMap::new();
            id_map.insert("ids".to_string(), Value::Array(src_ids));
            params.insert(
                rel_name.to_string() + src_suffix + dst_suffix + "_srcids",
                id_map.into(),
            );
        }
        query.push_str("\n");

        if let Some(src_query) = src_query_opt {
            query.push_str(&src_query);
        }

        if let Some(dst_query) = dst_query_opt {
            query.push_str(&dst_query);
        }

        if return_rel {
            query = Neo4jTransaction::add_rel_return(
                query,
                &(src_label.to_string() + src_suffix),
                &(rel_name.to_string() + src_suffix + dst_suffix),
                &(dst_var.to_string() + dst_suffix),
            );
        }

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: &str,
        _partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let mut statement = Statement::new(query.to_string());
            if let Some(params) = params_opt {
                params.into_iter().try_for_each(|(k, v)| {
                    statement
                        .add_param::<String, &serde_json::Value>(k, &v.try_into()?)
                        .map_err(Error::from)
                })?
            }
            let results = transaction.exec(statement)?;
            Neo4jTransaction::nodes(results, info)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: &str,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let mut statement = Statement::new(query.to_string());
            if let Some(params) = params_opt {
                params.into_iter().try_for_each(|(k, v)| {
                    statement
                        .add_param::<String, &serde_json::Value>(k, &v.try_into()?)
                        .map_err(Error::from)
                })?
            }
            let results = transaction.exec(statement)?;
            Neo4jTransaction::rels(results, partition_key_opt, props_type_name)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        ids: Vec<Value>,
        props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let mut query = "MATCH (node:".to_string()
                + label
                + ")\n"
                + "WHERE node.id IN $ids\n"
                + "SET node += $props\n";
            query = Neo4jTransaction::add_node_return(query, "node");

            let mut statement = Statement::new(query);
            statement.add_param::<String, &serde_json::Value>(
                "ids".to_string(),
                &Value::Array(ids).try_into()?,
            )?;
            statement.add_param::<String, &serde_json::Value>(
                "props".to_string(),
                &Value::Map(props).try_into()?,
            )?;

            let results = transaction.exec(statement)?;

            Neo4jTransaction::nodes(results, info)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Vec<Value>,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let mut query = "MATCH (src:".to_string()
                + src_label
                + ")-[rel:"
                + rel_name
                + "]->(dst)\n"
                + "WHERE rel.id IN $ids\n"
                + "SET rel += $props\n";
            query = Neo4jTransaction::add_rel_return(query, "src", "rel", "dst");

            let mut statement = Statement::new(query);
            statement.add_param::<String, &serde_json::Value>(
                "ids".to_string(),
                &Value::Array(rel_ids).try_into()?,
            )?;
            statement.add_param::<String, &serde_json::Value>(
                "props".to_string(),
                &Value::Map(props).try_into()?,
            )?;

            let results = transaction.exec(statement)?;

            Neo4jTransaction::rels(results, partition_key_opt, props_type_name)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Vec<Value>,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let query = "MATCH (node:".to_string()
                + label
                + ")\n"
                + "WHERE node.id IN $ids\n"
                + "DETACH DELETE node\n"
                + "RETURN count(*) as count\n";

            let mut statement = Statement::new(query);
            statement.add_param::<String, &serde_json::Value>(
                "ids".to_string(),
                &Value::Array(ids).try_into()?,
            )?;

            let results = transaction.exec(statement)?;
            Neo4jTransaction::extract_count(results)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn delete_rels(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Vec<Value>,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        if let Some(transaction) = self.transaction.as_mut() {
            let del_query = "MATCH (src:".to_string()
                + src_label
                + ")-[rel:"
                + rel_name
                + "]->()\n"
                + "WHERE "
                + "rel.id IN $ids\n"
                + "DELETE rel\n"
                + "RETURN count(*) as count\n";

            let mut statement = Statement::new(del_query);
            statement.add_param::<String, &serde_json::Value>(
                "ids".to_string(),
                &Value::Array(rel_ids).try_into()?,
            )?;

            let results = transaction.exec(statement)?;
            Neo4jTransaction::extract_count(results)
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn commit(&mut self) -> Result<(), Error> {
        debug!("transaction::commit called");
        if let Some(t) = self.transaction.take() {
            t.commit().map(|_| Ok(()))?
        } else {
            Err(Error::TransactionFinished)
        }
    }

    fn rollback(&mut self) -> Result<(), Error> {
        debug!("transaction::rollback called");
        if let Some(t) = self.transaction.take() {
            Ok(t.rollback()?)
        } else {
            Err(Error::TransactionFinished)
        }
    }
}
