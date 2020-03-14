use super::{get_env_var, DatabaseEndpoint, DatabasePool, QueryResult};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
use crate::{Error, ErrorKind};
use juniper::FieldError;
use log::trace;
use r2d2_cypher::CypherConnectionManager;
use rusted_cypher::cypher::result::CypherResult;
use rusted_cypher::cypher::transaction::{Started, Transaction};
use rusted_cypher::Statement;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

pub struct Neo4jEndpoint {
    db_url: String,
}

impl Neo4jEndpoint {
    pub fn from_env() -> Result<Neo4jEndpoint, Error> {
        Ok(Neo4jEndpoint {
            db_url: get_env_var("WG_NEO4J_URL")?,
        })
    }
}

impl DatabaseEndpoint for Neo4jEndpoint {
    fn get_pool(&self) -> Result<DatabasePool, Error> {
        let manager = CypherConnectionManager {
            url: self.db_url.to_owned(),
        };

        Ok(DatabasePool::Neo4j(
            r2d2::Pool::builder()
                .max_size(num_cpus::get().try_into().unwrap_or(8))
                .build(manager)
                .map_err(|e| Error::new(ErrorKind::CouldNotBuildDatabasePool(e), None))?,
        ))
    }
}

/*
#[derive(Clone, Debug)]
pub struct Neo4jPool {
    pool: Pool<CypherConnectionManager>,
}
*/

// impl DatabasePool for Pool<CypherConnectionManager> {
/*
type ImplClient = GraphClient;
fn get_client(&self) -> Result<GraphClient, FieldError> {
    Ok( *self.get()?)
}
*/
// }

/*
pub struct Neo4jClient<'t> {
    client: PooledConnection<CypherConnectionManager>,
    _t: PhantomData<Neo4jTransaction<'t>>,
}
*/

/*
impl DatabaseClient for GraphClient {
    type ImplTransaction = Transaction;
    fn get_transaction(&self) -> Result<Self::ImplTransaction, FieldError> {
        Ok( self.transaction().begin()?.0)
    }
}
*/

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
        trace!("transaction::begin called");
        Ok(())
    }
    fn commit(&mut self) -> Result<(), FieldError> {
        trace!("transaction::commit called");
        if let Some(t) = self.transaction.take() {
            t.commit().map(|_| Ok(()))?
        } else {
            Err(Error::new(ErrorKind::TransactionFinished, None).into())
        }
    }
    fn exec<V>(
        &mut self,
        query: &str,
        params: Option<&HashMap<String, V>>,
    ) -> Result<Neo4jQueryResult, FieldError>
    where
        V: Debug + Serialize,
    {
        trace!(
            "transaction::exec called with query, params: {:#?}, {:#?}",
            query,
            params
        );
        if let Some(transaction) = self.transaction.as_mut() {
            let mut statement = Statement::new(String::from(query));
            if let Some(p) = params {
                for (k, v) in p.iter() {
                    statement.add_param::<String, _>(k.into(), v)?;
                }
            }
            let result = transaction.exec(statement);
            trace!("transaction::exec result: {:#?}", result);
            Ok(Neo4jQueryResult::new(result?))
        } else {
            Err(Error::new(ErrorKind::TransactionFinished, None).into())
        }
    }

    fn rollback(&mut self) -> Result<(), FieldError> {
        trace!("transaction::rollback called");
        if let Some(t) = self.transaction.take() {
            Ok(t.rollback()?)
        } else {
            Err(Error::new(ErrorKind::TransactionFinished, None).into())
        }
    }
}

#[derive(Debug)]
pub struct Neo4jQueryResult {
    result: CypherResult,
}

impl Neo4jQueryResult {
    pub fn new(result: CypherResult) -> Neo4jQueryResult {
        Neo4jQueryResult { result }
    }
}

impl QueryResult for Neo4jQueryResult {
    fn get_nodes<GlobalCtx, ReqCtx>(
        &self,
        name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        trace!("Neo4jQueryResult::get_nodes called");

        let mut v = Vec::new();
        for row in self.result.rows() {
            v.push(Node::new(name.to_owned(), row.get(name)?))
        }
        Ok(v)
    }

