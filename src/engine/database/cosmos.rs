//! Provides database interface types and functions for Cosmos DB

use super::{env_string, env_u16, DatabaseEndpoint, DatabasePool, Transaction};
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{ClauseType, SuffixGenerator};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::{Info, NodeType};
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use gremlin_client::{
    ConnectionOptions, GKey, GValue, GraphSON, GremlinClient, Map, ToGValue, VertexProperty,
};
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// A Cosmos DB endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::cosmos::CosmosEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ce = CosmosEndpoint::from_env()?;
/// #    Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CosmosEndpoint {
    host: String,
    port: u16,
    user: String,
    pass: String,
}

impl CosmosEndpoint {
    /// Reads a set of environment variables to construct a [`CosmosEndpoint`]. The environment
    /// variables are as follows
    ///
    /// * WG_COSMOS_HOST - the hostname for the Cosmos DB. For example,
    /// *my-db*.gremlin.cosmos.azure.com
    /// * WG_COSMOS_PORT - the port number for the Cosmos DB. For example, 443
    /// * WG_COSMOS_USER - the database and collection of the Cosmos DB. For example,
    /// /dbs/*my-db-name*/colls/*my-collection-name*
    /// * WG_COSMOS_PASS - the read/write key for the Cosmos DB.
    ///
    /// [`CosmosEndpoint`]: ./struct.CosmosEndpoint.html
    ///
    /// # Errors
    ///
    /// * [`EnvironmentVariableNotFound`] - if an environment variable does not exist
    /// * [`EnvironmentVariableParseError`] - if an environment variable has the wrong type,
    /// typically meaning that the WG_COSMOS_PORT variable cannot be parsed from a strign into an
    /// integer
    ///
    /// [`EnvironmentVariableNotFound`]: ../../enum.ErrorKind.html
    /// [`EnvironmentVariableParseError`]: ../../enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::database::cosmos::CosmosEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ce = CosmosEndpoint::from_env()?;
    /// #    Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<CosmosEndpoint, Error> {
        Ok(CosmosEndpoint {
            host: env_string("WG_COSMOS_HOST")?,
            port: env_u16("WG_COSMOS_PORT")?,
            user: env_string("WG_COSMOS_USER")?,
            pass: env_string("WG_COSMOS_PASS")?,
        })
    }
}

#[async_trait]
impl DatabaseEndpoint for CosmosEndpoint {
    async fn pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Cosmos(GremlinClient::connect(
            ConnectionOptions::builder()
                .host(&self.host)
                .port(self.port)
                .pool_size(num_cpus::get().try_into().unwrap_or(8))
                .ssl(true)
                .serializer(GraphSON::V1)
                .deserializer(GraphSON::V1)
                .credentials(&self.user, &self.pass)
                .build(),
        )?))
    }
}

#[derive(Debug)]
pub(crate) struct CosmosTransaction {
    client: GremlinClient,
}

impl CosmosTransaction {
    pub fn new(client: GremlinClient) -> CosmosTransaction {
        CosmosTransaction { client }
    }

    fn add_node_return(mut query: String, _var_name: &str) -> String {
        /*
        query.push_str(
            &(".select('".to_string()
                + var_name
                + "')"
                + ".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())"),
        );
        */
        query.push_str(".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())");
        query
    }

