//! Provides database interface types and functions for Cosmos DB and other Gremlin-based DBs

use crate::engine::context::RequestContext;
#[cfg(feature = "gremlin")]
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
#[cfg(feature = "gremlin")]
use gremlin_client::TlsOptions;
use gremlin_client::{ConnectionOptions, GKey, GValue, GraphSON, Map, ToGValue, VertexProperty};
use juniper::futures::TryStreamExt;
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
    pool_size: u16,
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
    /// * WG_POOL_SIZE - connection pool size
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
            pool_size: env_u16("WG_POOL_SIZE")
                .unwrap_or_else(|_| num_cpus::get().try_into().unwrap_or(8)),
        })
    }
}

#[cfg(feature = "cosmos")]
#[async_trait]
impl DatabaseEndpoint for CosmosEndpoint {
    type PoolType = CosmosPool;

    async fn pool(&self) -> Result<Self::PoolType, Error> {
        Ok(CosmosPool::new(
            GremlinClient::connect(
                ConnectionOptions::builder()
                    .host(&self.host)
                    .port(self.port)
                    .pool_size(self.pool_size.into())
                    .ssl(true)
                    .serializer(GraphSON::V1)
                    .deserializer(GraphSON::V1)
                    .credentials(&self.user, &self.pass)
                    .build(),
            )
            .await?,
        ))
    }
}

#[cfg(feature = "cosmos")]
#[derive(Clone)]
pub struct CosmosPool {
    pool: GremlinClient,
}

#[cfg(feature = "cosmos")]
impl CosmosPool {
    fn new(pool: GremlinClient) -> Self {
        CosmosPool { pool }
    }
}

#[cfg(feature = "cosmos")]
#[async_trait]
impl DatabasePool for CosmosPool {
    type TransactionType = GremlinTransaction;

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(self.pool.clone(), true, true))
    }

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(self.pool.clone(), true, true))
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
    use_tls: bool,
    pool_size: u16,
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
    /// * WG_POOL_SIZE - connection pool size
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
            use_tls: env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true),
            pool_size: env_u16("WG_POOL_SIZE")
                .unwrap_or_else(|_| num_cpus::get().try_into().unwrap_or(8)),
        })
    }
}

#[cfg(feature = "gremlin")]
#[async_trait]
impl DatabaseEndpoint for GremlinEndpoint {
    type PoolType = GremlinPool;
    async fn pool(&self) -> Result<Self::PoolType, Error> {
        let mut options_builder = ConnectionOptions::builder()
            .host(&self.host)
            .port(self.port)
            .pool_size(self.pool_size.into())
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
        Ok(GremlinPool::new(GremlinClient::connect(options).await?))
    }
}

#[cfg(feature = "gremlin")]
#[derive(Clone)]
pub struct GremlinPool {
    pool: GremlinClient,
}

#[cfg(feature = "gremlin")]
impl GremlinPool {
    fn new(pool: GremlinClient) -> Self {
        GremlinPool { pool }
    }
}

#[cfg(feature = "gremlin")]
#[async_trait]
impl DatabasePool for GremlinPool {
    type TransactionType = GremlinTransaction;

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(self.pool.clone(), false, true))
    }

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(self.pool.clone(), false, true))
    }
}

/// A Neptune DB endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::gremlin::NeptuneEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ne = NeptuneEndpoint::from_env()?;
/// #    Ok(())
/// # }
/// ```
#[cfg(feature = "gremlin")]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NeptuneEndpoint {
    host: String,
    port: u16,
    user: Option<String>,
    pass: Option<String>,
    accept_invalid_certs: bool,
    use_tls: bool,
    read_host: String,
    pool_size: u16,
}

