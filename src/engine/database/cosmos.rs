//! Provides database interface types and functions for Cosmos DB

use super::{env_string, env_u16, DatabaseEndpoint, DatabasePool, Transaction};
use crate::engine::context::{GlobalContext, RequestContext};
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
    /// variables are
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

    fn add_has_id_clause(mut query: String, ids: Vec<Value>) -> Result<String, Error> {
        query.push_str(".hasId(");

        let mut ret_query = ids
            .iter()
            .enumerate()
            .try_fold(query, |mut query, (i, id_val)| {
                if let Value::String(id) = id_val {
                    if i == 0 {
                        query.push_str(&("'".to_string() + &id + "'"));
                    } else {
                        query.push_str(&(", '".to_string() + &id + "'"));
                    }
                    Ok(query)
                } else {
                    Err(Error::TypeNotExpected)
                }
            })?;

        ret_query.push_str(")");
        Ok(ret_query)
    }

    fn add_node_return(mut query: String) -> String {
        query.push_str(".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())");
        query
    }

    fn add_properties(
        query: String,
        props: &HashMap<String, Value>,
    ) -> (String, Vec<(String, &dyn ToGValue)>) {
        let (ret_query, param_list): (String, Vec<(String, &dyn ToGValue)>) = props.iter().fold(
            (query, Vec::new()),
            |(mut query, mut param_list), (k, v)| {
                if let Value::Array(a) = v {
                    a.iter().enumerate().fold(
                        (query, param_list),
                        |(mut query, mut param_list), (i, val)| {
                            query.push_str(
                                &(".property(list, '".to_string()
                                    + k
                                    + "', "
                                    + k
                                    + &i.to_string()
                                    + ")"),
                            );
                            param_list.push((k.to_string() + &i.to_string(), val));
                            (query, param_list)
                        },
                    )
                } else {
                    query.push_str(&(".property('".to_string() + k + "', " + k + ")"));
                    param_list.push((k.to_string(), v));
                    (query, param_list)
                }
            },
        );

        (ret_query, param_list)
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
        trace!("CosmosTransaction::rels called");
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
                        NodeRef::new(Value::String(src_id), src_label),
                        NodeRef::new(Value::String(dst_id), dst_label),
                    ))
                } else {
                    Err(Error::ResponseItemNotFound {
                        name: "Rel, src, or dst".to_string(),
                    })
                }
            })
            .collect()
    }

    fn single_rel_check<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &self,
        src_label: &str,
        src_ids: Vec<Value>,
        dst_ids: Vec<Value>,
        rel_name: &str,
        partition_key_opt: Option<&Value>,
    ) -> Result<(), Error> {
        if let Some(pk) = partition_key_opt {
            let param_list: Vec<(&str, &dyn ToGValue)> = vec![("partitionKey", pk)];
            let mut query = CosmosTransaction::add_has_id_clause(
                "g.V().has('partitionKey', partitionKey)".to_string()
                    + ".has('label', '"
                    + src_label
                    + "')",
                src_ids,
            )?;
            query.push_str(&(".outE('".to_string() + rel_name + "').count()"));

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            if CosmosTransaction::extract_count(results)? > 0 || dst_ids.len() > 1 {
                Err(Error::RelDuplicated {
                    rel_name: rel_name.to_string(),
                })
            } else {
                Ok(())
            }
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }
}

impl Transaction for CosmosTransaction {
    fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        if let Some(pk) = partition_key_opt {
            let (mut query, pl) = CosmosTransaction::add_properties(
                "g.addV('".to_string() + label + "').property('partitionKey', partitionKey)",
                &props,
            );
            let mut param_list: Vec<(&str, &dyn ToGValue)> =
                pl.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            param_list.push(("partitionKey", pk));
            query = CosmosTransaction::add_node_return(query);

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
        if let Some(pk) = partition_key_opt {
            let src_prop = info.type_def_by_name(src_label)?.property(rel_name)?;

            if !src_prop.list() {
                self.single_rel_check::<GlobalCtx, RequestCtx>(
                    src_label,
                    src_ids.clone(),
                    dst_ids.clone(),
                    rel_name,
                    partition_key_opt,
                )?;
            }

            let mut query = "g.V().has('partitionKey', partitionKey)".to_string();
            query.push_str(&(".hasLabel('".to_string() + dst_label + "')"));

            query = CosmosTransaction::add_has_id_clause(query, dst_ids)?;

            query.push_str(".as('dst').V().has('partitionKey', partitionKey)");
            query.push_str(&(".hasLabel('".to_string() + src_label + "')"));

            query = CosmosTransaction::add_has_id_clause(query, src_ids)?;

            query.push_str(&(".addE('".to_string() + rel_name + "').to('dst')"));
            query.push_str(".property('partitionKey', partitionKey)");

            let (mut query, pl) = CosmosTransaction::add_properties(query, &props);
            let mut param_list: Vec<(&str, &dyn ToGValue)> =
                pl.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            param_list.push(("partitionKey", pk));

            query = CosmosTransaction::add_rel_return(query);

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::rels(results, props_type_name, partition_key_opt)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn node_query(
        &mut self,
        rel_query_fragments: Vec<String>,
        mut params: HashMap<String, Value>,
        label: &str,
        _var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        let mut query = String::new();

        if return_node {
            query.push_str("g.V()");
        }

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
                        qs.push_str(&("outE()".to_string() + &rqf));
                    } else {
                        qs.push_str(&(", outE()".to_string() + &rqf));
                    }
                    qs
                });

