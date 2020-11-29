//! Provides database interface types and functions for Cosmos DB and other Gremlin-based DBs

use crate::engine::context::RequestContext;
#[cfg(feature = "gremlin")]
use crate::engine::database::env_bool;
use crate::engine::database::{
    env_string, env_u16, ClauseType, DatabaseEndpoint, DatabasePool, NodeQueryVar, RelQueryVar,
    SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::{Info, NodeType};
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
#[cfg(feature = "gremlin")]
use gremlin_client::TlsOptions;
use gremlin_client::{
    ConnectionOptions, GKey, GValue, GraphSON, GremlinClient, Map, ToGValue, VertexProperty,
};
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
#[cfg(feature = "gremlin")]
use std::env::var_os;
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
#[cfg(feature = "cosmos")]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CosmosEndpoint {
    host: String,
    port: u16,
    user: String,
    pass: String,
}

#[cfg(feature = "cosmos")]
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

#[cfg(feature = "cosmos")]
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
#[cfg(feature = "gremlin")]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct GremlinEndpoint {
    host: String,
    port: u16,
    user: Option<String>,
    pass: Option<String>,
    accept_invalid_certs: bool,
    uuid: bool,
    use_tls: bool,
}

#[cfg(feature = "gremlin")]
impl GremlinEndpoint {
    /// Reads a set of environment variables to construct a [`GremlinEndpoint`]. The environment
    /// variables are as follows
    ///
    /// * WG_GREMLIN_HOST - the hostname for the Gremlin-based DB. For example, `localhost`.
    /// * WG_GREMLIN_PORT - the port number for the Gremlin-based DB. For example, `443`.
    /// * WG_GREMLIN_USER - the username for the Gremlin-based DB. For example, `warpuser`.
    /// * WG_GREMLIN_PASS - the password used to authenticate the user.
    /// * WG_GREMLIN_USE_TLS - true if Warpgrapher should use TLS to connect to gremlin endpoint.
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
            user: var_os("WG_GREMLIN_USER").map(|osstr| osstr.to_string_lossy().into_owned()),
            pass: var_os("WG_GREMLIN_PASS").map(|osstr| osstr.to_string_lossy().into_owned()),
            accept_invalid_certs: env_bool("WG_GREMLIN_CERT")?,
            uuid: env_bool("WG_GREMLIN_UUID")?,
            use_tls: env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true),
        })
    }
}

#[cfg(feature = "gremlin")]
#[async_trait]
impl DatabaseEndpoint for GremlinEndpoint {
    async fn pool(&self) -> Result<DatabasePool, Error> {
        let mut options_builder = ConnectionOptions::builder()
            .host(&self.host)
            .port(self.port)
            .pool_size(num_cpus::get().try_into().unwrap_or(8))
            .serializer(GraphSON::V3)
            .deserializer(GraphSON::V3);
        if let (Some(user), Some(pass)) = (self.user.as_ref(), self.pass.as_ref()) {
            options_builder = options_builder.credentials(user, pass);
        }
        if self.use_tls {
            options_builder = options_builder.ssl(true).tls_options(TlsOptions {
                accept_invalid_certs: self.accept_invalid_certs,
            });
        }
        let options = options_builder.build();
        Ok(DatabasePool::Gremlin((
            GremlinClient::connect(options)?,
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
        props: HashMap<String, Value>,
        params: HashMap<String, Value>,
    ) -> (String, HashMap<String, Value>) {
        let mut sg = SuffixGenerator::new();

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

    fn nodes<RequestCtx: RequestContext>(
        results: Vec<GValue>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
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

    fn rels<RequestCtx: RequestContext>(
        results: Vec<GValue>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
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

    fn create_node<RequestCtx: RequestContext>(
        &mut self,
        label: &str,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<RequestCtx>, Error> {
        trace!("GremlinTransaction::create_node called -- label: {}, props: {:#?}, partition_key_opt: {:#?}", label, props, partition_key_opt);

        let mut first = "g.addV('".to_string() + label + "')";

        if self.partition {
            first.push_str(".property('partitionKey', partitionKey)");
        }

        let (mut query, params) = GremlinTransaction::add_properties(first, props, HashMap::new());
        trace!(
            "GremlinTransaction::create_node -- query: {}, param_list: {:#?}",
            query,
            params
        );

        query += NODE_RETURN_FRAGMENT;

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
        trace!("GremlinTransaction::create_node -- results: {:#?}", results);

        GremlinTransaction::nodes(results, info)?
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)
    }

    fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_query: &str,
        dst_query: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::create_rels called -- dst_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}, src_query: {}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        dst_query, params, rel_var, props, src_query, props_type_name, partition_key_opt);

        let query = "g.V()".to_string()
            + src_query
            + ".V()"
            + dst_query
            + ".addE('"
            + rel_var.label()
            + "').from('"
            + rel_var.src().name()
            + "').to('"
            + rel_var.dst().name()
            + "')";

        let (mut q, p) = GremlinTransaction::add_properties(query, props, params);

        q.push_str(REL_RETURN_FRAGMENT);

        trace!("GremlinTransaction::create_rels -- q: {}, p: {:#?}", q, p);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
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

        let raw_results = self.client.execute(q, param_list.as_slice())?;
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

        let mut query = String::new();

        /*
        match clause {
            ClauseType::Parameter | ClauseType::SubQuery | ClauseType::FirstSubQuery => {
                query.push_str(".V()")
            }
            ClauseType::Query => (),
        };
        query.push_str(".V()");
        */

        if node_var.label().is_ok() {
            query.push_str(&(".hasLabel('".to_string() + node_var.label()? + "')"));
        }

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
                query.push(')');
            }

            query.push(')');
        }

        if let ClauseType::SubQuery = clause {
            query.push_str(&(".as('".to_string() + node_var.name() + "')"));
        }

        trace!("GremlinTransaction::node_read_fragment returning -- match_query: {}, where_query: {}, params: {:#?}", "".to_string(), query, params);

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
        // query.push_str(&(match_fragment.to_string() + where_fragment));

        if let ClauseType::Query = clause {
            query.push_str(NODE_RETURN_FRAGMENT);
        }

        Ok((query, params))
    }

    fn node_read_by_ids_query<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        nodes: Vec<Node<RequestCtx>>,
        clause: ClauseType,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!(
            "GremlinTransaction::node_read_by_ids_query called -- node_var: {:#?}, nodes: {:#?}",
            node_var,
            nodes
        );

        let mut query = ".hasLabel('".to_string() + node_var.label()? + "')";

        if self.partition {
            query.push_str(".has('partitionKey', partitionKey)");
        }
        query.push_str(".hasId(within(id_list))");
        match clause {
            ClauseType::Parameter => (),
            ClauseType::FirstSubQuery | ClauseType::SubQuery | ClauseType::Query => {
                query.push_str(&(".as('".to_string() + node_var.name() + "')"))
            }
        }

        let ids = nodes
            .iter()
            .map(|n| n.id())
            .collect::<Result<Vec<&Value>, Error>>()?
            .into_iter()
            .cloned()
            .collect();
        let mut params = HashMap::new();
        params.insert("id_list".to_string(), Value::Array(ids));

        Ok(("".to_string(), query, params))
    }

    fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_nodes called -- query: {}, partition_key_opt: {:#?}, params_opt: {:#?}, info.name: {}", 
        query, partition_key_opt, params_opt, info.name());

        let params = params_opt.unwrap_or_else(HashMap::new);
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });

        trace!(
            "GremlinTransaction::read_nodes -- query: {}, params: {:#?}",
            query,
            params
        );
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
                query.push(')');
            }
            query.push(')');
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
            ClauseType::Parameter => "outE()".to_string(),
            ClauseType::FirstSubQuery => ".E()".to_string(),
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

    fn rel_read_by_ids_query<RequestCtx: RequestContext>(
        &mut self,
        rel_var: &RelQueryVar,
        rels: Vec<Rel<RequestCtx>>,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!(
            "GremlinTransaction:rel_read_by_ids_query called -- rel_var: {:#?}, rels: {:#?}",
            rel_var,
            rels
        );

        let mut query = ".hasLabel('".to_string() + rel_var.label() + "')";

        if self.partition {
            query.push_str(".has('partitionKey', partitionKey)");
        }
        query.push_str(&(".hasId(within(id_list))"));

        let ids = rels
            .iter()
            .map(|r| r.id())
            .collect::<Vec<&Value>>()
            .into_iter()
            .cloned()
            .collect();
        let mut params = HashMap::new();
        params.insert("id_list".to_string(), Value::Array(ids));

        Ok(("".to_string(), query, params))
    }

    fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
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
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        match_query: &str,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_nodes called: match_query: {}, params: {:#?}, node_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}",
        match_query, params, node_var, props, partition_key_opt);

        let query = "g".to_string() + match_query;

        let (mut q, p) = GremlinTransaction::add_properties(query, props, params);
        q.push_str(NODE_RETURN_FRAGMENT);

        trace!("GremlinTransaction::update_nodes -- q: {}, p: {:#?}", q, p);
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
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

        let raw_results = self.client.execute(q, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::nodes(results, info)
    }

    fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        match_query: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_rels called -- match_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        match_query, params, rel_var, props, props_type_name, partition_key_opt);

        let first = "g".to_string() + match_query;
        let (query, p) = GremlinTransaction::add_properties(first, props, params);
        let q = GremlinTransaction::add_rel_return(query);

        trace!(
            "GremlinTransaction::update_rels -- query: {}, params: {:#?}",
            q,
            p
        );

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
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

        let raw_results = self.client.execute(q, param_list.as_slice())?;
        let results = raw_results
            .map(|r| Ok(r?))
            .collect::<Result<Vec<GValue>, Error>>()?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn delete_nodes(
        &mut self,
        match_query: &str,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_nodes called -- match_query: {}, params: {:#?}, node_var: {:#?}, partition_key_opt: {:#?}", 
        match_query, params, node_var, partition_key_opt);

        let query = "g".to_string() + match_query + ".sideEffect(drop()).count()";
        // let query = match_query.to_string() + ".sideEffect(drop()).count()";

        trace!(
            "GremlinTransaction::delete_nodes -- query: {}, params: {:#?}",
            query,
            params
        );

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

    fn delete_rels(
        &mut self,
        match_query: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_rels called -- match_query: {}, params: {:#?}, rel_var: {:#?}, partition_key_opt: {:#?}",
        match_query, params, rel_var, partition_key_opt);

        let query = "g".to_string() + match_query + ".sideEffect(drop()).count()";
        // let query = match_query.to_string() + ".sideEffect(drop()).count()";

        trace!(
            "GremlinTransaction::delete_rels -- query: {}, params: {:#?}",
            query,
            params
        );

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
    #[cfg(feature = "cosmos")]
    use super::CosmosEndpoint;
    #[cfg(feature = "gremlin")]
    use super::GremlinEndpoint;
    use super::GremlinTransaction;

    #[cfg(feature = "cosmos")]
    #[test]
    fn test_cosmos_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosEndpoint>();
    }

    #[cfg(feature = "cosmos")]
    #[test]
    fn test_cosmos_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosEndpoint>();
    }

    #[cfg(feature = "gremlin")]
    #[test]
    fn test_gremlin_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<GremlinEndpoint>();
    }

    #[cfg(feature = "gremlin")]
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