    fn get_rels<GlobalCtx, ReqCtx>(
        &self,
        src_name: &str,
        src_suffix: &str,
        rel_name: &str,
        dst_name: &str,
        dst_suffix: &str,
        props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        trace!("Neo4jQueryResult::get_rels called, src_name, src_suffix, rel_name, dst_name, dst_suffix, props_type_name: {:#?}, {:#?}, {:#?}, {:#?}, {:#?}, {:#?}", src_name, src_suffix, rel_name, dst_name, dst_suffix, props_type_name);

        let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();

        for row in self.result.rows() {
            if let Value::Array(labels) =
                row.get(&(String::from(dst_name) + dst_suffix + "_label"))?
            {
                if let Value::String(dst_type) = &labels[0] {
                    v.push(Rel::new(
                        row.get::<Value>(&(String::from(rel_name) + src_suffix + dst_suffix))?
                            .get("id")
                            .ok_or_else(|| {
                                Error::new(ErrorKind::MissingResultElement("id".to_string()), None)
                            })?
                            .to_owned(),
                        match props_type_name {
                            Some(p_type_name) => Some(Node::new(
                                p_type_name.to_string(),
                                row.get(&(String::from(rel_name) + src_suffix + dst_suffix))?,
                            )),
                            None => None,
                        },
                        Node::new(
                            src_name.to_owned(),
                            row.get(&(String::from(src_name) + src_suffix))?,
                        ),
                        Node::new(
                            dst_type.to_owned(),
                            row.get(&(String::from(dst_name) + dst_suffix))?,
                        ),
                    ))
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidPropertyType(
                            String::from(dst_name) + dst_suffix + "_label",
                        ),
                        None,
                    )
                    .into());
                }
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidPropertyType(String::from(dst_name) + dst_suffix + "_label"),
                    None,
                )
                .into());
            };
        }
        Ok(v)
    }

    fn get_ids(&self, name: &str) -> Result<Vec<String>, FieldError> {
        trace!("Neo4jQueryResult::get_ids called");

        let mut v = Vec::new();
        for row in self.result.rows() {
            let n: Value = row.get(name)?;
            if let Value::String(id) = n.get("id").ok_or_else(|| Error::new(ErrorKind::MissingProperty("id".to_owned(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None))?
        {v.push(id.to_owned());
        } else {
            return Err(Error::new(ErrorKind::InvalidPropertyType("id".to_owned()), None).into());
        }
        }

        trace!("get_ids result: {:#?}", v);
        Ok(v)
    }

    fn get_count(&self) -> Result<i32, FieldError> {
        trace!("Neo4jQueryResult::get_count called");

        let ret_row = self
            .result
            .rows()
            .next()
            .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;
        let ret_val = ret_row
            .get("count")
            .map_err(|_| Error::new(ErrorKind::MissingResultElement("count".to_owned()), None))?;

        if let Value::Number(n) = ret_val {
            if let Some(i_val) = n.as_i64() {
                Ok(i_val as i32)
            } else {
                Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
            }
        } else {
            Err(Error::new(ErrorKind::InvalidPropertyType("int".to_owned()), None).into())
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

// let mut transaction = graph.transaction()?.begin()?.0;

// For the implementation of get_nodes:
// let mut v: Vec<Node<GlobalCtx, ReqCtx>> = Vec::new();
// for row in results.rows() {
//     v.push(Node::new(
//         p.type_name.to_owned(),
//         row.get(&(p.type_name.to_owned() + &var_suffix))
//             .ok_or_else(|| {
//                 Error::new(
//                     ErrorKind::MissingResultElement(
//                         String::from(p.type_name) + &var_suffix,
//                     ),
//                     None,
//                 )
//             })?
//             .to_owned(),
//     ))
// }

// For the implementation of get_node:
// let row = results
//     .rows()
//     .iter()
//     .nth(0)
//     .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;
// executor.resolve(
//     &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
//     &Node::new(
//         p.type_name.to_owned(),
//         row.get(&(p.type_name.to_owned() + &var_suffix))
//             .ok_or_else(|| {
//                 Error::new(
//                     ErrorKind::MissingResultElement(
//                         String::from(p.type_name) + &var_suffix,
//                     ),
//                     None,
//                 )
//             })?
//             .to_owned(),
//     ),
// )

// For the implementation of get_rels:
// let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
//
// for row in results.rows() {
//     if let Value::Array(labels) =
//         row.get(&(String::from(&dst_prop.type_name) + &dst_suffix + "_label"))?
//     {
//         if let Value::String(dst_type) = &labels[0] {
//             v.push(Rel::new(
//                 row.get::<Value>(&(String::from(rel_name) + &src_suffix + &dst_suffix))?
//                     .get("id")
//                     .ok_or_else(|| {
//                         Error::new(ErrorKind::MissingResultElement("id".to_string()), None)
//                     })?
//                     .to_owned(),
//                 match &props_prop {
//                     Ok(p) => Some(Node::new(
//                         p.type_name.to_owned(),
//                         row.get(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
//                     )),
//                     Err(_e) => None,
//                 },
//                 Node::new(
//                     src_prop.type_name.to_owned(),
//                     row.get(&(String::from(&src_prop.type_name) + &src_suffix))?,
//                 ),
//                 Node::new(
//                     dst_type.to_owned(),
//                     row.get(&(String::from(&dst_prop.type_name) + &dst_suffix))?,
//                 ),
//             ))
//         } else {
//             return Err(Error::new(
//                 ErrorKind::InvalidPropertyType(
//                     String::from(&dst_prop.type_name) + &dst_suffix + "_label",
//                 ),
//                 None,
//             )
//             .into());
//         }
//     } else {
//         return Err(Error::new(
//             ErrorKind::InvalidPropertyType(
//                 String::from(&dst_prop.type_name) + &dst_suffix + "_label",
//             ),
//             None,
//         )
//         .into());
//     };
// }
//
// executor.resolve(
//     &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
//     &v,
// )

// For the implementation of get_rel
//     let row = results
//         .rows()
//         .nth(0)
//         .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;
//
//     if let Value::Array(labels) =
//         row.get(&(String::from(&dst_prop.type_name) + &dst_suffix + "_label"))?
//     {
//         if let Value::String(dst_type) = &labels[0] {
//             executor.resolve(
//                 &Info::new(p.type_name.to_owned(), info.type_defs.clone()),
//                 &Rel::new(
//                     row.get::<Value>(&(String::from(rel_name) + &src_suffix + &dst_suffix))?
//                         .get("id")
//                         .ok_or_else(|| {
//                             Error::new(ErrorKind::MissingProperty("id".to_string(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None)
//                         })?
//                         .to_owned(),
//                     match props_prop {
//                         Ok(pp) => Some(Node::new(
//                             pp.type_name.to_owned(),
//                             row.get(&(String::from(rel_name) + &src_suffix + &dst_suffix))?,
//                         )),
//                         Err(_e) => None,
//                     },
//                     Node::new(
//                         src_prop.type_name.to_owned(),
//                         row.get(&(String::from(&src_prop.type_name) + &src_suffix))?,
//                     ),
//                     Node::new(
//                         dst_type.to_string(),
//                         row.get(&(String::from(&dst_prop.type_name) + &dst_suffix))?,
//                     ),
//                 ),
//             )
//         } else {
//             Err(Error::new(
//                 ErrorKind::InvalidPropertyType(
//                     String::from(&dst_prop.type_name) + &dst_suffix + "_label",
//                 ),
//                 None,
//             )
//             .into())
//         }
//     } else {
//         Err(Error::new(
//             ErrorKind::InvalidPropertyType(
//                 String::from(&dst_prop.type_name) + &dst_suffix + "_label",
//             ),
//             None,
//         )
//         .into())
//     }
// }

// fn extract_ids(results: &CypherResult, name: &str) -> Result<Vec<String>, FieldError> {
//     trace!("extract_ids called -- name: {}", name);

//     let mut v = Vec::new();
//     for row in results.rows() {
//         let n: Value = row.get(name)?;
//         if let Value::String(id) = n
//             .get("id")
//             .ok_or_else(|| Error::new(ErrorKind::MissingProperty("id".to_owned(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None))?
//         {
//             v.push(id.to_owned());
//         } else {
//             return Err(Error::new(ErrorKind::InvalidPropertyType("id".to_owned()), None).into());
//         }
//     }

//     trace!("extract_ids ids: {:#?}", v);
//     Ok(v)
// }

// For implementation of get_count:
// let ret_row = results
//     .rows()
//     .nth(0)
//     .ok_or_else(|| Error::new(ErrorKind::MissingResultSet, None))?;

// let ret_val = ret_row
//     .get("count")
//     .map_err(|_| Error::new(ErrorKind::MissingResultElement("count".to_owned()), None))?;

// let mut v: Vec<Rel<GlobalCtx, ReqCtx>> = Vec::new();
// for row in results.rows() {
//     if let Value::Array(labels) = row.get("b_label")? {
//         if let Value::String(dst_type) = &labels[0] {
//             v.push(Rel::new(
//                 row.get::<Value>("r")?
//                     .get("id")
//                     .ok_or_else(|| {
//                         Error::new(ErrorKind::MissingProperty("id".to_string(), Some("This is likely because a custom resolver created a node or rel without an id field.".to_owned())), None)
//                     })?
//                     .to_owned(),
//                 match rtd.get_prop("props") {
//                     Ok(pp) => Some(Node::new(pp.type_name.to_owned(), row.get("r")?)),
//                     Err(_e) => None,
//                 },
//                 Node::new(rtd.get_prop("src")?.type_name.to_owned(), row.get("a")?),
//                 Node::new(dst_type.to_string(), row.get("b")?),
//             ))
//         } else {
//             return Err(Error::new(
//                 ErrorKind::InvalidPropertyType("b_label".to_string()),
//                 None,
//             )
//             .into());
//         }
//     } else {
//         return Err(
//             Error::new(ErrorKind::InvalidPropertyType("b_label".to_string()), None).into(),
//         );
//     };
// }

// For new and merge
//   Value::Array(create_input_array) => {
//         let mut results = CypherResult {
//             columns: vec![
//                 "a".to_string(),
//                 "r".to_string(),
//                 "b".to_string(),
//                 "b_label".to_string(),
//             ],
//             data: vec![],
//         };
//         for create_input_value in create_input_array {
//             let r = visit_rel_create_mutation_input(
//                 src_label,
//                 &ids,
//                 rel_name,
//                 &Info::new(
//                     itd.get_prop("create")?.type_name.to_owned(),
//                     info.type_defs.clone(),
//                 ),
//                 create_input_value,
//                 validators,
//                 transaction,
//             );

//             let data = r?.data;
//             results.data.extend(data);
//         }
//         Ok(results)
//     }
