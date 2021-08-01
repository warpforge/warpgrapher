//! Provides database interface types and functions for Gremlin-based DBs

use crate::engine::context::RequestContext;
use crate::engine::database::env_bool;
use crate::engine::database::{
    env_string, env_u16, Comparison, DatabaseEndpoint, DatabasePool, NodeQueryVar, Operation,
    QueryFragment, QueryResult, RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::{Info, NodeType};
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use gremlin_client::aio::GremlinClient;
use gremlin_client::TlsOptions;
use gremlin_client::{ConnectionOptions, GKey, GValue, GraphSON, Map, ToGValue, VertexProperty};
use juniper::futures::TryStreamExt;
use log::trace;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::env::var_os;
use std::fmt::Debug;
#[cfg(feature = "gremlin")]
use uuid::Uuid;

static NODE_RETURN_FRAGMENT: &str =
    ".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())";

static REL_RETURN_FRAGMENT: &str = ".project('rID', 'rProps', 'srcID', 'srcLabel', 'dstID', 'dstLabel').by(id()).by(valueMap()).by(outV().id()).by(outV().label()).by(inV().id()).by(inV().label())";

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
#[derive(Clone, Debug)]
pub struct GremlinEndpoint {
    host: String,
    read_replica: String,
    port: u16,
    user: Option<String>,
    pass: Option<String>,
    use_tls: bool,
    validate_certs: bool,
    bindings: bool,
    long_ids: bool,
    partitions: bool,
    sessions: bool,
    version: GraphSON,
    pool_size: u16,
}

impl GremlinEndpoint {
    /// Reads a set of environment variables to construct a [`GremlinEndpoint`]. The environment
    /// variables are as follows
    ///
    /// * WG_GREMLIN_HOST - the hostname for the Gremlin-based DB. For example, `localhost`.
    /// * WG_GREMLIN_READ_REPLICA - a separate host name for read-only replica nodes, if being
    ///   used for additional scalability. If not set, the read pool connects to the same host as
    ///   the read/write connection pool.
    /// * WG_GREMLIN_PORT - the port number for the Gremlin-based DB. For example, `443`.
    /// * WG_GREMLIN_USER - the username for the Gremlin-based DB, if required. For example,
    ///   `warpuser`.
    /// * WG_GREMLIN_PASS - the password used to authenticate the user, if required.
    /// * WG_GREMLIN_USE_TLS - true if Warpgrapher should use TLS to connect to gremlin endpoint.
    ///   Defaults to true.
    /// * WG_GREMLIN_VALIDATE_CERTS - false if Warpgrapher should reject invalid certs for TLS
    ///   connections. true to validate certs. It may be necessary to set to false in test
    ///   environments, but it should be set to true in production environments. Defaults to true.
    /// * WG_GREMLIN_BINDINGS - true if Warpgrapher should use Gremlin bindings to send values
    ///   in queries (effectively query parameterization), and `false` if values should be
    ///   sanitized and sent inline in the query string itself. Defaults to `true`.
    /// * WG_GREMLIN_LONG_IDS - true if Warpgrapher should use long integers as vertex and edge ids
    ///   in the database; false if Warpgrapher should use strings. All identifiers are of type ID
    ///   in the GraphQL schema, which GraphQL serializes as strings. Defaults to false.
    /// * WG_GREMLIN_PARTITIONS - true if Warpgrapher should require a partition ID, and false if
    ///   Warpgrapher should ignore or omit partition IDs. Defaults to `false`.
    /// * WG_GREMLIN_SESSIONS - true if Warpgrapher mutations should be conducted within a single
    ///   Gremlin session, which in some databases provides transactional semantics, and `false` if
    ///   sessions should not be used. Defaults to `false`.
    /// * WG_GREMLIN_VERSION - may be set to `1`, `2`, or `3`, to indicate the version of GraphSON
    ///   serialization that should be used in communicating with the database. Defaults to `3`.
    /// * WG_POOL_SIZE - connection pool size
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
        let host = env_string("WG_GREMLIN_HOST")?;

        Ok(GremlinEndpoint {
            host: host.clone(),
            read_replica: env_string("WG_GREMLIN_READ_REPLICA").unwrap_or(host),
            port: env_u16("WG_GREMLIN_PORT")?,
            user: var_os("WG_GREMLIN_USER").map(|osstr| osstr.to_string_lossy().into_owned()),
            pass: var_os("WG_GREMLIN_PASS").map(|osstr| osstr.to_string_lossy().into_owned()),
            use_tls: env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true),
            validate_certs: env_bool("WG_GREMLIN_VALIDATE_CERTS").unwrap_or(true),
            bindings: env_bool("WG_GREMLIN_BINDINGS").unwrap_or(true),
            long_ids: env_bool("WG_GREMLIN_LONG_IDS").unwrap_or(false),
            partitions: env_bool("WG_GREMLIN_PARTITIONS").unwrap_or(false),
            sessions: env_bool("WG_GREMLIN_SESSIONS").unwrap_or(false),
            version: match env_u16("WG_GREMLIN_VERSION").unwrap_or(3) {
                1 => GraphSON::V1,
                2 => GraphSON::V2,
                _ => GraphSON::V3,
            },
            pool_size: env_u16("WG_POOL_SIZE")
                .unwrap_or_else(|_| num_cpus::get().try_into().unwrap_or(8)),
        })
    }
}