    fn add_properties(
        query: String,
        params: HashMap<String, Value>,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> (String, HashMap<String, Value>) {
        let (q, p): (String, HashMap<String, Value>) =
            props
                .into_iter()
                .fold((query, params), |(mut query, mut params), (k, v)| {
                    if let Value::Array(a) = v {
                        a.into_iter().enumerate().fold(
                            (query, params),
                            |(mut query, mut params), (i, val)| {
                                let suffix = sg.suffix();
                                query.push_str(
                                    &(".property(list, '".to_string()
                                        + &k
                                        + "', "
                                        + &k
                                        + &i.to_string()
                                        + &suffix
                                        + ")"),
                                );
                                params.insert(k.to_string() + &i.to_string() + &suffix, val);
                                (query, params)
                            },
                        )
                    } else {
                        let suffix = sg.suffix();
                        query.push_str(
                            &(".property('".to_string() + &k + "', " + &k + &suffix + ")"),
                        );
                        params.insert(k + &suffix, v);
                        (query, params)
                    }
                });

        (q, p)
    }

    fn add_rel_return(mut query: String) -> String {
        query.push_str(".project('rID', 'rProps', 'srcID', 'srcLabel', 'dstID', 'dstLabel')");
        query.push_str(".by(id()).by(valueMap())");
        query.push_str(".by(outV().id()).by(outV().label())");
        query.push_str(".by(inV().id()).by(inV().label())");
        query
    }

    fn extract_count(results: Vec<GValue>) -> Result<i32, Error> {
        if let Some(GValue::Int32(i)) = results.get(0) {
            Ok(*i)
        } else if let Some(GValue::Int64(i)) = results.get(0) {
            Ok(i32::try_from(*i)?)
        } else {
            Err(Error::TypeNotExpected)
        }
    }
    fn extract_node_properties(
        props: Map,
        type_def: &NodeType,
    ) -> Result<HashMap<String, Value>, Error> {
        trace!("CosmosTransaction::extract_node_properties called");
        props
            .into_iter()
            .map(|(key, property_list)| {
                if let (GKey::String(k), GValue::List(plist)) = (key, property_list) {
                    let v = if k == "partitionKey" || !type_def.property(&k)?.list() {
                        plist
                            .into_iter()
                            .next()
                            .ok_or_else(|| Error::ResponseItemNotFound {
                                name: k.to_string(),
                            })?
                            .try_into()?
                    } else {
                        Value::Array(
                            plist
                                .into_iter()
                                .map(|val| val.try_into())
                                .collect::<Result<Vec<Value>, Error>>()?,
                        )
                    };
                    Ok((k, v))
                } else {
                    Err(Error::TypeNotExpected)
                }
            })
            .collect::<Result<HashMap<String, Value>, Error>>()
    }

    fn gmap_to_hashmap(gv: GValue) -> Result<HashMap<String, GValue>, Error> {
        if let GValue::Map(map) = gv {
            map.into_iter()
                .map(|(k, v)| {
                    if let GKey::String(s) = k {
                        Ok((s, v))
                    } else {
                        Err(Error::TypeNotExpected)
                    }
                })
                .collect()
        } else {
            Err(Error::TypeNotExpected)
        }
    }

    fn nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        results: Vec<GValue>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::nodes called");

        results
            .into_iter()
            .map(|r| {
                let mut hm = CosmosTransaction::gmap_to_hashmap(r)?;

                if let (
                    Some(GValue::String(id)),
                    Some(GValue::String(label)),
                    Some(GValue::Map(props)),
                ) = (hm.remove("nID"), hm.remove("nLabel"), hm.remove("nProps"))
                {
                    let type_def = info.type_def_by_name(&label)?;
                    let mut fields = CosmosTransaction::extract_node_properties(props, type_def)?;
                    fields.insert("id".to_string(), Value::String(id));
                    Ok(Node::new(label, fields))
                } else {
                    Err(Error::ResponseItemNotFound {
                        name: "ID, label, or props".to_string(),
                    })
                }
            })
            .collect()
    }