#[cfg(feature = "gremlin")]
impl NeptuneEndpoint {
    /// Reads a set of environment variables to construct a [`NeptuneEndpoint`]. The environment
    /// variables are as follows
    ///
    /// * WG_GREMLIN_HOST - the hostname for the Gremlin-based DB. For example, `localhost`.
    /// * WG_GREMLIN_PORT - the port number for the Gremlin-based DB. For example, `443`.
    /// * WG_GREMLIN_USE_TLS - true if Warpgrapher should use TLS to connect to gremlin endpoint.
    /// * WG_GREMLIN_CERT - true if Warpgrapher should accept an invalid cert. This could be
    /// necessary in a test environment, but it should be set to false in production environments.
    /// * WG_NEPTUNE_READ_REPLICAS - hostname for the Neptune read replicas
    /// * WG_POOL_SIZE - connection pool size
    ///
    /// The accept_invalid_certs option may be set to true in a test environment, where a test
    /// Gremlin server is running with an invalid cert. It should be set to false in production
    /// environments.
    ///
    /// [`NeptuneEndpoint`]: ./struct.NeptuneEndpoint.html
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
    /// # use warpgrapher::engine::database::gremlin::NeptuneEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ge = NeptuneEndpoint::from_env()?;
    /// #    Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<NeptuneEndpoint, Error> {
        Ok(NeptuneEndpoint {
            host: env_string("WG_GREMLIN_HOST")?,
            port: env_u16("WG_GREMLIN_PORT")?,
            user: var_os("WG_GREMLIN_USER").map(|osstr| osstr.to_string_lossy().into_owned()),
            pass: var_os("WG_GREMLIN_PASS").map(|osstr| osstr.to_string_lossy().into_owned()),
            accept_invalid_certs: env_bool("WG_GREMLIN_CERT")?,
            use_tls: env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true),
            read_host: env_string("WG_NEPTUNE_READ_REPLICAS")?,
            pool_size: env_u16("WG_POOL_SIZE")
                .unwrap_or_else(|_| num_cpus::get().try_into().unwrap_or(8)),
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn read_host(&self) -> &str {
        &self.read_host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn tls(&self) -> bool {
        self.use_tls
    }
}

#[cfg(feature = "gremlin")]
#[async_trait]
impl DatabaseEndpoint for NeptuneEndpoint {
    type PoolType = NeptunePool;
    async fn pool(&self) -> Result<Self::PoolType, Error> {
        let mut write_options_builder = ConnectionOptions::builder()
            .host(&self.host)
            .port(self.port)
            .pool_size(self.pool_size.into())
            .serializer(GraphSON::V3)
            .deserializer(GraphSON::V3);
        if let (Some(user), Some(pass)) = (self.user.as_ref(), self.pass.as_ref()) {
            write_options_builder = write_options_builder.credentials(user, pass);
        }
        if self.use_tls {
            write_options_builder = write_options_builder.ssl(true).tls_options(TlsOptions {
                accept_invalid_certs: self.accept_invalid_certs,
            });
        }
        let write_options = write_options_builder.build();

        let mut ro_options_builder = ConnectionOptions::builder()
            .host(&self.read_host)
            .port(self.port)
            .pool_size(self.pool_size.into())
            .serializer(GraphSON::V3)
            .deserializer(GraphSON::V3);
        if let (Some(user), Some(pass)) = (self.user.as_ref(), self.pass.as_ref()) {
            ro_options_builder = ro_options_builder.credentials(user, pass);
        }
        if self.use_tls {
            ro_options_builder = ro_options_builder.ssl(true).tls_options(TlsOptions {
                accept_invalid_certs: self.accept_invalid_certs,
            });
        }
        let ro_options = ro_options_builder.build();

        #[allow(clippy::eval_order_dependence)]
        Ok(NeptunePool::new(
            GremlinClient::connect(write_options).await?,
            GremlinClient::connect(ro_options).await?,
        ))
    }
}

#[cfg(feature = "gremlin")]
#[derive(Clone)]
pub struct NeptunePool {
    read_pool: GremlinClient,
    write_pool: GremlinClient,
}

#[cfg(feature = "gremlin")]
impl NeptunePool {
    fn new(read_pool: GremlinClient, write_pool: GremlinClient) -> Self {
        NeptunePool {
            read_pool,
            write_pool,
        }
    }
}

#[cfg(feature = "gremlin")]
#[async_trait]
impl DatabasePool for NeptunePool {
    type TransactionType = GremlinTransaction;
    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(
            self.write_pool
                .clone()
                .create_session(Uuid::new_v4().to_hyphenated().to_string())
                .await?,
            false,
            false,
        ))
    }

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(GremlinTransaction::new(
            self.read_pool.clone(),
            false,
            false,
        ))
    }
}

pub struct GremlinTransaction {
    client: GremlinClient,
    partition: bool,
    use_bindings: bool,
}

impl GremlinTransaction {
    pub fn new(client: GremlinClient, partition: bool, use_bindings: bool) -> Self {
        GremlinTransaction {
            client,
            partition,
            use_bindings,
        }
    }

