//! Provides database interface types and functions for Gremlin-based DBs

use crate::engine::context::RequestContext;
use crate::engine::database::env_bool;
use crate::engine::database::{
    env_string, env_u16, Comparison, DatabaseEndpoint, DatabasePool, NodeQueryVar, Operation,
    QueryFragment, QueryResult, RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::loader::{NodeLoaderKey, RelLoaderKey};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::{Info, Property};
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use gremlin_client::aio::GremlinClient;
use gremlin_client::TlsOptions;
use gremlin_client::{ConnectionOptions, GKey, GValue, GraphSON, ToGValue, VertexProperty};
use juniper::futures::TryStreamExt;
use log::trace;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::env::var_os;
use std::fmt::Debug;
#[cfg(feature = "gremlin")]
use uuid::Uuid;

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

        println!("Version: {:#?}", env_u16("WG_GREMLIN_VERSION").unwrap_or(3));

        Ok(GremlinEndpoint {
            host: host.clone(),
            read_replica: env_string("WG_GREMLIN_READ_REPLICA").unwrap_or(host),
            port: env_u16("WG_GREMLIN_PORT")?,
            user: var_os("WG_GREMLIN_USER").map(|osstr| osstr.to_string_lossy().into_owned()),
            pass: var_os("WG_GREMLIN_PASS").map(|osstr| osstr.to_string_lossy().into_owned()),
            use_tls: env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true),
            validate_certs: env_bool("WG_GREMLIN_VALIDATE_CERTS").unwrap_or(true),
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
    long_ids: bool,
    partitions: bool,
    sessions: bool,
}

impl GremlinPool {
    fn new(
        ro_pool: GremlinClient,
        rw_pool: GremlinClient,
        long_ids: bool,
        partitions: bool,
        sessions: bool,
    ) -> Self {
        GremlinPool {
            ro_pool,
            rw_pool,
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
                self.long_ids,
                self.partitions,
                self.sessions,
            )
        } else {
            GremlinTransaction::new(
                self.rw_pool.clone(),
                self.long_ids,
                self.partitions,
                self.sessions,
            )
        })
    }
}

pub struct GremlinTransaction {
    client: GremlinClient,
    long_ids: bool,
    partitions: bool,
    sessions: bool,
}

impl GremlinTransaction {
    pub fn new(client: GremlinClient, long_ids: bool, partitions: bool, sessions: bool) -> Self {
        GremlinTransaction {
            client,
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
                let suffix = sg.suffix();
                query.push_str(&*(".property(id, ".to_string() + "id" + &*suffix + ")"));
                params.insert("id".to_string() + &*suffix, id_val);
            }
        }

        props
            .into_iter()
            .try_fold((query, params), |(mut outer_q, mut outer_p), (k, v)| {
                if let Value::Array(a) = v {
                    a.into_iter()
                        .try_fold((outer_q, outer_p), |(mut inner_q, mut inner_p), val| {
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
                            Ok((inner_q, inner_p))
                        })
                } else {
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
                }
            })
    }
}