    fn rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        results: Vec<GValue>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::rels called -- results: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        results, props_type_name, partition_key_opt);
        results
            .into_iter()
            .map(|r| {
                let mut hm = CosmosTransaction::gmap_to_hashmap(r)?;
                if let (
                    Some(GValue::String(rel_id)),
                    Some(GValue::Map(rel_props)),
                    Some(GValue::String(src_id)),
                    Some(GValue::String(src_label)),
                    Some(GValue::String(dst_id)),
                    Some(GValue::String(dst_label)),
                ) = (
                    hm.remove("rID"),
                    hm.remove("rProps"),
                    hm.remove("srcID"),
                    hm.remove("srcLabel"),
                    hm.remove("dstID"),
                    hm.remove("dstLabel"),
                ) {
                    let rel_fields = rel_props
                        .into_iter()
                        .map(|(key, val)| {
                            if let GKey::String(k) = key {
                                Ok((k, val.try_into()?))
                            } else {
                                Err(Error::TypeNotExpected)
                            }
                        })
                        .collect::<Result<HashMap<String, Value>, Error>>()?;

                    Ok(Rel::new(
                        Value::String(rel_id),
                        partition_key_opt.cloned(),
                        props_type_name.map(|ptn| Node::new(ptn.to_string(), rel_fields)),
                        NodeRef::Identifier {
                            id: Value::String(src_id),
                            label: src_label,
                        },
                        NodeRef::Identifier {
                            id: Value::String(dst_id),
                            label: dst_label,
                        },
                    ))
                } else {
                    Err(Error::ResponseItemNotFound {
                        name: "Rel, src, or dst".to_string(),
                    })
                }
            })
            .collect()
    }
}

impl Transaction for CosmosTransaction {
    fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn query_start() -> String {
        "g".to_string()
    }

    fn node_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &str,
        label: &str,
        clause: ClauseType,
        partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        _info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmsosTransaction::node_create_query called -- rel_create_fragments: {:#?}, params: {:#?}, label: {}, partition_key_opt: {:#?}, props: {:#?}", 
        rel_create_fragments, params, label, partition_key_opt, props);