            if multiple_rqfs {
                query.push_str(")");
            }

            query.push_str(")");
        }

        if return_node {
            query = CosmosTransaction::add_node_return(query);
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
        _dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        let mut query = String::new();

        if return_rel {
            query.push_str("g.E()");
        }

        query.push_str(&(".hasLabel('".to_string() + rel_name + "')"));
        query.push_str(".has('partitionKey', partitionKey)");

        props.into_iter().for_each(|(k, v)| {
            query.push_str(
                &(".has('".to_string() + &k + "', " + &k + src_suffix + dst_suffix + ")"),
            );
            params.insert(k + src_suffix + dst_suffix, v);
        });
        query.push_str(".where(");

        if dst_query_opt.is_some() {
            query.push_str("and(");
        }

        if let Some(src_query) = src_query_opt {
            query.push_str(&("outV()".to_string() + &src_query));
        } else {
            query.push_str(&("outV().hasLabel('".to_string() + src_label + "')"));
        }

        if let Some(src_ids) = src_ids_opt {
            query = CosmosTransaction::add_has_id_clause(query, src_ids)?;
        }

        if let Some(dst_query) = dst_query_opt {
            query.push_str(&(", ".to_string() + "inV()" + &dst_query + ")"));
        }

        query.push_str(")");

        if return_rel {
            query = CosmosTransaction::add_rel_return(query);
        }

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: &str,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
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

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: &str,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
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

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        ids: Vec<Value>,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        if let Some(pk) = partition_key_opt {
            let query = CosmosTransaction::add_has_id_clause(
                "g.V().hasLabel('".to_string() + label + "').has('partitionKey', partitionKey)",
                ids,
            )?;

            let (mut query, pl) = CosmosTransaction::add_properties(query, &props);
            let mut param_list: Vec<(&str, &dyn ToGValue)> =
                pl.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            param_list.push(("partitionKey", pk));

            query = CosmosTransaction::add_node_return(query);

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::nodes(results, info)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        _src_label: &str,
        rel_name: &str,
        rel_ids: Vec<Value>,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        if let Some(pk) = partition_key_opt {
            let mut query = String::from("g.E().hasLabel('")
                + rel_name
                + "').has('partitionKey', partitionKey)";

            query = CosmosTransaction::add_has_id_clause(query, rel_ids)?;

            let (mut query, pl) = CosmosTransaction::add_properties(query, &props);
            let mut param_list: Vec<(&str, &dyn ToGValue)> =
                pl.iter().map(|(k, v)| (k.as_str(), *v)).collect();
            param_list.push(("partitionKey", pk));

            query = CosmosTransaction::add_rel_return(query);

            let raw_results = self.client.execute(query, param_list.as_slice())?;
            let results = raw_results
                .map(|r| Ok(r?))
                .collect::<Result<Vec<GValue>, Error>>()?;

            CosmosTransaction::rels(results, props_type_name, partition_key_opt)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Vec<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        if let Some(pk) = partition_key_opt {
            let mut query =
                String::from("g.V().hasLabel('") + label + "').has('partitionKey', partitionKey)";
            let length = ids.len();

            query = CosmosTransaction::add_has_id_clause(query, ids)?;
            query.push_str(".drop()");

            let param_list: Vec<(&str, &dyn ToGValue)> = vec![("partitionKey", pk)];

            self.client.execute(query, param_list.as_slice())?;

            let results: Vec<GValue> = vec![(GValue::Int32(i32::try_from(length)?))];
            CosmosTransaction::extract_count(results)
        } else {
            Err(Error::PartitionKeyNotFound)
        }
    }

    fn delete_rels(
        &mut self,
        _src_label: &str,
        rel_name: &str,
        rel_ids: Vec<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        if let Some(pk) = partition_key_opt {
            let mut query = String::from("g.E().hasLabel('")
                + rel_name
                + "').has('partitionKey', partitionKey)";
            let length = rel_ids.len();

            query = CosmosTransaction::add_has_id_clause(query, rel_ids)?;
            query.push_str(".drop()");

            let param_list: Vec<(&str, &dyn ToGValue)> = vec![("partitionKey", pk)];

            self.client.execute(query, param_list.as_slice())?;
            let results: Vec<GValue> = vec![(GValue::Int32(i32::try_from(length)?))];
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