#[async_trait]
impl Transaction for GremlinTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    #[tracing::instrument(
        level = "info",
        name = "wg-gremlin-execute-query",
        skip(self, query, params)
    )]
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
        level = "info",
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
            true,
            self.long_ids,
            sg,
        )?;

        q.push_str(".valueMap(true)");

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
        let mut results: Vec<GValue> = raw_results.try_collect().await?;
        trace!("GremlinTransaction::create_node -- results: {:#?}", results);

        Ok((results.pop().ok_or(Error::ResponseSetNotFound)?, info).try_into()?)
    }

    #[tracing::instrument(
        level = "info",
        name = "wg-gremlin-create-rels",
        skip(
            self,
            src_fragment,
            dst_fragment,
            rel_var,
            props,
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
        partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::create_rels called -- src_fragment: {:#?}, dst_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}",
        src_fragment, dst_fragment, rel_var, props, partition_key_opt);

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
            true,
            self.long_ids,
            sg,
        )?;

        q.push_str(
            ".project('src_id', 'rel', 'dst_id').by(outV().id()).by(valueMap(true)).by(inV().id())",
        );

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
        let results: Vec<GValue> = raw_results.try_collect().await?;

        trace!("create_rels -- results: {:#?}", results);

        results
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
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
                + &*(k.clone() + &*param_suffix)
                + "))"),
            );

            params.insert(k + &*param_suffix, c.operand);
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

    #[tracing::instrument(level = "info", name = "wg-gremlin-load-rels", skip(self, info))]
    async fn load_nodes<RequestCtx: RequestContext>(
        &mut self,
        keys: &[NodeLoaderKey],
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("GremlinTransaction::load_nodes called -- keys: {:#?}", keys);

        let mut sg = SuffixGenerator::new();
        let mut query = String::new();
        let mut params = HashMap::new();
        for (i, nlk) in keys.iter().enumerate() {
            let suffix = sg.suffix();
            if i == 0 {
                query.push_str(&("g.V().union(has(id, id".to_string() + &*suffix + ")"));
            } else {
                query.push_str(&(", has(id, id".to_string() + &*suffix + ")"));
            }
            params.insert(
                "id".to_string() + &*suffix,
                Value::String(nlk.id().to_string()),
            );

            if self.partitions {
                let pk_suffix = sg.suffix();
                query.push_str(&(".has('partitionKey', pk".to_string() + &*pk_suffix + ")"));
                params.insert(
                    "pk".to_string() + &*pk_suffix,
                    Value::String(nlk.partition_key_opt().clone().ok_or(
                        Error::InputItemNotFound {
                            name: "partitionKey".to_string(),
                        },
                    )?),
                );
            }
        }
        query.push_str(").valueMap(true)");

        trace!("GremlinTransaction::load_nodes -- query: {}", query,);

        let param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });
        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results: Vec<GValue> = raw_results.try_collect().await?;

        trace!("GremlinTransaction::load_nodes -- results: {:#?}", results);

        results
            .into_iter()
            .map(|r| (r, info).try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        level = "info",
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

        let query = "g.V()".to_string() + query_fragment.where_fragment() + ".valueMap(true)";
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
        let results: Vec<GValue> = raw_results.try_collect().await?;

        results
            .into_iter()
            .map(|n| (n, info).try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
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

        query.push_str(".hasId(within(id_list))");

        let ids = rels
            .iter()
            .map(|r| r.id())
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
                + &*(k.clone() + &*param_suffix)
                + ")"
                + ")"),
            );
            params.insert(k + &*param_suffix, c.operand);
        }

        query.push_str(".where(");

        if dst_fragment_opt.is_some() {
            query.push_str("and(");
        }

        query.push_str(&("outV().hasLabel('".to_string() + rel_var.src.label()? + "')"));

        if let Some(src_fragment) = src_fragment_opt {
            query.push_str(src_fragment.where_fragment());
            params.extend(src_fragment.params());
        }

        if let Some(dst_fragment) = dst_fragment_opt {
            query.push_str(", ");
            query.push_str(&(", inV()".to_string() + dst_fragment.where_fragment() + ")"));
            params.extend(dst_fragment.params());
        }

        query.push(')');

        Ok(QueryFragment::new(String::new(), query, params))
    }

    #[tracing::instrument(level = "info", name = "wg-gremlin-load-rels", skip(self))]
    async fn load_rels<RequestCtx: RequestContext>(
        &mut self,
        keys: &[RelLoaderKey],
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::load_rels called -- keys: {:#?}", keys);

        let mut sg = SuffixGenerator::new();
        let mut query = String::new();
        let mut params = HashMap::new();
        for (i, rlk) in keys.iter().enumerate() {
            let suffix = sg.suffix();
            if i == 0 {
                query.push_str(
                    &("g.E().union(hasLabel('".to_string()
                        + rlk.rel_name()
                        + "').where(outV().has(id, id"
                        + &*suffix
                        + "))"),
                );
            } else {
                query.push_str(
                    &(", hasLabel('".to_string()
                        + rlk.rel_name()
                        + "').where(outV().has(id, id"
                        + &*suffix
                        + "))"),
                );
            }
            params.insert(
                "id".to_string() + &*suffix,
                Value::String(rlk.src_id().to_string()),
            );

            if self.partitions {
                let pk_suffix = sg.suffix();
                query.push_str(&(".has('partitionKey', pk".to_string() + &*pk_suffix + ")"));
                params.insert(
                    "pk".to_string() + &*pk_suffix,
                    Value::String(rlk.partition_key_opt().clone().ok_or(
                        Error::InputItemNotFound {
                            name: "partitionKey".to_string(),
                        },
                    )?),
                );
            }
        }
        query.push_str(
            ").project('src_id', 'rel', 'dst_id').by(outV().id()).by(valueMap(true)).by(inV().id())",
        );
        trace!("GremlinTransaction::load_rels -- query: {}", query,);

        let param_list: Vec<(&str, &dyn ToGValue)> =
            params.iter().fold(Vec::new(), |mut pl, (k, v)| {
                pl.push((k, v));
                pl
            });
        let raw_results = self.client.execute(query, param_list.as_slice()).await?;
        let results: Vec<GValue> = raw_results.try_collect().await?;

        results
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        level = "info",
        name = "wg-gremlin-read-rels",
        skip(self, query_fragment, rel_var, partition_key_opt)
    )]
    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::read_rels called -- query_fragment: {:#?}, rel_var: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, partition_key_opt);

        let query = "g.E()".to_string()
            + query_fragment.where_fragment()
            + ".project('src_id', 'rel', 'dst_id').by(outV().id()).by(valueMap(true)).by(inV().id())";

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
        let results: Vec<GValue> = raw_results.try_collect().await?;

        results
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        level = "info",
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
            false,
            self.long_ids,
            sg,
        )?;

        q.push_str(".valueMap(true)");

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
        let results: Vec<GValue> = raw_results.try_collect().await?;

        results
            .into_iter()
            .map(|n| (n, info).try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        level = "info",
        name = "wg-gremlin-update-rels",
        skip(self, query_fragment, rel_var, props, partition_key_opt, sg)
    )]
    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("GremlinTransaction::update_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, props, partition_key_opt);

        let first = "g.E()".to_string() + query_fragment.where_fragment();
        let (mut q, p) = GremlinTransaction::add_properties(
            first,
            props,
            query_fragment.params(),
            false,
            false,
            self.long_ids,
            sg,
        )?;

        q.push_str(
            ".project('src_id', 'rel', 'dst_id').by(outV().id()).by(valueMap(true)).by(inV().id())",
        );

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
        let results: Vec<GValue> = raw_results.try_collect().await?;

        results
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        level = "info",
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
        let mut results: Vec<GValue> = raw_results.try_collect().await?;

        Ok(
            TryInto::<i64>::try_into(results.pop().ok_or(Error::ResponseSetNotFound)?)?
                .try_into()?,
        )
    }

    #[tracing::instrument(
        level = "info",
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
        let mut results: Vec<GValue> = raw_results.try_collect().await?;

        Ok(
            TryInto::<i64>::try_into(results.pop().ok_or(Error::ResponseSetNotFound)?)?
                .try_into()?,
        )
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

