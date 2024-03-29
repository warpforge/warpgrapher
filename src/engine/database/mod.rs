//! Traits and helper structs for interacting with the graph storage database

#[cfg(feature = "cypher")]
pub mod cypher;
#[cfg(feature = "gremlin")]
pub mod gremlin;
pub mod no_database;

use crate::engine::context::RequestContext;
use crate::engine::loader::{NodeLoaderKey, RelLoaderKey};
use crate::engine::objects::{Node, Options, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use async_trait::async_trait;
#[cfg(feature = "cypher")]
use bolt_proto::message::Record;
#[cfg(feature = "gremlin")]
use gremlin_client::GValue;
use std::collections::HashMap;
use std::convert::TryFrom;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
use std::env::var_os;
use std::fmt::Debug;

#[cfg(feature = "gremlin")]
pub fn env_bool(var_name: &str) -> Result<bool, Error> {
    Ok(env_string(var_name)?.parse::<bool>()?)
}

#[cfg(any(feature = "gremlin", feature = "cypher"))]
fn env_string(var_name: &str) -> Result<String, Error> {
    var_os(var_name)
        .map(|osstr| osstr.to_string_lossy().into_owned())
        .ok_or_else(|| Error::EnvironmentVariableNotFound {
            name: var_name.to_string(),
        })
}

#[cfg(any(feature = "gremlin", feature = "cypher"))]
fn env_u16(var_name: &str) -> Result<u16, Error> {
    Ok(env_string(var_name)?.parse::<u16>()?)
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
    /// # #[cfg(feature = "cypher")]
    /// # use tokio::runtime::Runtime;
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "cypher")]
    /// # use warpgrapher::engine::database::cypher::CypherEndpoint;
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "cypher")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "cypher")]
    /// let endpoint = CypherEndpoint::from_env()?;
    /// # #[cfg(feature = "cypher")]
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

    /// Returns a [`Transaction`] for the database for which this DatabasePool has connections. If
    /// the database types offers specific read replicas, such as AWS Neptune, the connection is to
    /// a read replica. If no separate read replicas are offered, the connection is to the same
    /// endpoint that serves read/write requests.
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
    /// # #[cfg(feature = "cypher")]
    /// # use warpgrapher::engine::database::cypher::CypherEndpoint;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "cypher")]
    /// let endpoint = CypherEndpoint::from_env()?;
    /// # #[cfg(feature = "cypher")]
    /// let pool = endpoint.pool().await?;
    /// # #[cfg(feature = "cypher")]
    /// let transaction = pool.read_transaction().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn read_transaction(&self) -> Result<Self::TransactionType, Error>;

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
    /// # #[cfg(feature = "cypher")]
    /// # use warpgrapher::engine::database::cypher::CypherEndpoint;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "cypher")]
    /// let endpoint = CypherEndpoint::from_env()?;
    /// # #[cfg(feature = "cypher")]
    /// let pool = endpoint.pool().await?;
    /// # #[cfg(feature = "cypher")]
    /// let transaction = pool.transaction().await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn transaction(&self) -> Result<Self::TransactionType, Error>;
}

#[async_trait]
pub trait Transaction: Send + Sync {
    async fn begin(&mut self) -> Result<(), Error>;

    async fn execute_query<RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
    ) -> Result<QueryResult, Error>;

    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        options: Options,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error>;

    #[allow(clippy::too_many_arguments)]
    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_query_fragment: QueryFragment,
        dst_query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        id_opt: Option<Value>,
        props: HashMap<String, Value>,
        options: Options,
        sg: &mut SuffixGenerator,
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

    async fn load_nodes<RequestCtx: RequestContext>(
        &mut self,
        keys: &[NodeLoaderKey],
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        options: Options,
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

    async fn load_rels<RequestCtx: RequestContext>(
        &mut self,
        keys: &[RelLoaderKey],
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        options: Options,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        options: Options,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    #[allow(clippy::too_many_arguments)]
    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        options: Options,
        sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        options: Options,
    ) -> Result<i32, Error>;

    async fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        options: Options,
    ) -> Result<i32, Error>;

    async fn commit(&mut self) -> Result<(), Error>;

    async fn rollback(&mut self) -> Result<(), Error>;
}

pub enum QueryResult {
    #[cfg(feature = "gremlin")]
    Gremlin(Vec<GValue>),

    #[cfg(feature = "cypher")]
    Cypher(Vec<Record>),

    NoDatabsae(),
}

/// Represents the different types of Crud Operations along with the target of the
/// operation (node typename and rel typename for rel ops). This enum is passed to
/// event handler functions.
#[derive(Clone, Debug)]
pub enum CrudOperation {
    ReadNode(String),
    ReadRel(String, String),
    CreateNode(String),
    CreateRel(String, String),
    UpdateNode(String),
    UpdateRel(String, String),
    DeleteNode(String),
    DeleteRel(String, String),
    None,
}

/// Represents the different type of comparison match operations
#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
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
    #[allow(dead_code)]
    operation: Operation,
    #[allow(dead_code)]
    operand: Value,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    match_fragment: String,
    #[allow(dead_code)]
    where_fragment: String,
    #[allow(dead_code)]
    params: HashMap<String, Value>,
}

impl QueryFragment {
    #[cfg(any(feature = "gremlin", feature = "cypher"))]
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

    #[cfg(feature = "cypher")]
    pub(crate) fn match_fragment(&self) -> &str {
        &self.match_fragment
    }

    #[cfg(any(feature = "gremlin", feature = "cypher"))]
    pub(crate) fn where_fragment(&self) -> &str {
        &self.where_fragment
    }

    #[cfg(any(feature = "gremlin", feature = "cypher"))]
    pub(crate) fn params(self) -> HashMap<String, Value> {
        self.params
    }
}

#[derive(Clone, Debug)]
pub struct NodeQueryVar {
    #[allow(dead_code)]
    base: String,
    #[allow(dead_code)]
    suffix: String,
    #[allow(dead_code)]
    label: Option<String>,
    #[allow(dead_code)]
    name: String,
}

impl NodeQueryVar {
    pub(crate) fn new(label: Option<String>, base: String, suffix: String) -> NodeQueryVar {
        NodeQueryVar {
            base: base.clone(),
            suffix: suffix.clone(),
            label,
            name: base + &*suffix,
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

    #[cfg(any(feature = "gremlin", feature = "cypher"))]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub struct RelQueryVar {
    label: String,
    #[allow(dead_code)]
    suffix: String,
    #[allow(dead_code)]
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
            name: "rel".to_string() + &*suffix,
            src,
            dst,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    #[cfg(any(feature = "cypher"))]
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
        "_".to_string() + &*self.seed.to_string()
    }
}