#[async_trait]
impl DatabaseEndpoint for GremlinEndpoint {
    type PoolType = GremlinPool;

    async fn pool(&self) -> Result<Self::PoolType, Error> {
        let mut ro_options_builder = ConnectionOptions::builder()
            .host(&self.read_replica)
            .port(self.port)
            .pool_size(self.pool_size.into())
            .serializer(self.version.clone())
            .deserializer(self.version.clone());
        if let (Some(user), Some(pass)) = (self.user.as_ref(), self.pass.as_ref()) {
            ro_options_builder = ro_options_builder.credentials(user, pass);
        }
        if self.use_tls {
            ro_options_builder = ro_options_builder
                .ssl(self.use_tls)
                .tls_options(TlsOptions {
                    accept_invalid_certs: !self.validate_certs,
                });
        }
        let ro_options = ro_options_builder.build();

        let mut rw_options_builder = ConnectionOptions::builder()
            .host(&self.host)
            .port(self.port)
            .pool_size(self.pool_size.into())
            .serializer(self.version.clone())
            .deserializer(self.version.clone());
        if let (Some(user), Some(pass)) = (self.user.as_ref(), self.pass.as_ref()) {
            rw_options_builder = rw_options_builder.credentials(user, pass);
        }
        if self.use_tls {
            rw_options_builder = rw_options_builder
                .ssl(self.use_tls)
                .tls_options(TlsOptions {
                    accept_invalid_certs: !self.validate_certs,
                });
        }
        let rw_options = rw_options_builder.build();

        #[allow(clippy::eval_order_dependence)]
        Ok(GremlinPool::new(
            GremlinClient::connect(ro_options).await?,
            GremlinClient::connect(rw_options).await?,
            self.bindings,
            self.long_ids,
            self.partitions,
            self.sessions,
        ))
    }
}

#[derive(Clone)]
pub struct GremlinPool {
    ro_pool: GremlinClient,
    rw_pool: GremlinClient,
    bindings: bool,
    long_ids: bool,
    partitions: bool,
    sessions: bool,
}

impl GremlinPool {
    fn new(
        ro_pool: GremlinClient,
        rw_pool: GremlinClient,
        bindings: bool,
        long_ids: bool,
        partitions: bool,
        sessions: bool,
    ) -> Self {
        GremlinPool {
            ro_pool,
            rw_pool,
            bindings,
            long_ids,
            partitions,
            sessions,
        }
    }
}

#[async_trait]
impl DatabasePool for GremlinPool {
    type TransactionType = GremlinTransaction;

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(
            self.ro_pool.clone(),
            self.bindings,
            self.long_ids,
            self.partitions,
            false,
        ))
    }

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(if self.sessions {
            GremlinTransaction::new(
                self.rw_pool
                    .clone()
                    .create_session(Uuid::new_v4().to_hyphenated().to_string())
                    .await?,
                self.bindings,
                self.long_ids,
                self.partitions,
                self.sessions,
            )
        } else {
            GremlinTransaction::new(
                self.rw_pool.clone(),
                self.bindings,
                self.long_ids,
                self.partitions,
                self.sessions,
            )
        })
    }
}

pub struct GremlinTransaction {
    client: GremlinClient,
    bindings: bool,
    long_ids: bool,
    partitions: bool,
    sessions: bool,
}

