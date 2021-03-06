//! Traits and helper structs for interacting with the graph storage database

#[cfg(any(feature = "cosmos", feature = "gremlin"))]
pub mod gremlin;
#[cfg(feature = "neo4j")]
pub mod neo4j;
pub mod no_database;

use crate::engine::context::RequestContext;
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use async_trait::async_trait;
#[cfg(any(feature = "cosmos", feature = "gremlin"))]
use gremlin_client::GremlinClient;
#[cfg(feature = "neo4j")]
use mobc::Connection;
#[cfg(feature = "neo4j")]
use mobc_boltrs::BoltConnectionManager;
use std::collections::HashMap;
use std::convert::TryFrom;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use std::env::var_os;
use std::fmt::Debug;

#[cfg(feature = "gremlin")]
pub fn env_bool(var_name: &str) -> Result<bool, Error> {
    Ok(env_string(var_name)?.parse::<bool>()?)
}

#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
fn env_string(var_name: &str) -> Result<String, Error> {
    var_os(var_name)
        .map(|osstr| osstr.to_string_lossy().into_owned())
        .ok_or_else(|| Error::EnvironmentVariableNotFound {
            name: var_name.to_string(),
        })
}

#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
fn env_u16(var_name: &str) -> Result<u16, Error> {
    Ok(env_string(var_name)?.parse::<u16>()?)
}

/// Contains a database client
pub enum DatabaseClient {
    /// Cosmos database client
    #[cfg(any(feature = "cosmos", feature = "gremlin"))]
    Gremlin(Box<GremlinClient>),

    /// Neo4J database client
    #[cfg(feature = "neo4j")]
    Neo4j(Box<Connection<BoltConnectionManager>>),

    /// No database has been configured for use
    NoDatabase,
}

/// Trait for a database endpoint. Structs that implement this trait typically take in a connection
/// string and produce a database pool of clients connected to the database
#[async_trait]
pub trait DatabaseEndpoint {
    type PoolType: DatabasePool;

    /// Returns a [`DatabasePool`] to the database for which this DatabaseEndpoint has connection
    /// information
    ///
    /// [`DatabasePool`]: ./trait.DatabasePool.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the database pool cannot be built, for example if the database
    /// connection information in the implementation of the DatabaseEndpoint does not successfully
    /// connect to a database. The specific [`Error`] variant depends on the database back-end.
    ///
    /// [`Error`]: ../../enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let endpoint = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let pool = runtime.block_on(endpoint.pool())?;
    /// # Ok(())
    /// # }
    /// ```
    async fn pool(&self) -> Result<Self::PoolType, Error>;
}

/// Trait for a database pool. Structs that implement this trait are created by a database endpoint
/// and provide a way to get a transaction from the pool.
#[async_trait]
pub trait DatabasePool: Clone + Sync + Send {
    type TransactionType: Transaction;

    /// Returns a [`Transaction`] for the database for which this DatabasePool has connections
    ///
    /// [`Transaction`]: ./trait.DatabasePool.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the transaction cannot be created. The specific [`Error`] variant
    /// depends on the database back-end.
    ///
    /// [`Error`]: ../../enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use tokio::main;
    /// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let endpoint = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let pool = endpoint.pool().await?;
    /// # #[cfg(feature = "neo4j")]
    /// let transaction = pool.transaction().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn transaction(&self) -> Result<Self::TransactionType, Error>;

    /// Returns a [`DatabaseClient`] for the database for which this DatabasePool has connections
    ///
    /// [`DatabaseClient`]: ./enum.DatabaseClient.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the client cannot be obtained from the pool. The specific [`Error`]
    /// variant depends on the database back-end.
    ///
    /// [`Error`]: ../../enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use tokio::main;
    /// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let endpoint = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let pool = endpoint.pool().await?;
    /// # #[cfg(feature = "neo4j")]
    /// let client = pool.client().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn client(&self) -> Result<DatabaseClient, Error>;
}

#[async_trait]
pub trait Transaction: Send + Sync {
    async fn begin(&mut self) -> Result<(), Error>;

    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<RequestCtx>, Error>;

    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_query_fragment: QueryFragment,
        dst_query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    fn node_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        nodes: &[Node<RequestCtx>],
    ) -> Result<QueryFragment, Error>;

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<QueryFragment>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error>;

    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    fn rel_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        rel_var: &RelQueryVar,
        rels: &[Rel<RequestCtx>],
    ) -> Result<QueryFragment, Error>;

    fn rel_read_fragment(
        &mut self,
        src_fragment_opt: Option<QueryFragment>,
        dst_fragment_opt: Option<QueryFragment>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error>;

    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    async fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    async fn commit(&mut self) -> Result<(), Error>;

    async fn rollback(&mut self) -> Result<(), Error>;
}

/// Represents the different types of Crud Operations along with the target of the
/// operation (node typename and rel typename for rel ops). This enum is passed to
/// event handler functions.
pub enum CrudOperation {
    ReadNode(String),
    ReadRel(String, String),
    CreateNode(String),
    CreateRel(String, String),
    UpdateNode(String),
    UpdateRel(String, String),
    DeleteNode(String),
    DeleteRel(String, String),
}