impl<RequestCtx: RequestContext> TryFrom<(GValue, &Info)> for Node<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: (GValue, &Info)) -> Result<Self, Error> {
        if let GValue::Map(map) = value.0.clone() {
            let label = TryInto::<String>::try_into(
                map.get("label")
                    .ok_or(Error::ResponseItemNotFound {
                        name: "label".to_string(),
                    })?
                    .clone(),
            )?;
            let properties = map
                .into_iter()
                .filter(|(k, _v)| k != &GKey::String("label".to_string()))
                .map(|(k, v)| {
                    Ok((
                        k.clone().try_into()?,
                        (
                            v,
                            TryInto::<String>::try_into(k.clone())?.as_str(),
                            value
                                .1
                                .type_def_by_name(&label)?
                                .property(TryInto::<String>::try_into(k)?.as_str())?,
                        )
                            .try_into()?,
                    ))
                })
                .collect::<Result<HashMap<String, Value>, Error>>()?;
            Ok(Node::new(label, properties))
        } else if let GValue::Vertex(vertex) = value.0.clone() {
            let id = vertex.id().to_gvalue().try_into()?;
            let type_name = vertex.label().clone();
            let mut properties: HashMap<String, Value> = vertex
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        (
                            v,
                            k.as_str(),
                            value.1.type_def_by_name(&type_name)?.property(&k)?,
                        )
                            .try_into()?,
                    ))
                })
                .collect::<Result<HashMap<String, Value>, Error>>()?;
            properties.insert("id".to_string(), id);
            Ok(Node::new(type_name, properties))
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Node".to_string(),
            })
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<GValue> for Rel<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: GValue) -> Result<Self, Error> {
        if let GValue::Map(map) = value.clone() {
            let src_ref = (map
                .get("src_id")
                .ok_or(Error::ResponseItemNotFound {
                    name: "src".to_string(),
                })?
                .clone())
            .try_into()?;
            let dst_ref = (map
                .get("dst_id")
                .ok_or(Error::ResponseItemNotFound {
                    name: "dst".to_string(),
                })?
                .clone())
            .try_into()?;

            if let GValue::Map(rel_map) = map
                .get("rel")
                .ok_or(Error::ResponseItemNotFound {
                    name: "rel".to_string(),
                })?
                .clone()
            {
                let label = TryInto::<String>::try_into(
                    rel_map
                        .get("label")
                        .ok_or(Error::ResponseItemNotFound {
                            name: "label".to_string(),
                        })?
                        .clone(),
                )?;
                let properties = rel_map
                    .into_iter()
                    .filter(|(k, _v)| k != &GKey::String("label".to_string()))
                    .map(|(k, v)| Ok((k.try_into()?, v.try_into()?)))
                    .collect::<Result<HashMap<String, Value>, Error>>()?;
                Ok(Rel::new(label, properties, src_ref, dst_ref))
            } else {
                Err(Error::TypeConversionFailed {
                    src: format!("{:#?}", value),
                    dst: "Map".to_string(),
                })
            }
        } else if let GValue::Edge(edge) = value {
            let id = edge.id().to_gvalue().try_into()?;
            let rel_name = edge.label().clone();
            let src_ref = NodeRef::Identifier(edge.out_v().id().to_gvalue().try_into()?);
            let dst_ref = NodeRef::Identifier(edge.in_v().id().to_gvalue().try_into()?);
            let mut properties: HashMap<String, Value> = edge
                .into_iter()
                .map(|(k, v)| Ok((k, v.value().clone().try_into()?)))
                .collect::<Result<HashMap<String, Value>, Error>>()?;
            properties.insert("id".to_string(), id);
            Ok(Rel::new(rel_name, properties, src_ref, dst_ref))
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Rel".to_string(),
            })
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<GValue> for NodeRef<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: GValue) -> Result<Self, Error> {
        if let GValue::Vertex(v) = value {
            Ok(NodeRef::Identifier(v.id().to_gvalue().try_into()?))
        } else if let GValue::Int32(i) = value {
            Ok(NodeRef::Identifier(Value::String(i.to_string())))
        } else if let GValue::Int64(i) = value {
            Ok(NodeRef::Identifier(Value::String(i.to_string())))
        } else if let GValue::String(s) = value {
            Ok(NodeRef::Identifier(Value::String(s)))
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "NodeRef::Identifier".to_string(),
            })
        }
    }
}

