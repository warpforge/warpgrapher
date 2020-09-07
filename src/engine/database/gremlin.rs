//! Provides database interface types and functions for Cosmos DB and other Gremlin-based DBs

use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{
    env_bool, env_string, env_u16, ClauseType, DatabaseEndpoint, DatabasePool, NodeQueryVar,
    RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::{Info, NodeType};
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use gremlin_client::{
    ConnectionOptions, GKey, GValue, GraphSON, GremlinClient, Map, TlsOptions, ToGValue,
    VertexProperty,
};
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use uuid::Uuid;

static NODE_RETURN_FRAGMENT: &str =
    ".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())";

static REL_RETURN_FRAGMENT: &str = ".project('rID', 'rProps', 'srcID', 'srcLabel', 'dstID', 'dstLabel').by(id()).by(valueMap()).by(outV().id()).by(outV().label()).by(inV().id()).by(inV().label())";

/// A Cosmos DB endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::gremlin::CosmosEndpoint;
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
    /// # use warpgrapher::engine::database::gremlin::CosmosEndpoint;
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

/// A Gremlin DB endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::gremlin::GremlinEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ge = GremlinEndpoint::from_env()?;
/// #    Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct GremlinEndpoint {
    host: String,
    port: u16,
    user: String,
    pass: String,
    accept_invalid_certs: bool,
    uuid: bool,
}

impl GremlinEndpoint {
    /// Reads a set of environment variables to construct a [`GremlinEndpoint`]. The environment
    /// variables are as follows
    ///
    /// * WG_GREMLIN_HOST - the hostname for the Gremlin-based DB. For example, `localhost`.
    /// * WG_GREMLIN_PORT - the port number for the Gremlin-based DB. For example, `443`.
    /// * WG_GREMLIN_USER - the username for the Gremlin-based DB. For example, `warpuser`.
    /// * WG_GREMLIN_PASS - the password used to authenticate the user.
    /// * WG_GREMLIN_CERT - true if Warpgrapher should accept an invalid cert. This could be
    /// necessary in a test environment, but it should be set to false in production environments.
    /// * WG_GREMLIN_UUID - true if the GREMLIN database uses a UUID type for node and vertex ids,
    /// false if the UUIDs for node and vertex ids are represented as string types
    ///
    /// The accept_invalid_certs option may be set to true in a test environment, where a test
    /// Gremlin server is running with an invalid cert. It should be set to false in production
    /// environments.
    ///
    /// [`GremlinEndpoint`]: ./struct.GremlinEndpoint.html
    ///
    /// # Errors
    ///
    /// * [`EnvironmentVariableNotFound`] - if an environment variable does not exist
    /// * [`EnvironmentVariableParseError`] - if an environment variable has the wrong type,
    /// typically meaning that the WG_GREMLIN_PORT variable cannot be parsed from a string into an
    /// integer
    ///
    /// [`EnvironmentVariableNotFound`]: ../../enum.ErrorKind.html
    /// [`EnvironmentVariableParseError`]: ../../enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::database::gremlin::GremlinEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ge = GremlinEndpoint::from_env()?;
    /// #    Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<GremlinEndpoint, Error> {
        Ok(GremlinEndpoint {
            host: env_string("WG_GREMLIN_HOST")?,
            port: env_u16("WG_GREMLIN_PORT")?,
            user: env_string("WG_GREMLIN_USER")?,
            pass: env_string("WG_GREMLIN_PASS")?,
            accept_invalid_certs: env_bool("WG_GREMLIN_CERT")?,
            uuid: env_bool("WG_GREMLIN_UUID")?,
        })
    }
}

#[async_trait]
impl DatabaseEndpoint for GremlinEndpoint {
    async fn pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Gremlin((
            GremlinClient::connect(
                ConnectionOptions::builder()
                    .host(&self.host)
                    .port(self.port)
                    .pool_size(num_cpus::get().try_into().unwrap_or(8))
                    .ssl(true)
                    .tls_options(TlsOptions {
                        accept_invalid_certs: self.accept_invalid_certs,
                    })
                    .serializer(GraphSON::V3)
                    .deserializer(GraphSON::V3)
                    .credentials(&self.user, &self.pass)
                    .build(),
            )?,
            self.uuid,
        )))
    }
}

#[derive(Debug)]
pub(crate) struct GremlinTransaction {
    client: GremlinClient,
    partition: bool,
    uuid: bool,
}

