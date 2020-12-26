//! Traits and helper structs for interacting with the graph storage database

#[cfg(any(feature = "cosmos", feature = "gremlin"))]
pub mod gremlin;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::engine::context::RequestContext;
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use async_trait::async_trait;
#[cfg(feature = "neo4j")]
use bb8::Pool;
#[cfg(feature = "neo4j")]
use bb8_bolt::BoltConnectionManager;
#[cfg(any(feature = "cosmos", feature = "gremlin"))]
use gremlin_client::GremlinClient;
use std::collections::HashMap;
use std::convert::TryFrom;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use std::env::var_os;
use std::fmt::Debug;

#[cfg(feature = "gremlin")]
fn env_bool(var_name: &str) -> Result<bool, Error> {
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

/// Contains a pool of database connections, or an enumeration variant indicating that there is no
/// back-end database
#[derive(Clone, Debug)]
pub enum DatabasePool {
    /// Contians a pool of Neo4J database clients
    #[cfg(feature = "neo4j")]
    Neo4j(Pool<BoltConnectionManager>),

    /// Contains a pool of Cosmos DB database clients
    #[cfg(feature = "cosmos")]
    Cosmos(GremlinClient),

    /// Contains a pool of Gremlin-based DB database clients, and a boolean indicating whether the
    /// back-end database stores identifiers using a UUID type (true) or a UUID in a String type
    /// (false).
    #[cfg(feature = "gremlin")]
    Gremlin((GremlinClient, bool)),

    /// Used to serve the schema without a database backend
    NoDatabase,
}

impl DatabasePool {
    #[cfg(feature = "neo4j")]
    pub fn neo4j(&self) -> Result<&bb8::Pool<bb8_bolt::BoltConnectionManager>, Error> {
        match self {
            DatabasePool::Neo4j(pool) => Ok(pool),
            _ => Err(Error::DatabaseNotFound {}),
        }
    }

    #[cfg(feature = "cosmos")]
    pub fn cosmos(&self) -> Result<&GremlinClient, Error> {
        match self {
            DatabasePool::Cosmos(pool) => Ok(pool),
            _ => Err(Error::DatabaseNotFound {}),
        }
    }

    #[cfg(feature = "gremlin")]
    pub fn gremlin(&self) -> Result<&GremlinClient, Error> {
        match self {
            DatabasePool::Gremlin((pool, _)) => Ok(pool),
            _ => Err(Error::DatabaseNotFound {}),
        }
    }
}

impl Default for DatabasePool {
    fn default() -> Self {
        DatabasePool::NoDatabase
    }
}

/// Trait for a database endpoint. Structs that implement this trait typically take in a connection
/// string and produce a database pool of clients connected to the database
#[async_trait]
pub trait DatabaseEndpoint {
    /// Returns a [`DatabasePool`] to the database for which this DatabaseEndpoint has connection
    /// information
    ///
    /// [`DatabasePool`]: ./enum.DatabasePool.html
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
    async fn pool(&self) -> Result<DatabasePool, Error>;
}

#[derive(Clone, Debug)]
pub enum Operation {
    EQ,
    CONTAINS,
    IN,
    GT,
    GTE,
    LT,
    LTE
}

#[derive(Clone, Debug)]
pub struct Comparison {
    operation: Operation,
    operand: Value,
    negated: bool
}

impl Comparison {

    pub fn new(operation: Operation, negated: bool, operand: Value) -> Self {
        Comparison {
            operation,
            operand,
            negated
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
                let operation_str = m.keys().nth(0).unwrap(); // TODO: handle error
                let operand = m.values().nth(0).unwrap(); // TODO: handle error
                Comparison::new(
                    match operation_str.as_ref() {
                        "EQ" => Operation::EQ,
                        "NOTEQ" => Operation::EQ,
                        "CONTAINS" => Operation::CONTAINS,
                        "NOTCONTAINS" => Operation::CONTAINS,
                        "IN" => Operation::IN,
                        "NOTIN" => Operation::IN,
                        "GT" => Operation::GT,
                        "NOTGT" => Operation::GT,
                        "GTE" => Operation::GTE,
                        "NOTGTE" => Operation::GTE,
                        "LT" => Operation::LT,
                        "NOTLT" => Operation::LT,
                        "LTE" => Operation::LTE,
                        "NOTLTE" => Operation::LTE,
                        _ => panic!("unknown operation") // TODO: return error
                    },
                    match operation_str.as_ref() {
                        "NOTEQ" |
                        "NOTCONTAINS" |
                        "NOTIN" |
                        "NOTGT" |
                        "NOTGTE" |
                        "NOTLT" | 
                        "NOTLTE" => true,
                        _ => false
                    },
                    operand.clone(), // TODO: use reference?
                )
            },
            _ => {
                //return Err(Error::ComparisonParsingFailed)
                panic!("need custom error");
            }
        })
    }
}

pub(crate) trait Transaction {
    fn begin(&mut self) -> Result<(), Error>;

    fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<RequestCtx>, Error>;

    fn create_rels<RequestCtx: RequestContext>(
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
        //props: HashMap<String, Value>,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error>;

    fn read_nodes<RequestCtx: RequestContext>(
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
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error>;

    fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    fn commit(&mut self) -> Result<(), Error>;

    fn rollback(&mut self) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub(crate) struct QueryFragment {
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
pub(crate) struct NodeQueryVar {
    base: String,
    suffix: String,
    label: Option<String>,
    name: String,
}

impl NodeQueryVar {
    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn new(label: Option<String>, base: String, suffix: String) -> NodeQueryVar {
        NodeQueryVar {
            base: base.clone(),
            suffix: suffix.clone(),
            label,
            name: base + &suffix,
        }
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn base(&self) -> &str {
        &self.base
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn label(&self) -> Result<&str, Error> {
        self.label.as_deref().ok_or(Error::LabelNotFound)
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn suffix(&self) -> &str {
        &self.suffix
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RelQueryVar {
    label: String,
    suffix: String,
    name: String,
    src: NodeQueryVar,
    dst: NodeQueryVar,
}

impl RelQueryVar {
    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
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

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    #[cfg(any(feature = "neo4j"))]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn src(&self) -> &NodeQueryVar {
        &self.src
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn dst(&self) -> &NodeQueryVar {
        &self.dst
    }
}

#[derive(Debug, Default)]
pub(crate) struct SuffixGenerator {
    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    seed: i32,
}

impl SuffixGenerator {
    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn new() -> SuffixGenerator {
        SuffixGenerator { seed: -1 }
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn suffix(&mut self) -> String {
        self.seed += 1;
        "_".to_string() + &self.seed.to_string()
    }
}
