#[cfg(feature = "graphson2")]
pub mod graphson2;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::error::Error;
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use crate::ErrorKind;
use juniper::FieldError;
#[cfg(feature = "neo4j")]
use r2d2::Pool;
#[cfg(feature = "neo4j")]
use r2d2_cypher::CypherConnectionManager;
use serde::Serialize;
use std::collections::HashMap;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use std::env::var_os;
use std::fmt::Debug;

/*
#[cfg(feature = "graphson2")]
use gremlin_client::GremlinClient;
use r2d2::Pool;
use r2d2_cypher::CypherConnectionManager;
*/

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
fn get_env_var(var_name: &str) -> Result<String, Error> {
    match var_os(var_name) {
        None => Err(Error::new(
            ErrorKind::EnvironmentVariableNotFound(var_name.to_string()),
            None,
        )),
        Some(os) => match os.to_str() {
            None => Err(Error::new(
                ErrorKind::EnvironmentVariableNotFound(var_name.to_string()),
                None,
            )),
            Some(osstr) => Ok(osstr.to_owned()),
        },
    }
}

/*
pub trait DatabaseClient {
    type ImplTransaction: Transaction;
    fn get_transaction(&self) -> Result<Self::ImplTransaction, FieldError>;
}
*/

#[derive(Clone, Debug)]
pub enum DatabasePool {
    #[cfg(feature = "neo4j")]
    Neo4j(Pool<CypherConnectionManager>),
    #[cfg(feature = "graphson2")]
    Graphson2,
    // Used to serve the schema without a database backend
    NoDatabase,
}

pub trait DatabaseEndpoint {
    fn get_pool(&self) -> Result<DatabasePool, Error>;
}

pub trait Transaction {
    type ImplQueryResult: QueryResult + Debug;
    fn begin(&self) -> Result<(), FieldError>;
    fn commit(&mut self) -> Result<(), FieldError>;
    fn exec<V>(
        &mut self,
        query: &str,
        params: Option<&HashMap<String, V>>,
    ) -> Result<Self::ImplQueryResult, FieldError>
    where
        V: Debug + Serialize;
    fn rollback(&mut self) -> Result<(), FieldError>;
}

pub trait QueryResult: Debug {
    fn get_nodes<GlobalCtx, ReqCtx>(
        &self,
        name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug;
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
        ReqCtx: WarpgrapherRequestContext + Debug;
    fn get_ids(&self, column_name: &str) -> Result<Vec<String>, FieldError>;
    fn get_count(&self) -> Result<i32, FieldError>;
    fn len(&self) -> i32;
    fn is_empty(&self) -> bool;
}

// impl DatabasePool for () {
/*
type ImplClient = ();

fn get_client(&self) -> Result<(), FieldError> {
    Ok(())
}
*/
// }

/*
impl DatabaseClient for () {
    type ImplTransaction = ();

    fn get_transaction(&self) -> Result<(), FieldError> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ).into())
    }
}
*/

/*
impl Transaction for () {
    type ImplQueryResult = ();

    fn begin(&self) -> Result<(), Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }
    fn commit(&self) -> Result<(), Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }
    fn exec<K, V>(&self, _query: &str, _params: Option<&HashMap<K, V>>) -> Result<(), Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }
    fn empty_result(&self) -> Result<(), Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }
    fn rollback(&self) -> Result<(), Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }
}

impl QueryResult for () {
    fn get_nodes<GlobalCtx, ReqCtx>(
        &self,
        _type_name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, Error>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }

    fn get_rels<GlobalCtx, ReqCtx>(
        &self,
        _type_name: &str,
    ) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, Error>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }

    fn get_ids(&self, _type_name: &str) -> Result<Vec<String>, FieldError> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        )
        .into())
    }

    fn get_count(&self) -> Result<Value, Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }

    fn merge(&mut self, _r: impl QueryResult) -> Result<(), Error> {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("No database chosen.".to_owned()),
            None,
        ))
    }

    fn len(&self) -> i32 {
        0
    }
}
*/

/*
#[derive(Clone)]
pub enum DatabaseEndpoint {
    #[cfg(feature = "graphson2")]
    Graphson2 { db_url: String },

    #[cfg(feature = "neo4j")]
    Neo4j { db_url: String },
}

impl DatabaseEndpoint {
    pub fn from_env_vars() -> Result<DatabaseEndpoint, Error> {
        match get_env_var("WG_DB_TYPE")?.as_str() {
            #[cfg(feature = "graphson2")]
            "graphson2" => Ok(DatabaseEndpoint::Graphson2 {
                db_url: get_env_var("WG_NEO4J_URL")?,
            }),
            #[cfg(feature = "neo4j")]
            "neo4j" => Ok(DatabaseEndpoint::Neo4j {
                db_url: get_env_var("WG_GRAPHSON2_URL")?,
            }),
            db_type => Err(Error::new(
                ErrorKind::UnsupportedDatabase(db_type.to_owned()),
                None,
            )),
        }
    }

    pub fn get_pool(&self) -> Result<DatabasePool, Error> {
        match self {
            #[cfg(feature = "graphson2")]
            DatabaseEndpoint::Graphson2 { db_url } => DatabasePool::new(self),

            #[cfg(feature = "neo4j")]
            DatabaseEndpoint::Neo4j { db_url } => DatabasePool::new(self),
        }
    }
}

#[derive(Clone)]
pub enum DatabasePool {
    // The gremlin client does pooling within the client itself, so the DatabasePool
    // in Warpgrapher is just a pass through to the gremlin client. Contrast with
    // rusted_cypher, for which we're responsible for client pooling.
    #[cfg(feature = "graphson2")]
    Graphson2 { client: GremlinClient },

    #[cfg(feature = "neo4j")]
    Neo4j { pool: Pool<CypherConnectionManager> },
}

impl DatabasePool {
    pub fn new(database_endpoint: &DatabaseEndpoint) -> Result<DatabasePool, Error> {
        match database_endpoint {
            #[cfg(feature = "graphson2")]
            DatabaseEndpoint::Graphson2 { db_url } => {
                let manager = CypherConnectionManager {
                    url: db_url.to_owned(),
                };
                Ok(DatabasePool::Neo4j {
                    pool: Pool::builder()
                        .max_size(5)
                        .build(manager)
                        .map_err(|e| Error::new(ErrorKind::CouldNotBuildDatabasePool(e), None))?,
                })
            }

            #[cfg(feature = "neo4j")]
            DatabaseEndpoint::Neo4j { db_url } => {
                let manager = CypherConnectionManager {
                    url: db_url.to_owned(),
                };
                Ok(DatabasePool::Neo4j {
                    pool: Pool::builder()
                        .max_size(5)
                        .build(manager)
                        .map_err(|e| Error::new(ErrorKind::CouldNotBuildDatabasePool(e), None))?,
                })
            }
        }
    }*/

/*
pub fn client(&self) -> Result<impl Client, Error> {
    match self {
        #[cfg(feature = "graphson2")]
        DatabasePool::Graphson2 { client } => Ok(Graphson2Client {
            client: client.clone(),
        }),

        #[cfg(feature = "neo4j")]
        DatabasePool::Neo4j { pool } => Ok(Neo4jClient {
            client: pool.get()?,
        }),
    }
}
*/
// }