impl GremlinTransaction {
    pub fn new(client: GremlinClient, partition: bool, uuid: bool) -> GremlinTransaction {
        GremlinTransaction {
            client,
            partition,
            uuid,
        }
    }

    fn add_properties(
        query: String,
        params: HashMap<String, Value>,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> (String, HashMap<String, Value>) {
        let (ret_query, ret_params): (String, HashMap<String, Value>) =
            props
                .into_iter()
                .fold((query, params), |(mut outer_q, mut outer_p), (k, v)| {
                    if let Value::Array(a) = v {
                        a.into_iter()
                            .fold((outer_q, outer_p), |(mut inner_q, mut inner_p), val| {
                                let suffix = sg.suffix();
                                inner_q.push_str(
                                    &(".property(list, '".to_string()
                                        + &k
                                        + "', "
                                        + &k
                                        + &suffix
                                        + ")"),
                                );
                                inner_p.insert(k.to_string() + &suffix, val);
                                (inner_q, inner_p)
                            })
                    } else {
                        let suffix = sg.suffix();
                        outer_q.push_str(
                            &(".property('".to_string() + &k + "', " + &k + &suffix + ")"),
                        );
                        outer_p.insert(k + &suffix, v);
                        (outer_q, outer_p)
                    }
                });

        (ret_query, ret_params)
    }

    fn add_rel_return(query: String) -> String {
        query
            + ".project('rID', 'rProps', 'srcID', 'srcLabel', 'dstID', 'dstLabel')"
            + ".by(id()).by(valueMap())"
            + ".by(outV().id()).by(outV().label())"
            + ".by(inV().id()).by(inV().label())"
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
        trace!("GremlinTransaction::extract_node_properties called");
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
                .map(|(k, v)| match (k, v) {
                    (GKey::String(s), GValue::Uuid(uuid)) => {
                        Ok((s, GValue::String(uuid.to_hyphenated().to_string())))
                    }
                    (GKey::String(s), v) => Ok((s, v)),
                    (_, _) => Err(Error::TypeNotExpected),
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
        trace!(
            "GremlinTransaction::nodes called -- info.name: {}, results: {:#?}",
            info.name(),
            results
        );

        results
            .into_iter()
            .map(|r| {
                let mut hm = GremlinTransaction::gmap_to_hashmap(r)?;

                if let (
                    Some(GValue::String(id)),
                    Some(GValue::String(label)),
                    Some(GValue::Map(props)),
                ) = (hm.remove("nID"), hm.remove("nLabel"), hm.remove("nProps"))
                {
                    let type_def = info.type_def_by_name(&label)?;
                    let mut fields = GremlinTransaction::extract_node_properties(props, type_def)?;
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
        trace!("GremlinTransaction::rels called -- results: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        results, props_type_name, partition_key_opt);
        results
            .into_iter()
            .map(|r| {
                let mut hm = GremlinTransaction::gmap_to_hashmap(r)?;
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

impl Transaction for GremlinTransaction {
    fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn node_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::node_create_query called -- rel_create_fragments: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}, clause: {:#?}", 
        rel_create_fragments, params, node_var, props, clause);
        let mut first = match clause {
            ClauseType::Parameter => "addV('".to_string(),
            ClauseType::FirstSubQuery | ClauseType::SubQuery => ".addV('".to_string(),
            ClauseType::Query => "g.addV('".to_string(),
        } + node_var.label()?
            + "')";

        if self.partition {
            first.push_str(".property('partitionKey', partitionKey)");
        }

        let (mut query, params) = GremlinTransaction::add_properties(first, params, props, sg);

        query.push_str(&(".as('".to_string() + node_var.name() + "')"));

        if !rel_create_fragments.is_empty() {
            query.push_str(&rel_create_fragments.into_iter().fold(
                String::new(),
                |mut acc, fragment| {
                    acc.push_str(&fragment);
                    acc
                },
            ));
            query.push_str(&(".select('".to_string() + node_var.name() + "')"));
        }

        if let ClauseType::Query = clause {
            query.push_str(NODE_RETURN_FRAGMENT);
        }

        Ok((query, params))
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        trace!("GremlinTransaction::create_node called -- query: {}, params: {:#?}, partition_key_opt: {:#?}", query, params, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let r0 = self.client.execute(query, param_list.as_slice());
        trace!("GremlinTransaction::create_node -- r0: {:#?}", r0);
        let raw_results = r0?;
        trace!(
            "GremlinTransaction::create_node -- raw_results: {:#?}",
            raw_results
        );
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::nodes(results, info)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::ResponseSetNotFound)
    }

    fn rel_create_fragment<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        dst_query: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_create_fragment called -- dst_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}, clause: {:#?}", 
        dst_query, params, rel_var, props, clause);

        let query = dst_query.to_string()
            + ".addE('"
            + rel_var.label()
            + "').from('"
            + rel_var.src().name()
            + "').to('"
            + rel_var.dst().name()
            + "')";
        let (mut q, p) = GremlinTransaction::add_properties(query, params, props, sg);

        match clause {
            ClauseType::Parameter | ClauseType::FirstSubQuery | ClauseType::SubQuery => {
                q.push_str(&(".as('".to_string() + rel_var.name() + "')"))
            }
            ClauseType::Query => q.push_str(REL_RETURN_FRAGMENT),
        };

        Ok((q, p))
    }

    fn rel_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query_opt: Option<String>,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        rel_vars: Vec<RelQueryVar>,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_create_query called -- src_query_opt: {:#?}, rel_create_fragments: {:#?}, params: {:#?}, rel_vars: {:#?}, clause: {:#?}",
        src_query_opt, rel_create_fragments, params, rel_vars, clause);
        let mut query = if let ClauseType::Query = clause {
            "g".to_string()
        } else {
            String::new()
        };

        if let Some(src_query) = src_query_opt {
            query.push_str(&src_query)
        }

        rel_create_fragments
            .iter()
            .for_each(|rcf| query.push_str(rcf));

        if rel_vars.len() > 1 {
            query.push_str(".union(");
        } else {
            query.push_str(".")
        }

        rel_vars.iter().enumerate().for_each(|(i, return_var)| {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&("select('".to_string() + return_var.name() + "')"));
            query.push_str(REL_RETURN_FRAGMENT);
        });

        if rel_create_fragments.len() > 1 {
            query.push_str(")");
        }

        Ok((query, params))
    }

    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("GremlinTransaction::create_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query, params, props_type_name, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<(String, String)>,
        mut params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::node_read_fragment called -- rel_query_fragment: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}, clause: {:#?}",
        rel_query_fragments, params, node_var, props, clause);

        let param_suffix = sg.suffix();
        let mut query = if node_var.label().is_ok() {
            ".hasLabel('".to_string() + node_var.label()? + "')"
        } else {
            String::new()
        };

        if self.partition {
            query.push_str(".has('partitionKey', partitionKey)");
        }

        for (k, v) in props.into_iter() {
            if k == "id" {
                // For id, we omit the single quotes, because it's a "system" property, not just a
                // user defined property.
                query.push_str(&(".has(".to_string() + &k + ", " + &k + &param_suffix + ")"));
            } else {
                // For all user-defined properties, we single-quote the property name
                query.push_str(&(".has('".to_string() + &k + "', " + &k + &param_suffix + ")"));
            }

            if self.uuid && k == "id" {
                if let Value::String(s) = v {
                    params.insert(k + &param_suffix, Value::Uuid(Uuid::parse_str(&s)?));
                } else {
                    return Err(Error::TypeConversionFailed {
                        src: format!("{:#?}", v),
                        dst: "String".to_string(),
                    });
                }
            } else {
                params.insert(k + &param_suffix, v);
            }
        }

        if !rel_query_fragments.is_empty() {
            query.push_str(".where(");

            if rel_query_fragments.len() > 1 {
                query.push_str("and(");
            }

            rel_query_fragments.iter().enumerate().for_each(|(i, rqf)| {
                if i == 0 {
                    query.push_str(&("outE()".to_string() + &rqf.1));
                } else {
                    query.push_str(&(", outE()".to_string() + &rqf.1));
                }
            });

            if rel_query_fragments.len() > 1 {
                query.push_str(")");
            }

            query.push_str(")");
        }

        if let ClauseType::SubQuery = clause {
            query.push_str(&(".as('".to_string() + node_var.name() + "')"));
        }

        Ok(("".to_string(), query, params))
    }

    fn node_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        _node_var: &NodeQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::node_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, clause: {:#?}",
        match_fragment, where_fragment, params, clause);

        let mut query = if let ClauseType::Query = clause {
            "g".to_string()
        } else {
            String::new()
        };

        query.push_str(&(".V()".to_string() + match_fragment + where_fragment));

        if let ClauseType::Query = clause {
            query.push_str(NODE_RETURN_FRAGMENT);
        }

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_nodes called -- query: {}, partition_key_opt: {:#?}, params_opt: {:#?}, info.name: {}", 
        query, partition_key_opt, params_opt, info.name());

        let params = params_opt.unwrap_or_else(HashMap::new);
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::nodes(results, info)
    }

    fn rel_read_fragment(
        &mut self,
        src_query_opt: Option<(String, String)>,
        dst_query_opt: Option<(String, String)>,
        mut params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_read_fragment called -- src_query_opt: {:#?}, dst_query_opt: {:#?}, params: {:#?}, rel_var: {:#?}, props: {:#?}",
        src_query_opt, dst_query_opt, params, rel_var, props);

        let param_suffix = sg.suffix();
        let mut query = ".hasLabel('".to_string() + rel_var.label() + "')";

        if self.partition {
            query.push_str(".has('partitionKey', partitionKey)");
        }

        for (k, v) in props.into_iter() {
            if k == "id" {
                // For id, we omit the single quotes, because it's a "system" property, not just a
                // user defined property.
                query.push_str(&(".has(".to_string() + &k + ", " + &k + &param_suffix + ")"));
            } else {
                // For all other user defined properties, we sinlge quote the property name.
                query.push_str(&(".has('".to_string() + &k + "', " + &k + &param_suffix + ")"));
            }

            if self.uuid && k == "id" {
                if let Value::String(s) = v {
                    params.insert(k + &param_suffix, Value::Uuid(Uuid::parse_str(&s)?));
                } else {
                    return Err(Error::TypeConversionFailed {
                        src: format!("{:#?}", v),
                        dst: "String".to_string(),
                    });
                }
            } else {
                params.insert(k + &param_suffix, v);
            }
        }

        if src_query_opt.is_some() || dst_query_opt.is_some() {
            query.push_str(".where(");

            let both = src_query_opt.is_some() && dst_query_opt.is_some();

            if both {
                query.push_str("and(");
            }

            if let Some(src_query) = src_query_opt {
                query.push_str(&("outV()".to_string() + &src_query.1));
            }

            if both {
                query.push_str(", ");
            }

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
        rel_var: &RelQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, rel_var: {:#?}, clause: {:#?}",
        match_fragment, where_fragment, params, rel_var, clause);

        let mut q = match clause {
            // ClauseType::Parameter => "outE('".to_string() + rel_var.label() + "')",
            ClauseType::Parameter => "outE()".to_string(),
            ClauseType::FirstSubQuery => ".E()".to_string(),
            // ClauseType::SubQuery => ".outE('".to_string() + rel_var.label() + "')",
            ClauseType::SubQuery => ".outE()".to_string(),
            ClauseType::Query => "g.E()".to_string(),
        };

        q.push_str(match_fragment);
        q.push_str(where_fragment);

        let query = match clause {
            ClauseType::Parameter => q,
            ClauseType::FirstSubQuery => q + ".as('" + rel_var.name() + "')",
            ClauseType::SubQuery => q + ".as('" + rel_var.name() + "')",
            ClauseType::Query => GremlinTransaction::add_rel_return(q),
        };

        Ok((query, params))
    }

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_rels called -- query: {}, props_type_name: {:#?}, partition_key_opt: {:#?}, params_opt: {:#?}", 
        query, props_type_name, partition_key_opt, params_opt);

        let params = params_opt.unwrap_or_else(HashMap::new);
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        trace!(
            "GremlinTransaction::read_rels -- raw_results: {:#?}",
            raw_results
        );
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn node_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        change_queries: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::node_update_query called: match_query: {}, change_queries: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}, clause: {:#?}",
        match_query, change_queries, params, node_var, props, clause);
        let mut query = if let ClauseType::Query = clause {
            "g".to_string() + &match_query
        } else {
            match_query
        };

        if !change_queries.is_empty() {
            for cq in change_queries.iter() {
                query.push_str(cq);
            }
            query.push_str(&(".select('".to_string() + node_var.name() + "')"));
        }
        let (mut query, params) = GremlinTransaction::add_properties(query, params, props, sg);
        query.push_str(NODE_RETURN_FRAGMENT);

        Ok((query, params))
    }

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_nodes called: query: {}, params: {:#?}, partition_key_opt: {:#?}",
        query, params, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        trace!(
            "GremlinTransaction::update_nodes -- raw_results: {:#?}",
            raw_results
        );
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::nodes(results, info)
    }

    fn rel_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_update_query called -- match_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}, clause: {:#?}",
        match_query, params, rel_var, props, clause);

        let mut fragment = if let ClauseType::Query = clause {
            "g".to_string() + &match_query
        } else {
            match_query
        };

        fragment.push_str(&(".select('".to_string() + rel_var.name() + "')"));
        let (q, p) = GremlinTransaction::add_properties(fragment, params, props, sg);

        match clause {
            ClauseType::Parameter | ClauseType::FirstSubQuery | ClauseType::SubQuery => Ok((q, p)),
            ClauseType::Query => Ok((GremlinTransaction::add_rel_return(q), p)),
        }
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query, params, props_type_name, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn node_delete_query(
        &mut self,
        match_query: String,
        rel_delete_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        clause: ClauseType,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::node_delete_query -- match_query: {}, rel_delete_fragments: {:#?}, params: {:#?}, node_var: {:#?}, clause: {:#?}",
        match_query, rel_delete_fragments, params, node_var, clause);

        let mut query = if let ClauseType::Query = clause {
            "g".to_string() + &match_query
        } else {
            match_query
        };

        if !rel_delete_fragments.is_empty() {
            query.push_str(&(".as('".to_string() + node_var.name() + "')"));
            for q in rel_delete_fragments.iter() {
                query.push_str(q)
            }
            query.push_str(&(".select('".to_string() + node_var.name() + "')"));
        }
        query.push_str(&(".sideEffect(drop())"));
        if let ClauseType::Query = clause {
            query.push_str(".count()");
        }
        Ok((query, params))
    }

    fn delete_nodes(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_nodes called -- query: {}, params: {:#?}, partition_key_opt: {:#?}", 
        query, params, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::extract_count(results)
    }

    fn rel_delete_query(
        &mut self,
        query: String,
        src_delete_query_opt: Option<String>,
        dst_delete_query_opt: Option<String>,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        clause: ClauseType,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("GremlinTransaction::rel_delete_query called -- query: {}, src_delete_query_opt: {:#?}, dst_delete_query_opt: {:#?}, params: {:#?}, rel_var: {:#?}, clause: {:#?}",
        query, src_delete_query_opt, dst_delete_query_opt, params, rel_var, clause);
        let mut q = if let ClauseType::Query = clause {
            "g".to_string() + &query
        } else {
            query
        };

        if src_delete_query_opt.is_some() || dst_delete_query_opt.is_some() {
            if let Some(sdq) = src_delete_query_opt {
                q.push_str(&sdq);
            }
            if let Some(ddq) = dst_delete_query_opt {
                q.push_str(&ddq);
            }

            q.push_str(&(".select('".to_string() + rel_var.name() + "')"));
        }

        q.push_str(".sideEffect(drop())");

        if let ClauseType::Query = clause {
            q.push_str(".count()");
        }

        Ok((q, params))
    }

    fn delete_rels(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_rels called -- query: {}, params: {:#?}, partition_key_opt: {:#?}",
        query, params, partition_key_opt);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partition {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::extract_count(results)
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
            Value::Uuid(uuid) => GValue::Uuid(*uuid),
        }
    }
}

impl TryFrom<GValue> for Value {
    type Error = Error;

    fn try_from(gvalue: GValue) -> Result<Value, Error> {
        match gvalue {
            GValue::Null => Ok(Value::Null),
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
    use super::{CosmosEndpoint, GremlinEndpoint, GremlinTransaction};

    #[test]
    fn test_cosmos_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosEndpoint>();
    }

    #[test]
    fn test_cosmos_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosEndpoint>();
    }

    #[test]
    fn test_gremlin_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GremlinEndpoint>();
    }

    #[test]
    fn test_gremlin_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<GremlinEndpoint>();
    }

    #[test]
    fn test_gremlin_transaction_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GremlinTransaction>();
    }

    #[test]
    fn test_gremlin_transaction_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<GremlinTransaction>();
    }
}