    fn add_properties(
        mut query: String,
        mut props: HashMap<String, Value>,
        mut params: HashMap<String, Value>,
        note_singles: bool,
        use_bindings: bool,
        create_query: bool,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
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
                            Ok(if use_bindings {
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
                                (inner_q, inner_p)
                            } else {
                                inner_q.push_str(
                                    // Use
                                    &*(".property(set, '".to_string()
                                        + &*k
                                        + "', "
                                        + &*val.to_property_value()?
                                        + ")"),
                                );
                                (inner_q, inner_p)
                            })
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
            Err(Error::TypeNotExpected { details: None })
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
                    Err(Error::TypeNotExpected { details: None })
                }
            })
            .collect::<Result<HashMap<String, Value>, Error>>()
    }

    fn gmap_to_hashmap(gv: GValue) -> Result<HashMap<String, GValue>, Error> {
        if let GValue::Map(map) = gv {
            map.into_iter()
                .map(|(k, v)| match (k, v) {
                    (GKey::String(s), v) => Ok((s, v)),
                    (_, _) => Err(Error::TypeNotExpected { details: None }),
                })
                .collect()
        } else {
            Err(Error::TypeNotExpected { details: None })
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

                let rel_id = match hm.remove("rID") {
                    Some(GValue::String(s)) => Value::String(s),
                    Some(GValue::Int64(i)) => Value::Int64(i),
                    Some(GValue::Uuid(uuid)) => Value::Uuid(uuid),
                    _ => {
                        return Err(Error::ResponseItemNotFound {
                            name: "Rel, src, or dst".to_string(),
                        })
                    }
                };

                if let (
                    Some(GValue::Map(rel_props)),
                    Some(GValue::String(src_id)),
                    Some(GValue::String(src_label)),
                    Some(GValue::String(dst_id)),
                    Some(GValue::String(dst_label)),
                ) = (
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
                                Err(Error::TypeNotExpected { details: None })
                            }
                        })
                        .collect::<Result<HashMap<String, Value>, Error>>()?;

                    Ok(Rel::new(
                        rel_id,
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

#[async_trait]
impl Transaction for GremlinTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        Ok(())
    }

    #[tracing::instrument(name = "wg-gremlin-execute-query", skip(self, query, params))]
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
        name = "wg-gremlin-create-nodes",
        skip(self, node_var, props, partition_key_opt, info, sg)
    )]
    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        mut props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error> {
        trace!("GremlinTransaction::create_node called -- node_var: {:#?}, props: {:#?}, partition_key_opt: {:#?}", node_var, props, partition_key_opt);

        let mut query = "g.addV('".to_string() + node_var.label()? + "')";

        if self.partition {
            query.push_str(".property('partitionKey', partitionKey)");
        }

        if !props.contains_key("id") {
            props.insert(
                "id".to_string(),
                Value::String(Uuid::new_v4().to_hyphenated().to_string()),
            );
        }

        let (mut q, p) = GremlinTransaction::add_properties(
            query,
            props,
            HashMap::new(),
            true,
            self.use_bindings,
            true,
            sg,
        )?;
        q += NODE_RETURN_FRAGMENT;

        trace!("GremlinTransaction::create_node -- q: {}, p: {:#?}", q, p);

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

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;
        trace!("GremlinTransaction::create_node -- results: {:#?}", results);

        GremlinTransaction::nodes(results, info)?
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)
    }

    #[tracing::instrument(
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
            self.use_bindings,
            true,
            sg,
        )?;

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

        if self.partition {
            query.push_str(".has('partitionKey', partitionKey)");
        }
        query.push_str(".hasId(within(id_list))");
        let ids = nodes
            .iter()
            .map(|n| n.id())
            .collect::<Result<Vec<&Value>, Error>>()?
            .into_iter()
            .cloned()
            .collect();
        let mut params = HashMap::new();
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

        if self.partition {
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
                + &*(if self.use_bindings { k.clone() + &*param_suffix } else { c.operand.to_property_value()? })
                + "))"),
            );

            if self.use_bindings {
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

        if self.partition {
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

        if self.partition {
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
                + &*(if self.use_bindings {k.clone() + &*param_suffix} else {c.operand.to_property_value()?})
                + ")"
                + ")"),
            );
            if self.use_bindings {
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

        if self.partition {
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
            self.use_bindings,
            false,
            sg,
        )?;
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

        let raw_results = self.client.execute(q, param_list.as_slice()).await?;
        let results = raw_results.try_collect().await?;

        GremlinTransaction::nodes(results, info)
    }

    #[tracing::instrument(
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
            self.use_bindings,
            false,
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

        if self.partition {
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

        if self.partition {
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

        if self.partition {
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
        self.client.close_session().await.map_err(Error::from)
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
    #[cfg(feature = "cosmos")]
    use super::CosmosEndpoint;
    #[cfg(feature = "gremlin")]
    use super::GremlinEndpoint;
    use super::GremlinTransaction;
    use crate::engine::database::SuffixGenerator;
    use crate::Value;
    use maplit::hashmap;
    use std::collections::HashMap;
    use uuid::Uuid;

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
            &mut SuffixGenerator::new(),
        )
        .unwrap();

        assert_eq!(".property(single, 'my_prop', -1)", q);
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