        if let Some(_pk) = partition_key_opt {
            let (mut query, params) = if let ClauseType::Parameter(_) = clause {
                CosmosTransaction::add_properties(
                    "addV('".to_string() + label + "').property('partitionKey', partitionKey)",
                    params,
                    props,
                    sg,
                )
            } else {
                CosmosTransaction::add_properties(
                    ".addV('".to_string() + label + "').property('partitionKey', partitionKey)",
                    params,
                    props,
                    sg,
                )
            };

            query = query + ".as('" + node_var + "')";

            if !rel_create_fragments.is_empty() {
                query = rel_create_fragments
                    .into_iter()
                    .fold(query, |mut query, fragment| {
                        query.push_str(&fragment);
                        query
                    });
                query.push_str(&(".select('".to_string() + node_var + "')"));
            }

            query = if let ClauseType::Query(return_var) = clause {
                CosmosTransaction::add_node_return(query, &return_var)
            } else {
                query
            };

            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        trace!("CosmosTransaction::create_node called -- query: {}, params: {:#?}, partition_key_opt: {:#?}, info.name: {}", query, params, partition_key_opt, info.name());
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::nodes(results, info)?
                .into_iter()
                .next()
                .ok_or_else(|| Error::ResponseSetNotFound)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn rel_create_fragment<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query_opt: Option<String>,
        params: HashMap<String, Value>,
        src_var: &str,
        dst_query: &str,
        src_label: &str,
        dst_label: &str,
        dst_var: &str,
        _rel_var: &str,
        rel_name: &str,
        props: HashMap<String, Value>,
        _props_type_name: Option<&str>,
        clause: ClauseType,
        partition_key_opt: Option<&Value>,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_create_fragment called -- src_query_opt: {:#?}, params: {:#?}, src_var: {}, src_label: {}, dst_query: {}, dst_label: {}, dst_var: {:#?}, rel_name: {}, props: {:#?}, partition_key_opt: {:#?}, info.name: {}", 
        src_query_opt, params, src_var, src_label, dst_query, dst_label, dst_var, rel_name, props, partition_key_opt, info.name());
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let _src_prop = info.type_def_by_name(src_label)?.property(rel_name)?;

            /*
            if !src_prop.list() {
                self.single_rel_check::<GlobalCtx, RequestCtx>(
                    src_label,
                    src_ids.clone(),
                    dst_ids.clone(),
                    rel_name,
                    partition_key_opt,
                )?;
            }
            */

            let mut query = if let Some(src_query) = src_query_opt {
                src_query
            } else {
                String::new()
            };

            query.push_str(dst_query); // + ".as('" + dst_var + "')";

            /*
            let mut query = "V().has('partitionKey', partitionKey)".to_string();
            query.push_str(&(".hasLabel('".to_string() + dst_label + "')"));
            */

            // query = CosmosTransaction::add_has_id_clause(query, dst_ids)?;

            /*
            query.push_str(".as('dst').V().has('partitionKey', partitionKey)");
            query.push_str(&(".hasLabel('".to_string() + src_label + "')"));
            */

            // query = CosmosTransaction::add_has_id_clause(query, src_ids)?;

            query.push_str(
                &(".addE('".to_string()
                    + rel_name
                    + "').from('"
                    + src_var
                    + "').to('"
                    + dst_var
                    + "')"),
            );
            let (query, params) = CosmosTransaction::add_properties(query, params, props, sg);

            let query = match clause {
                ClauseType::Parameter(return_var) => query + ".as('" + &return_var + "')",
                ClauseType::FirstSubQuery(return_var) => query + ".as('" + &return_var + "')",
                ClauseType::SubQuery(return_var) => query + ".as('" + &return_var + "')",
                ClauseType::Query(_return_var) => CosmosTransaction::add_rel_return(query),
            };

            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn rel_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query_opt: Option<String>,
        rel_create_fragments: Vec<String>,
        _src_var: &str,
        _src_label: &str,
        rel_vars: Vec<String>,
        _dst_vars: Vec<String>,
        params: HashMap<String, Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_create_query called -- src_query_opt: {:#?}, rel_create_fragments: {:#?}, params: {:#?}", 
        src_query_opt, rel_create_fragments, params);

        let mut query = if let Some(src_query) = src_query_opt {
            src_query
        } else {
            String::new()
        };

        query.push_str(
            &rel_create_fragments
                .iter()
                .fold(String::new(), |mut rcfs, rcf| {
                    rcfs.push_str(rcf);
                    rcfs
                }),
        );
        if rel_vars.len() > 1 {
            query.push_str(".union(");
        } else {
            query.push_str(".")
        }

        query.push_str(&rel_vars.iter().enumerate().fold(
            String::new(),
            |mut rvs, (i, return_var)| {
                if i > 0 {
                    rvs.push_str(", ");
                }
                rvs.push_str(&("select('".to_string() + return_var + "')"));
                CosmosTransaction::add_rel_return(rvs)
            },
        ));

        if rel_create_fragments.len() > 1 {
            query.push_str(")");
        }

        Ok((query, params))
    }

    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_label: &str,
        _src_ids: Vec<Value>,
        _dst_label: &str,
        _dst_ids: Vec<Value>,
        _rel_name: &str,
        _props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        _info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::create_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", query, params, props_type_name, partition_key_opt);
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::rels(results, props_type_name, partition_key_opt)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<(String, String)>,
        mut params: HashMap<String, Value>,
        label: &str,
        _node_var: &str,
        _name_node: bool,
        union_type: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
        clause: ClauseType,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::node_read_fragment: rel_query_fragment: {:#?}, params: {:#?}, label: {}, union_type: {}, param_suffix: {}, props: {:#?}",
        rel_query_fragments, params, label, union_type, param_suffix, props);

        let mut query = String::new();

        if !union_type {
            query.push_str(&(".hasLabel('".to_string() + label + "')"));
        }

        query.push_str(".has('partitionKey', partitionKey)");

        props.into_iter().for_each(|(k, v)| {
            query.push_str(&(".has('".to_string() + &k + "', " + &k + param_suffix + ")"));
            params.insert(k + param_suffix, v);
        });

        if !rel_query_fragments.is_empty() {
            query.push_str(".where(");

            let multiple_rqfs = rel_query_fragments.len() > 1;

            if multiple_rqfs {
                query.push_str("and(");
            }

            query = rel_query_fragments
                .into_iter()
                .enumerate()
                .fold(query, |mut qs, (i, rqf)| {
                    if i == 0 {
                        qs.push_str(&("outE()".to_string() + &rqf.1));
                    } else {
                        qs.push_str(&(", outE()".to_string() + &rqf.1));
                    }
                    qs
                });
            /*
            let mut query = match return_rel {
                ReturnClause::None => query + "outE('" + rel_name + "')",
                ReturnClause::SubQuery(_) => query + ".outE('" + rel_name + "')",
                ReturnClause::Query(_) => {
                    query + ".E().hasLabel('" + rel_name + "').has('partitionKey', partitionKey)"
                }
            };
            */

            if multiple_rqfs {
                query.push_str(")");
            }

            query.push_str(")");
        }