impl GremlinTransaction {
    pub fn new(
        client: GremlinClient,
        bindings: bool,
        long_ids: bool,
        partitions: bool,
        sessions: bool,
    ) -> Self {
        GremlinTransaction {
            client,
            bindings,
            long_ids,
            partitions,
            sessions,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_properties(
        mut query: String,
        mut props: HashMap<String, Value>,
        mut params: HashMap<String, Value>,
        note_singles: bool,
        use_bindings: bool,
        create_query: bool,
        long_ids: bool,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        if long_ids {
            if let Some(Value::String(s)) = props.remove("id") {
                let id = if let Ok(i) = s.parse::<i64>() {
                    Value::Int64(i)
                } else {
                    Value::String(s)
                };
                props.insert("id".to_string(), id);
            }
        }

        if create_query {
            if let Some(id_val) = props.remove("id") {
                if use_bindings {
                    let suffix = sg.suffix();
                    query.push_str(&*(".property(id, ".to_string() + "id" + &*suffix + ")"));
                    params.insert("id".to_string() + &*suffix, id_val);
                } else {
                    query.push_str(
                        &(".property(id, ".to_string() + &*id_val.to_property_value()? + ")"),
                    );
                }
            }
        }

        props
            .into_iter()
            .try_fold((query, params), |(mut outer_q, mut outer_p), (k, v)| {
                if let Value::Array(a) = v {
                    if !use_bindings {
                        outer_q.push_str(
                            &*(".sideEffect(properties('".to_string() + &*k + "').drop())"),
                        );
                    }
                    a.into_iter()
                        .try_fold((outer_q, outer_p), |(mut inner_q, mut inner_p), val| {
                            if use_bindings {
                                let suffix = sg.suffix();
                                inner_q.push_str(
                                    &*(".property(list, '".to_string()
                                        + &*k
                                        + "', "
                                        + &*k
                                        + &*suffix
                                        + ")"),
                                );
                                inner_p.insert(k.to_string() + &*suffix, val);
                            } else {
                                inner_q.push_str(
                                    // Use
                                    &*(".property(set, '".to_string()
                                        + &*k
                                        + "', "
                                        + &*val.to_property_value()?
                                        + ")"),
                                );
                            };
                            Ok((inner_q, inner_p))
                        })
                } else if use_bindings {
                    let suffix = sg.suffix();
                    outer_q.push_str(
                        &*(".property(".to_string()
                            + if note_singles { "single, " } else { "" }
                            + "'"
                            + &*k
                            + "', "
                            + &*k
                            + &*suffix
                            + ")"),
                    );
                    outer_p.insert(k + &*suffix, v);
                    Ok((outer_q, outer_p))
                } else {
                    outer_q.push_str(
                        &*(".property(".to_string()
                            + if note_singles { "single, " } else { "" }
                            + "'"
                            + &*k
                            + "', "
                            + &*v.to_property_value()?
                            + ")"),
                    );
                    Ok((outer_q, outer_p))
                }
            })
    }

    fn extract_count(results: Vec<GValue>) -> Result<i32, Error> {
        if let Some(GValue::Int32(i)) = results.get(0) {
            Ok(*i)
        } else if let Some(GValue::Int64(i)) = results.get(0) {
            Ok(i32::try_from(*i)?)
        } else {
            Err(Error::TypeNotExpected { details: Some("extract_count value is not GValue Int32 or Int64".to_string()) })
        }
    }

    fn extract_node_properties(
        props: Map,
        type_def: &NodeType,
    ) -> Result<HashMap<String, Value>, Error> {
        trace!(
            "GremlinTransaction::extract_node_properties called: {:#?}",
            props
        );
        props
            .into_iter()
            .map(|(key, val)| {
                if let (GKey::String(k), GValue::List(plist)) = (key.clone(), val.clone()) {
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
                } else if let GKey::String(k) = key {
                    Ok((k, val.try_into()?))
                } else {
                    Err(Error::TypeNotExpected { details: Some("GValue is not String or List".to_string()) })
                }
            })
            .collect::<Result<HashMap<String, Value>, Error>>()
    }

    fn gmap_to_hashmap(gv: GValue) -> Result<HashMap<String, GValue>, Error> {
        if let GValue::Map(map) = gv {
            map.into_iter()
                .map(|(k, v)| match (k, v) {
                    (GKey::String(s), v) => Ok((s, v)),
                    (_, _) => Err(Error::TypeNotExpected { details: Some("GValue is not String".to_string()) }),
                })
                .collect()
        } else {
            Err(Error::TypeNotExpected { details: Some("GValue is not Map".to_string()) })
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

                let id = match hm.remove("nID") {
                    Some(GValue::String(s)) => Value::String(s),
                    Some(GValue::Int64(i)) => Value::String(i.to_string()),
                    Some(GValue::Uuid(uuid)) => Value::Uuid(uuid),
                    _ => {
                        return Err(Error::ResponseItemNotFound {
                            name: "Node id".to_string(),
                        })
                    }
                };

                if let (Some(GValue::String(label)), Some(GValue::Map(props))) =
                    (hm.remove("nLabel"), hm.remove("nProps"))
                {
                    let type_def = info.type_def_by_name(&label)?;
                    let mut fields = GremlinTransaction::extract_node_properties(props, type_def)?;
                    fields.insert("id".to_string(), id);
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

                let rel_id = match hm.remove("rID") {
                    Some(GValue::String(s)) => Value::String(s),
                    Some(GValue::Int64(i)) => Value::String(i.to_string()),
                    Some(GValue::Uuid(uuid)) => Value::Uuid(uuid),
                    _ => {
                        return Err(Error::ResponseItemNotFound {
                            name: "Rel ID".to_string(),
                        })
                    }
                };

                let src_id = match hm.remove("srcID") {
                    Some(GValue::String(s)) => Value::String(s),
                    Some(GValue::Int64(i)) => Value::String(i.to_string()),
                    Some(GValue::Uuid(uuid)) => Value::Uuid(uuid),
                    _ => {
                        return Err(Error::ResponseItemNotFound {
                            name: "Src ID".to_string(),
                        })
                    }
                };

                let dst_id = match hm.remove("dstID") {
                    Some(GValue::String(s)) => Value::String(s),
                    Some(GValue::Int64(i)) => Value::String(i.to_string()),
                    Some(GValue::Uuid(uuid)) => Value::Uuid(uuid),
                    _ => {
                        return Err(Error::ResponseItemNotFound {
                            name: "Dst ID".to_string(),
                        })
                    }
                };

                if let (
                    Some(GValue::Map(rel_props)),
                    Some(GValue::String(src_label)),
                    Some(GValue::String(dst_label)),
                ) = (
                    hm.remove("rProps"),
                    hm.remove("srcLabel"),
                    hm.remove("dstLabel"),
                ) {
                    let rel_fields = rel_props
                        .into_iter()
                        .map(|(key, val)| {
                            if let GKey::String(k) = key {
                                Ok((k, val.try_into()?))
                            } else {
                                Err(Error::TypeNotExpected { details: Some("GKey is not String".to_string()) })
                            }
                        })
                        .collect::<Result<HashMap<String, Value>, Error>>()?;

                    Ok(Rel::new(
                        rel_id,
                        partition_key_opt.cloned(),
                        props_type_name.map(|ptn| Node::new(ptn.to_string(), rel_fields)),
                        NodeRef::Identifier {
                            id: src_id,
                            label: src_label,
                        },
                        NodeRef::Identifier {
                            id: dst_id,
                            label: dst_label,
                        },
                    ))
                } else {
                    Err(Error::ResponseItemNotFound {
                        name: "Rel props, src label, or dst label".to_string(),
                    })
                }
            })
            .collect()
    }
}

#[async_trait]
impl Transaction for GremlinTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    #[tracing::instrument(level="info", name="wg-gremlin-execute-query", skip(self, query, params))]
    async fn execute_query<RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
    ) -> Result<QueryResult, Error> {
        trace!(
            "GremlinTransaction::execute_query called -- query: {}, params: {:#?}",
            query,
            params
        );

        let param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        trace!(
            "GremlinTransaction::execute_query -- results: {:#?}",
            results
        );

        Ok(QueryResult::Gremlin(results))
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-create-nodes",
        skip(self, node_var, props, partition_key_opt, info, sg)
    )]
    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error> {
        trace!("GremlinTransaction::create_node called -- node_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}", node_var, props, partition_key_opt);

        let mut query = "g.addV('".to_string() + node_var.label()? + "')";

        if self.partitions {
            query.push_str(".property('partitionKey', partitionKey)");
        }

        let (mut q, p) = GremlinTransaction::add_properties(
            query,
            props,
            HashMap::new(),
            true,
            self.bindings,
            true,
            self.long_ids,
            sg,
        )?;
        q += NODE_RETURN_FRAGMENT;

        trace!("GremlinTransaction::create_node -- q: {}, p: {:#?}", q, p);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;
        trace!("GremlinTransaction::create_node -- results: {:#?}", results);

        GremlinTransaction::nodes(results, info)?
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-create-rels",
        skip(
            self,
            src_fragment,
            dst_fragment,
            rel_var,
            props,
            props_type_name,
            partition_key_opt,
            sg
        )
    )]
    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_fragment: QueryFragment,
        dst_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        id_opt: Option<Value>,
        mut props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::create_rels called -- src_fragment: {:#?}, dst_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        src_fragment, dst_fragment, rel_var, props, props_type_name, partition_key_opt);

        let query = "g.V()".to_string()
            + src_fragment.where_fragment()
            + ".as('"
            + rel_var.src().name()
            + "')"
            + ".V()"
            + dst_fragment.where_fragment()
            + ".as('"
            + rel_var.dst().name()
            + "')"
            + ".addE('"
            + rel_var.label()
            + "').from('"
            + rel_var.src().name()
            + "').to('"
            + rel_var.dst().name()
            + "')";
        let mut params = src_fragment.params();
        params.extend(dst_fragment.params());

        if let Some(id_val) = id_opt {
            props.insert("id".to_string(), id_val);
        }
        let (mut q, p) = GremlinTransaction::add_properties(
            query,
            props,
            params,
            false,
            self.bindings,
            true,
            self.long_ids,
            sg,
        )?;

        q.push_str(REL_RETURN_FRAGMENT);

        trace!("GremlinTransaction::create_rels -- q: {}, p: {:#?}", q, p);

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    fn node_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        nodes: &[Node<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        trace!(
            "GremlinTransaction::node_read_by_ids_query called -- node_var: {:#?}, nodes: {:#?}",
            node_var,
            nodes
        );

        let mut query = ".hasLabel('".to_string() + node_var.label()? + "')";

        if self.partitions {
            query.push_str(".has('partitionKey', partitionKey)");
        }
        let mut params = HashMap::new();

        if self.bindings {
            query.push_str(".hasId(within(id_list))");
            let ids = nodes
                .iter()
                .map(|n| n.id())
                .collect::<Result<Vec<&Value>, Error>>()?
                .into_iter()
                .cloned()
                .collect::<Vec<Value>>()
                .into_iter()
                .map(|id| {
                    if self.long_ids {
                        if let Value::String(s) = id {
                            if let Ok(i) = s.parse::<i64>() {
                                Value::Int64(i)
                            } else {
                                Value::String(s)
                            }
                        } else {
                            id
                        }
                    } else {
                        id
                    }
                })
                .collect();
            params.insert("id_list".to_string(), Value::Array(ids));
        } else {
            let fragment = nodes.iter().enumerate().try_fold(
                ".hasId(within(".to_string(),
                |mut acc, (i, n)| -> Result<String, Error> {
                    if i > 0 {
                        acc.push_str(", ");
                    }

                    acc.push_str(
                        &*(n.id().and_then(|id| {
                            if self.long_ids {
                                if let Value::String(s) = id {
                                    if let Ok(i) = s.parse::<i64>() {
                                        return Value::Int64(i).to_property_value();
                                    }
                                }
                            }
                            id.to_property_value()
                        })?),
                    );
                    Ok(acc)
                },
            )?;
            query.push_str(&*(fragment + "))"));
        }

        Ok(QueryFragment::new("".to_string(), query, params))
    }

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<QueryFragment>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        trace!("GremlinTransaction::node_read_fragment called -- rel_query_fragments: {:#?}, node_var: {:#?}, props: {:#?}, sg: {:#?}", 
        rel_query_fragments, node_var, props, sg);

        let param_suffix = sg.suffix();

        let mut query = String::new();
        let mut params = HashMap::new();

        if node_var.label().is_ok() {
            query.push_str(&(".hasLabel('".to_string() + node_var.label()? + "')"));
        }

        if self.partitions {
            query.push_str(".has('partitionKey', partitionKey)");
        }

        for (k, c) in props.into_iter() {
            query.push_str(
                &(".has".to_string()
                + "("
                + if k=="id" { "" } else { "'" }  // omit quotes if key is id because it's a "system" property
                + &*k
                + if k=="id" { "" } else { "'" }  // omit quotes if key is id because it's a "system" property
                + ", " 
                + &*gremlin_comparison_operator(&c)
                + "("
                + &*(if self.bindings { k.clone() + &*param_suffix } else { c.operand.to_property_value()? })
                + "))"),
            );

            if self.bindings {
                params.insert(k + &*param_suffix, c.operand);
            }
        }

        if !rel_query_fragments.is_empty() {
            query.push_str(".where(");

            let multi_fragment = rel_query_fragments.len() > 1;

            if multi_fragment {
                query.push_str("and(");
            }

            rel_query_fragments
                .into_iter()
                .enumerate()
                .for_each(|(i, rqf)| {
                    if i == 0 {
                        query.push_str(&("outE()".to_string() + rqf.where_fragment()));
                    } else {
                        query.push_str(&(", outE()".to_string() + rqf.where_fragment()));
                    }

                    params.extend(rqf.params());
                });

            if multi_fragment {
                query.push(')');
            }

            query.push(')');
        }

        let qf = QueryFragment::new("".to_string(), query, params);

        trace!(
            "GremlinTransaction::node_read_fragment returning -- {:#?}",
            qf
        );

        Ok(qf)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-read-nodes",
        skip(self, _node_var, query_fragment, partition_key_opt, info)
    )]
    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        _node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_nodes called -- query_fragment: {:#?}, partition_key_opt: {:#?}, info.name: {}", 
        query_fragment, partition_key_opt, info.name());

        let query = "g.V()".to_string() + query_fragment.where_fragment() + NODE_RETURN_FRAGMENT;
        let params = query_fragment.params();

        trace!(
            "GremlinTransaction::read_nodes -- query: {}, params: {:#?}",
            query,
            params
        );
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::nodes(results, info)
    }

    fn rel_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        rel_var: &RelQueryVar,
        rels: &[Rel<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        trace!(
            "GremlinTransaction:rel_read_by_ids_query called -- rel_var: {:#?}, rels: {:#?}",
            rel_var,
            rels
        );

        let mut query = ".hasLabel('".to_string() + rel_var.label() + "')";

        if self.partitions {
            query.push_str(".has('partitionKey', partitionKey)");
        }

        let mut params = HashMap::new();

        if self.bindings {
            query.push_str(&(".hasId(within(id_list))"));

            let ids = rels
                .iter()
                .map(|r| r.id())
                .collect::<Vec<&Value>>()
                .into_iter()
                .cloned()
                .collect::<Vec<Value>>()
                .into_iter()
                .map(|id| {
                    if self.long_ids {
                        if let Value::String(s) = id {
                            if let Ok(i) = s.parse::<i64>() {
                                Value::Int64(i)
                            } else {
                                Value::String(s)
                            }
                        } else {
                            id
                        }
                    } else {
                        id
                    }
                })
                .collect();
            params.insert("id_list".to_string(), Value::Array(ids));
        } else {
            let fragment = rels.iter().enumerate().try_fold(
                ".hasId(within(".to_string(),
                |mut acc, (i, r)| -> Result<String, Error> {
                    if i > 0 {
                        acc.push_str(", ");
                    }

                    acc.push_str(
                        &*(if self.long_ids {
                            if let Value::String(s) = r.id() {
                                if let Ok(i) = s.parse::<i64>() {
                                    Value::Int64(i).to_property_value()
                                } else {
                                    r.id().to_property_value()
                                }
                            } else {
                                r.id().to_property_value()
                            }
                        } else {
                            r.id().to_property_value()
                        })?,
                    );
                    Ok(acc)
                },
            )?;
            query.push_str(&*(fragment + "))"));
        }

        Ok(QueryFragment::new("".to_string(), query, params))
    }

    fn rel_read_fragment(
        &mut self,
        src_fragment_opt: Option<QueryFragment>,
        dst_fragment_opt: Option<QueryFragment>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        trace!("GremlinTransaction::rel_read_fragment called -- src_fragment_opt: {:#?}, dst_fragment_opt: {:#?}, rel_var: {:#?}, props: {:#?}",
        src_fragment_opt, dst_fragment_opt, rel_var, props);

        let param_suffix = sg.suffix();
        let mut query = ".hasLabel('".to_string() + rel_var.label() + "')";
        let mut params = HashMap::new();

        if self.partitions {
            query.push_str(".has('partitionKey', partitionKey)");
        }

        for (k, c) in props.into_iter() {
            query.push_str(
                &(".has".to_string()
                + "("
                + if k=="id" { "" } else { "'" }  // ommit quotes if key is id because it's a "system" property
                + &*k
                + if k=="id" { "" } else { "'" }  // ommit quotes if key is id because it's a "system" property
                + ", " 
                + &*gremlin_comparison_operator(&c)
                + "("
                + &*(if self.bindings {k.clone() + &*param_suffix} else {c.operand.to_property_value()?})
                + ")"
                + ")"),
            );
            if self.bindings {
                params.insert(k + &*param_suffix, c.operand);
            }
        }

        if src_fragment_opt.is_some() || dst_fragment_opt.is_some() {
            query.push_str(".where(");

            let both = src_fragment_opt.is_some() && dst_fragment_opt.is_some();

            if both {
                query.push_str("and(");
            }

            if let Some(src_fragment) = src_fragment_opt {
                query.push_str(&("outV()".to_string() + src_fragment.where_fragment()));

                params.extend(src_fragment.params());
            }

            if both {
                query.push_str(", ");
            }

            if let Some(dst_fragment) = dst_fragment_opt {
                query.push_str(&("inV()".to_string() + dst_fragment.where_fragment()));

                params.extend(dst_fragment.params());
            }

            if both {
                query.push(')');
            }
            query.push(')');
        }

        Ok(QueryFragment::new(String::new(), query, params))
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-read-rels",
        skip(self, query_fragment, rel_var, props_type_name, partition_key_opt)
    )]
    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, props_type_name, partition_key_opt);

        let query = "g.E()".to_string() + query_fragment.where_fragment() + REL_RETURN_FRAGMENT;
        let params = query_fragment.params();

        trace!(
            "GremlinTransaction::read_rels -- query: {}, params: {:#?}",
            query,
            params
        );

        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-update-nodes",
        skip(self, query_fragment, node_var, props, partition_key_opt, info, sg)
    )]
    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_nodes called: query_fragment: {:#?}, node_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}, info.name: {}",
        query_fragment, node_var, props, partition_key_opt, info.name());

        let query = "g.V()".to_string() + query_fragment.where_fragment();

        let (mut q, p) = GremlinTransaction::add_properties(
            query,
            props,
            query_fragment.params(),
            true,
            self.bindings,
            false,
            self.long_ids,
            sg,
        )?;
        q.push_str(NODE_RETURN_FRAGMENT);

        trace!("GremlinTransaction::update_nodes -- q: {}, p: {:#?}", q, p);
        let mut param_list: Vec<(&str, &dyn ToGValue)> =
            p.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k.as_str(), v));
                pl
            });

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::nodes(results, info)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-update-rels",
        skip(
            self,
            query_fragment,
            rel_var,
            props,
            props_type_name,
            partition_key_opt,
            sg
        )
    )]
    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, props, props_type_name, partition_key_opt);

        let first = "g.E()".to_string() + query_fragment.where_fragment();
        let (mut q, p) = GremlinTransaction::add_properties(
            first,
            props,
            query_fragment.params(),
            false,
            self.bindings,
            false,
            self.long_ids,
            sg,
        )?;
        q.push_str(REL_RETURN_FRAGMENT);

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

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::rels(results, props_type_name, partition_key_opt)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-delete-nodes",
        skip(self, query_fragment, node_var, partition_key_opt)
    )]
    async fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_nodes called -- query_fragment: {:#?}, node_var: {:#?}, partition_key_opt: {:#?}", 
        query_fragment, node_var, partition_key_opt);

        let query =
            "g.V()".to_string() + query_fragment.where_fragment() + ".sideEffect(drop()).count()";
        let params = query_fragment.params();

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

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::extract_count(results)
    }

    #[tracing::instrument(
        level="info",
        name = "wg-gremlin-delete-rels",
        skip(self, query_fragment, rel_var, partition_key_opt)
    )]
    async fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!("GremlinTransaction::delete_rels called -- query_fragment: {:#?}, rel_var: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, partition_key_opt);

        let query =
            "g.E()".to_string() + query_fragment.where_fragment() + ".sideEffect(drop()).count()";
        let params = query_fragment.params();

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

        if self.partitions {
            if let Some(pk) = partition_key_opt {
                param_list.push(("partitionKey", pk));
            } else {
                return Err(Error::PartitionKeyNotFound);
            }
        }

        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::extract_count(results)
    }

    async fn commit(&mut self) -> Result<(), Error> {
        if self.sessions {
            self.client
                .close_session()
                .await
                .map(|_r| ())
                .map_err(Error::from)
        } else {
            Ok(())
        }
    }

    async fn rollback(&mut self) -> Result<(), Error> {
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
        vp.take::<GValue>()
            .map_err(|_e| Error::TypeConversionFailed {
                src: "VertexProperty".to_string(),
                dst: "Value".to_string(),
            })?
            .try_into()
    }
}