/// Represents the different type of comparison match operations
#[derive(Clone, Debug)]
pub enum Operation {
    EQ,
    CONTAINS,
    IN,
    GT,
    GTE,
    LT,
    LTE,
}

/// Struct representing a value comparison. In query operations, visitors take provided
/// operation/value nested map and converted them into a `Comparison` struct and pass
/// it on the database-specific transaction for use in creating match portion of queries.
#[derive(Clone, Debug)]
pub struct Comparison {
    operation: Operation,
    operand: Value,
    negated: bool,
}

impl Comparison {
    pub fn new(operation: Operation, negated: bool, operand: Value) -> Self {
        Comparison {
            operation,
            operand,
            negated,
        }
    }

    pub fn default(v: Value) -> Self {
        Self::new(Operation::EQ, false, v)
    }
}

impl TryFrom<Value> for Comparison {
    type Error = Error;

    fn try_from(v: Value) -> Result<Comparison, Error> {
        Ok(match v {
            Value::String(_) => Comparison::default(v),
            Value::Int64(_) => Comparison::default(v),
            Value::Float64(_) => Comparison::default(v),
            Value::Bool(_) => Comparison::default(v),
            Value::Map(m) => {
                let (operation_str, operand) =
                    m.into_iter().next().ok_or(Error::InputItemNotFound {
                        name: "Comparison keys".to_string(),
                    })?;
                Comparison::new(
                    match operation_str.as_ref() {
                        "EQ" => Operation::EQ,
                        "NOTEQ" => Operation::EQ,
                        "CONTAINS" => Operation::CONTAINS,
                        "NOTCONTAINS" => Operation::CONTAINS,
                        "IN" => Operation::IN,
                        "NOTIN" => Operation::IN,
                        "GT" => Operation::GT,
                        "GTE" => Operation::GTE,
                        "LT" => Operation::LT,
                        "LTE" => Operation::LTE,
                        _ => {
                            return Err(Error::TypeNotExpected {
                                details: Some(format!("comparison operation {}", operation_str)),
                            })
                        }
                    },
                    matches!(operation_str.as_ref(), "NOTEQ" | "NOTCONTAINS" | "NOTIN"),
                    operand,
                )
            }
            _ => {
                return Err(Error::TypeNotExpected {
                    details: Some(format!("comparison value: {:#?}", v)),
                })
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct QueryFragment {
    match_fragment: String,
    where_fragment: String,
    params: HashMap<String, Value>,
}

impl QueryFragment {
    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn new(
        match_fragment: String,
        where_fragment: String,
        params: HashMap<String, Value>,
    ) -> QueryFragment {
        QueryFragment {
            match_fragment,
            where_fragment,
            params,
        }
    }

    #[cfg(feature = "neo4j")]
    pub(crate) fn match_fragment(&self) -> &str {
        &self.match_fragment
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn where_fragment(&self) -> &str {
        &self.where_fragment
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn params(self) -> HashMap<String, Value> {
        self.params
    }
}

#[derive(Clone, Debug)]
pub struct NodeQueryVar {
    base: String,
    suffix: String,
    label: Option<String>,
    name: String,
}

impl NodeQueryVar {
    pub(crate) fn new(label: Option<String>, base: String, suffix: String) -> NodeQueryVar {
        NodeQueryVar {
            base: base.clone(),
            suffix: suffix.clone(),
            label,
            name: base + &suffix,
        }
    }

    pub(crate) fn base(&self) -> &str {
        &self.base
    }

    pub(crate) fn label(&self) -> Result<&str, Error> {
        self.label.as_deref().ok_or(Error::LabelNotFound)
    }

    pub(crate) fn suffix(&self) -> &str {
        &self.suffix
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub struct RelQueryVar {
    label: String,
    suffix: String,
    name: String,
    src: NodeQueryVar,
    dst: NodeQueryVar,
}

impl RelQueryVar {
    pub(crate) fn new(
        label: String,
        suffix: String,
        src: NodeQueryVar,
        dst: NodeQueryVar,
    ) -> RelQueryVar {
        RelQueryVar {
            label,
            suffix: suffix.clone(),
            name: "rel".to_string() + &suffix,
            src,
            dst,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    #[cfg(any(feature = "neo4j"))]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn src(&self) -> &NodeQueryVar {
        &self.src
    }

    pub(crate) fn dst(&self) -> &NodeQueryVar {
        &self.dst
    }
}

#[derive(Debug, Default)]
pub struct SuffixGenerator {
    seed: i32,
}

impl SuffixGenerator {
    pub(crate) fn new() -> SuffixGenerator {
        SuffixGenerator { seed: -1 }
    }

    pub(crate) fn suffix(&mut self) -> String {
        self.seed += 1;
        "_".to_string() + &self.seed.to_string()
    }
}