        if let ClauseType::SubQuery(return_name) = clause {
            query.push_str(&(".as('".to_string() + &return_name + "')"));
        }

        Ok(("".to_string(), query, params))
    }

    fn node_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        _label: &str,
        _node_var: &str,
        _name_node: bool,
        _union_type: bool,
        clause: ClauseType,
        _param_suffix: &str,
        _props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::node_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, clause: {:#?}",
        match_fragment, where_fragment, params, clause);

        let mut query = ".V()".to_string() + match_fragment;

        /*
        match return_node {
            ReturnClause::None => (),
            _ => query.push_str(".V()"),
        };
        */

        query.push_str(where_fragment);

        if let ClauseType::Query(var_name) = clause {
            query = CosmosTransaction::add_node_return(query, &var_name);
        }

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::read_nodes called -- query: {}, partition_key_opt: {:#?}, params_opt: {:#?}, info.name: {}", query, partition_key_opt, params_opt, info.name());
        if let Some(pk) = partition_key_opt {
            let params = params_opt.unwrap_or_else(HashMap::new);
            let param_list: Vec<(&str, &dyn ToGValue)> =
                params
                    .iter()
                    .fold(vec![("partitionKey", pk)], |mut param_list, (k, v)| {
                        param_list.push((k, v));
                        param_list
                    });

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::nodes(results, info)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn rel_read_fragment(
        &mut self,
        mut params: HashMap<String, Value>,
        src_label: &str,
        _src_var: &str,
        src_query_opt: Option<(String, String)>,
        rel_name: &str,
        rel_suffix: &str,
        _dst_var: &str,
        _dst_suffix: &str,
        dst_query_opt: Option<(String, String)>,
        _top_level_query: bool,
        props: HashMap<String, Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_read_fragment called -- params: {:#?}, src_label: {}, src_query_opt: {:#?}, rel_name: {}, rel_suffix: {}, dst_query_opt: {:#?}, props: {:#?}",
        params, src_label, src_query_opt, rel_name, rel_suffix, dst_query_opt, props);

        let mut query = String::new();

        /*
        let mut query = match return_rel {
            ReturnClause::None => query + "outE('" + rel_name + "')",
            ReturnClause::SubQuery(_) => query + ".outE('" + rel_name + "')",
            ReturnClause::Query(_) => {
                query + ".E().hasLabel('" + rel_name + "').has('partitionKey', partitionKey)"
            }
        };
        */

        query.push_str(
            &(".hasLabel('".to_string() + rel_name + "').has('partitionKey', partitionKey)"),
        );

        props.into_iter().for_each(|(k, v)| {
            query.push_str(&(".has('".to_string() + &k + "', " + &k + rel_suffix + ")"));
            params.insert(k + rel_suffix, v);
        });

        if src_query_opt.is_some() || dst_query_opt.is_some() {
            query.push_str(".where(");

            let both = src_query_opt.is_some() && dst_query_opt.is_some();

            if both {
                query.push_str("and(");
            }

            if let Some(src_query) = src_query_opt {
                query.push_str(&("outV()".to_string() + &src_query.1));
            } else {
                // query.push_str(&("outV().hasLabel('".to_string() + src_label + "')"));
            }

            if both {
                query.push_str(", ");
            }

            /*
            if let Some(src_ids) = src_ids_opt {
                query = CosmosTransaction::add_has_id_clause(query, src_ids)?;
            }
            */

            if let Some(dst_query) = dst_query_opt {
                query.push_str(&("inV()".to_string() + &dst_query.1));
            }

            if both {
                query.push_str(")");
            }
            query.push_str(")");
        }

        Ok(("".to_string(), query, params))
    }

    fn rel_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        _src_label: &str,
        _src_var: &str,
        rel_name: &str,
        rel_suffix: &str,
        _dst_var: &str,
        _dst_suffix: &str,
        _top_level_query: bool,
        clause: ClauseType,
        _props: HashMap<String, Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, rel_name: {}, rel_suffix: {}, clause: {:#?}",
        match_fragment, where_fragment, params, rel_name, rel_suffix, clause);

        let mut query = match clause {
            ClauseType::Parameter(_) => "outE('".to_string() + rel_name + "')",
            ClauseType::FirstSubQuery(_) => ".E()".to_string(),
            ClauseType::SubQuery(_) => ".outE('".to_string() + rel_name + "')",
            ClauseType::Query(_) => ".E()".to_string(),
        };
        query.push_str(&match_fragment);

        /*
        let mut query = match return_rel {
            ReturnClause::None => query + "outE('" + rel_name + "')",
            ReturnClause::SubQuery(_) => query + ".outE('" + rel_name + "')",
            ReturnClause::Query(_) => {
                query + ".E().hasLabel('" + rel_name + "').has('partitionKey', partitionKey)"
            }
        };
        */

        query.push_str(where_fragment);

        let query = match clause {
            ClauseType::Parameter(_) => query,
            ClauseType::FirstSubQuery(_return_var) => query + ".as('rel" + rel_suffix + "')",
            ClauseType::SubQuery(_return_var) => query + ".as('rel" + rel_suffix + "')",
            ClauseType::Query(_return_var) => CosmosTransaction::add_rel_return(query),
        };

        Ok((query, params))
    }

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::read_rels called -- query: {}, props_type_name: {:#?}, partition_key_opt: {:#?}, params_opt: {:#?}", query, props_type_name, partition_key_opt, params_opt);
        if let Some(pk) = partition_key_opt {
            let params = params_opt.unwrap_or_else(HashMap::new);
            let param_list: Vec<(&str, &dyn ToGValue)> =
                params
                    .iter()
                    .fold(vec![("partitionKey", pk)], |mut param_list, (k, v)| {
                        param_list.push((k, v));
                        param_list
                    });

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::rels(results, props_type_name, partition_key_opt)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn node_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        change_queries: Vec<String>,
        params: HashMap<String, Value>,
        label: &str,
        node_var: &str,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        _info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::node_update_query called -- match_query: {}, change_queries: {:#?}, params: {:#?}, label: {}, node_var: {}, props: {:#?}, partition_key_opt: {:#?}", 
        match_query, change_queries, params, label, node_var, props, partition_key_opt);

        if let Some(_pk) = partition_key_opt {
            let mut query = match_query;
            if !change_queries.is_empty() {
                // query.push_str(&(".as('".to_string() + node_var + "')"));
                for cq in change_queries.iter() {
                    query.push_str(cq);
                }
                query.push_str(&(".select('".to_string() + node_var + "')"));
            }
            let (mut query, params) = CosmosTransaction::add_properties(query, params, props, sg);
            query = CosmosTransaction::add_node_return(query, node_var);

            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::update_nodes called -- query: {}, params: {:#?}, partition_key_opt: {:#?}, info.name: {}", query, params, partition_key_opt, info.name());
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::nodes(results, info)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn rel_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_var: &str,
        _src_label: &str,
        _src_suffix: &str,
        rel_name: &str,
        _rel_suffix: &str,
        rel_var: &str,
        _dst_suffix: &str,
        top_level_query: bool,
        props: HashMap<String, Value>,
        _props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_update_query called -- query: {}, params: {:#?}, rel_name: {}, props: {:#?}, partition_key_opt: {:#?}", 
        query, params, rel_name, props, partition_key_opt);
        if let Some(_pk) = partition_key_opt {
            /*
            let query = if top_level_query {
                // query + ".E().hasLabel('" + rel_name + "').has('partitionKey', partitionKey)"
                query + ".select('" + rel_var + "')"
            } else {
                query + ".select('" + src_var + "').outE('" + rel_name + "')"
                // query + "outE('" + rel_name + "')"
            };
            */
            let query = query + ".select('" + rel_var + "')";

            // query = CosmosTransaction::add_has_id_clause(query, rel_ids)?;

            let (mut query, params) = CosmosTransaction::add_properties(query, params, props, sg);

            if top_level_query {
                query = CosmosTransaction::add_rel_return(query);
            }

            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_label: &str,
        _rel_name: &str,
        _rel_ids: Vec<Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("CosmosTransaction::update_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", query, params, props_type_name, partition_key_opt);
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));
            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::rels(results, props_type_name, partition_key_opt)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn node_delete_query(
        &mut self,
        match_query: String,
        rel_delete_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &str,
        label: &str,
        partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
        top_level_query: bool,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::node_delete_query -- match_query: {}, params: {:#?}, label: {}, partition_key_opt: {:#?}", 
        match_query, params, label, partition_key_opt);
        let mut query = match_query;

        if let Some(_pk) = partition_key_opt {
            if !rel_delete_fragments.is_empty() {
                query.push_str(&(".as('".to_string() + node_var + "')"));
                for q in rel_delete_fragments.iter() {
                    query.push_str(q)
                }
                query.push_str(&(".select('".to_string() + node_var + "')"));
            }
            query.push_str(&(".sideEffect(drop())"));
            if top_level_query {
                query.push_str(".count()");
            }
            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn delete_nodes(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "CosmosTransaction::delete_nodes -- query: {}, params: {:#?}, partition_key_opt: {:#?}",
            query,
            params,
            partition_key_opt
        );
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::extract_count(results)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn rel_delete_query(
        &mut self,
        mut query: String,
        src_delete_query_opt: Option<String>,
        dst_delete_query_opt: Option<String>,
        params: HashMap<String, Value>,
        _src_label: &str,
        rel_name: &str,
        rel_suffix: &str,
        partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
        top_level_query: bool,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("CosmosTransaction::rel_delete_query -- query: {}, params: {:#?}, rel_name: {}, rel_suffix: {:#?}, partition_key_opt: {:#?}", 
        query, params, rel_name, rel_suffix, partition_key_opt);
        if let Some(_pk) = partition_key_opt {
            // let _length = rel_ids.len();

            if src_delete_query_opt.is_some() || dst_delete_query_opt.is_some() {
                if let Some(sdq) = src_delete_query_opt {
                    query.push_str(&sdq);
                }
                if let Some(ddq) = dst_delete_query_opt {
                    query.push_str(&ddq);
                }

                query.push_str(&(".select('rel".to_string() + rel_suffix + "')"));
            }

            // query = CosmosTransaction::add_has_id_clause(query, rel_ids)?;
            // query.push_str(&(".select('rel".to_string() + rel_suffix + "').sideEffect(drop())"));
            query.push_str(".sideEffect(drop())");
            if top_level_query {
                query.push_str(".count()");
            }

            Ok((query, params))
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn delete_rels(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_label: &str,
        _rel_name: &str,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "CosmosTransaction::delete_rels -- query: {}, partition_key_opt: {:#?}",
            query,
            partition_key_opt
        );
        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            params
                .iter()
                .for_each(|(k, v)| param_list.push((k.as_str(), v)));
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::extract_count(results)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn commit(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl ToGValue for Value {
    fn to_gvalue(&self) -> GValue {
        match self {
            Value::Array(a) => GValue::List(gremlin_client::List::new(
                a.iter().map(|val| val.to_gvalue()).collect(),
            )),
            Value::Bool(b) => b.to_gvalue(),
            Value::Float64(f) => f.to_gvalue(),
            Value::Int64(i) => i.to_gvalue(),
            Value::Map(hm) => GValue::Map(
                hm.iter()
                    .map(|(k, v)| (k.to_string(), v.to_gvalue()))
                    .collect::<HashMap<String, GValue>>()
                    .into(),
            ),
            Value::Null => GValue::String("".to_string()),
            Value::String(s) => s.to_gvalue(),
            // Note, the conversion of a UInt64 to an Int64 may be lossy, but GValue has
            // neither unsigned integer types, nor a try/error interface for value conversion
            Value::UInt64(i) => GValue::Int64(*i as i64),
        }
    }
}

impl TryFrom<GValue> for Value {
    type Error = Error;

    fn try_from(gvalue: GValue) -> Result<Value, Error> {
        match gvalue {
            GValue::Vertex(_v) => Err(Error::TypeConversionFailed {
                src: "GValue::Vertex".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Edge(_e) => Err(Error::TypeConversionFailed {
                src: "Gvalue::Edge".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::VertexProperty(vp) => Ok(vp.try_into()?),
            GValue::Property(_p) => Err(Error::TypeConversionFailed {
                src: "GValue::Property".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Uuid(u) => Ok(Value::String(u.to_hyphenated().to_string())),
            GValue::Int32(i) => Ok(Value::Int64(i.into())),
            GValue::Int64(i) => Ok(Value::Int64(i)),
            GValue::Float(f) => Ok(Value::Float64(f.into())),
            GValue::Double(f) => Ok(Value::Float64(f)),
            GValue::Date(_d) => Err(Error::TypeConversionFailed {
                src: "GValue::Date".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::List(_l) => Err(Error::TypeConversionFailed {
                src: "GValue::List".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Set(_s) => Err(Error::TypeConversionFailed {
                src: "GValue::Set".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Map(_m) => Err(Error::TypeConversionFailed {
                src: "GValue::Map".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Token(_t) => Err(Error::TypeConversionFailed {
                src: "GValue::Token".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::String(s) => Ok(Value::String(s)),
            GValue::Path(_p) => Err(Error::TypeConversionFailed {
                src: "GValue::Path".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::TraversalMetrics(_tm) => Err(Error::TypeConversionFailed {
                src: "GValue::TraversalMetrics".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Metric(_m) => Err(Error::TypeConversionFailed {
                src: "GValue::Metric".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::TraversalExplanation(_m) => Err(Error::TypeConversionFailed {
                src: "GVaue::TraversalExplanation".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::IntermediateRepr(_ir) => Err(Error::TypeConversionFailed {
                src: "GValue::IntermediateRepr".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::P(_p) => Err(Error::TypeConversionFailed {
                src: "GValue::P".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::T(_t) => Err(Error::TypeConversionFailed {
                src: "GValue::T".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Bytecode(_bc) => Err(Error::TypeConversionFailed {
                src: "GValue::Bytecode".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Traverser(_t) => Err(Error::TypeConversionFailed {
                src: "GValue::Traverser".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Scope(_s) => Err(Error::TypeConversionFailed {
                src: "GValue::Scope".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Order(_o) => Err(Error::TypeConversionFailed {
                src: "GValue::Order".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Bool(b) => Ok(Value::Bool(b)),
            GValue::TextP(_tp) => Err(Error::TypeConversionFailed {
                src: "GValue::TextP".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Pop(_p) => Err(Error::TypeConversionFailed {
                src: "GValue::Pop".to_string(),
                dst: "Value".to_string(),
            }),
            GValue::Cardinality(_c) => Err(Error::TypeConversionFailed {
                src: "GValue::Cardinality".to_string(),
                dst: "Value".to_string(),
            }),
        }
    }
}

impl TryFrom<VertexProperty> for Value {
    type Error = Error;

    fn try_from(vp: VertexProperty) -> Result<Value, Error> {
        Ok(vp
            .take::<GValue>()
            .map_err(|_e| Error::TypeConversionFailed {
                src: "VertexProperty".to_string(),
                dst: "Value".to_string(),
            })?
            .try_into()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{CosmosEndpoint, CosmosTransaction};
    #[test]
    fn test_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosEndpoint>();
    }

    #[test]
    fn test_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosEndpoint>();
    }

    #[test]
    fn test_transaction_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosTransaction>();
    }

    #[test]
    fn test_transaction_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosTransaction>();
    }
}