fn gremlin_comparison_operator(c: &Comparison) -> String {
    match (&c.operation, &c.negated) {
        (Operation::EQ, false) => "eq".to_string(),
        (Operation::EQ, true) => "neq".to_string(),
        (Operation::CONTAINS, false) => "containing".to_string(),
        (Operation::CONTAINS, true) => "notContaining".to_string(),
        (Operation::IN, false) => "within".to_string(),
        (Operation::IN, true) => "without".to_string(),
        (Operation::GT, _) => "gt".to_string(),
        (Operation::GTE, _) => "gte".to_string(),
        (Operation::LT, _) => "lt".to_string(),
        (Operation::LTE, _) => "lte".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::GremlinEndpoint;
    use super::GremlinTransaction;
    use crate::engine::database::SuffixGenerator;
    use crate::Value;
    use maplit::hashmap;
    use std::collections::HashMap;
    use uuid::Uuid;

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

    #[test]
    fn test_add_properties_array_with_bindings() {
        let s1 = Value::String("String one".to_string());
        let s2 = Value::String("String two".to_string());
        let a = Value::Array(vec![s1, s2]);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => a},
            HashMap::new(),
            true,
            true,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(
            ".property(list, 'my_prop', my_prop_0)".to_string()
                + ".property(list, 'my_prop', my_prop_1)",
            q
        );
        assert!(p.contains_key(&"my_prop_0".to_string()));
        assert_eq!(
            &Value::String("String one".to_string()),
            p.get(&"my_prop_0".to_string()).unwrap()
        );
        assert!(p.contains_key(&"my_prop_1".to_string()));
        assert_eq!(
            &Value::String("String two".to_string()),
            p.get(&"my_prop_1".to_string()).unwrap()
        );
    }

    #[test]
    fn test_add_properties_array_without_bindings() {
        let s1 = Value::String("String one".to_string());
        let s2 = Value::String("String two".to_string());
        let a = Value::Array(vec![s1, s2]);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => a},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(
            ".sideEffect(properties('my_prop').drop())".to_string()
                + ".property(set, 'my_prop', 'String one')"
                + ".property(set, 'my_prop', 'String two')",
            q
        );
        assert!(p.is_empty());
    }

    #[test]
    fn test_add_properties_scalar_with_bindings() {
        let s1 = Value::String("String one".to_string());

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => s1},
            HashMap::new(),
            true,
            true,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', my_prop_0)".to_string(), q);
        assert!(p.contains_key(&"my_prop_0".to_string()));
        assert_eq!(
            &Value::String("String one".to_string()),
            p.get(&"my_prop_0".to_string()).unwrap()
        );
    }

    #[test]
    fn test_add_properties_scalar_without_bindings() {
        let s1 = Value::String("String one".to_string());

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => s1},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', 'String one')", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_bool_without_bindings() {
        let b = Value::Bool(true);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => b},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', true)", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_float_without_bindings() {
        let f = Value::Float64(3.3);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => f},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', 3.3f)", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_int_without_bindings() {
        let i = Value::Int64(-1);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => i},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', -1L)", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_map_without_bindings() {
        let s1 = Value::String("String one".to_string());
        let hm = hashmap! { "s1".to_string() => s1 };
        let m = Value::Map(hm);

        assert!(GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => m},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .is_err());
    }

    #[test]
    fn test_null_without_bindings() {
        let n = Value::Null;

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => n},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', '')", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_parameterization_without_bindings() {
        let s = Value::String(
            "Doesn't work without parameterizing \\ characters but not \" marks.".to_string(),
        );

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => s},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', 'Doesn\\\'t work without parameterizing \\\\ characters but not \" marks.')", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_uint_without_bindings() {
        let u = Value::UInt64(1);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => u},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', 1)", q);
        assert!(p.is_empty());
    }

    #[test]
    fn test_uuid_without_bindings() {
        let uuid = Uuid::new_v4();

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => Value::Uuid(uuid)},
            HashMap::new(),
            true,
            false,
            true,
            false,
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(
            format!(
                ".property(single, 'my_prop', '{}')",
                uuid.to_hyphenated().to_string()
            ),
            q
        );
        assert!(p.is_empty());
    }
}