impl TryFrom<(Vec<VertexProperty>, &str, &Property)> for Value {
    type Error = crate::Error;

    fn try_from(value: (Vec<VertexProperty>, &str, &Property)) -> Result<Self, Error> {
        if value.1 == "partitionKey" || !value.2.list() {
            value
                .0
                .into_iter()
                .next()
                .ok_or(Error::ResponseItemNotFound {
                    name: value.1.to_string(),
                })?
                .value()
                .clone()
                .try_into()
        } else {
            Ok(Value::Array(
                value
                    .0
                    .into_iter()
                    .map(|val| val.value().clone().try_into())
                    .collect::<Result<Vec<Value>, Error>>()?,
            ))
        }
    }
}

impl TryFrom<(GValue, &str, &Property)> for Value {
    type Error = crate::Error;

    fn try_from(value: (GValue, &str, &Property)) -> Result<Self, Error> {
        if value.1 == "id" {
            Ok(Value::String(
                TryInto::<Value>::try_into(value.0)?.to_string(),
            ))
        } else if let GValue::List(list) = value.0 {
            if value.1 == "partitionKey" || !value.2.list() {
                list.into_iter()
                    .next()
                    .ok_or(Error::ResponseItemNotFound {
                        name: value.1.to_string(),
                    })?
                    .try_into()
            } else {
                Ok(Value::Array(
                    list.into_iter()
                        .map(|val| val.try_into())
                        .collect::<Result<Vec<Value>, Error>>()?,
                ))
            }
        } else {
            value.0.try_into()
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
    fn test_add_properties_array() {
        let s1 = Value::String("String one".to_string());
        let s2 = Value::String("String two".to_string());
        let a = Value::Array(vec![s1, s2]);

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => a},
            HashMap::new(),
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
    fn test_add_properties_scalar() {
        let s1 = Value::String("String one".to_string());

        let (q, p) = GremlinTransaction::add_properties(
            String::new(),
            hashmap! {"my_prop".to_string() => s1},
            HashMap::new(),
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
}
