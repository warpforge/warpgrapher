//! Traits and helper structs for interacting with the graph storage database

#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use async_trait::async_trait;
#[cfg(feature = "neo4j")]
use bb8::Pool;
#[cfg(feature = "neo4j")]
use bb8_bolt::BoltConnectionManager;
#[cfg(feature = "cosmos")]
use gremlin_client::GremlinClient;
use std::collections::HashMap;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use std::env::var_os;
use std::fmt::Debug;

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
fn env_string(var_name: &str) -> Result<String, Error> {
    var_os(var_name)
        .map(|osstr| osstr.to_string_lossy().into_owned())
        .ok_or_else(|| Error::EnvironmentVariableNotFound {
            name: var_name.to_string(),
        })
}

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
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

    /// Used to serve the schema without a database backend
    NoDatabase,
}

impl DatabasePool {
    #[cfg(feature = "neo4j")]
    pub(crate) fn neo4j(&self) -> Result<&bb8::Pool<bb8_bolt::BoltConnectionManager>, Error> {
        match self {
            DatabasePool::Neo4j(pool) => Ok(pool),
            _ => Err(Error::DatabaseNotFound {}),
        }
    }

    #[cfg(feature = "cosmos")]
    pub(crate) fn cosmos(&self) -> Result<&GremlinClient, Error> {
        match self {
            DatabasePool::Cosmos(pool) => Ok(pool),
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

pub(crate) trait Transaction {
    fn begin(&mut self) -> Result<(), Error>;

    fn node_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error>;

    fn rel_create_fragment<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        dst_query: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn rel_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query_opt: Option<String>,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        rel_vars: Vec<RelQueryVar>,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>;

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<(String, String)>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error>;

    fn node_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: Option<HashMap<String, Value>>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>;

    fn rel_read_fragment(
        &mut self,
        src_query_opt: Option<(String, String)>,
        dst_query_opt: Option<(String, String)>,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error>;

    fn rel_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: Option<HashMap<String, Value>>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>;

    #[allow(clippy::too_many_arguments)]
    fn node_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        change_queries: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>;

    fn rel_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>;

    fn node_delete_query(
        &mut self,
        match_query: String,
        rel_delete_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn delete_nodes(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    #[allow(clippy::too_many_arguments)]
    fn rel_delete_query(
        &mut self,
        match_query: String,
        src_delete_query_opt: Option<String>,
        dst_delete_query_opt: Option<String>,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn delete_rels(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error>;

    fn commit(&mut self) -> Result<(), Error>;

    fn rollback(&mut self) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub(crate) struct NodeQueryVar {
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
        self.label.as_deref().ok_or_else(|| Error::LabelNotFound)
    }

    pub(crate) fn suffix(&self) -> &str {
        &self.suffix
    }

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

#[derive(Clone, Copy, Debug)]
pub(crate) enum ClauseType {
    Parameter,
    FirstSubQuery,
    SubQuery,
    Query,
}

#[derive(Default)]
pub(crate) struct SuffixGenerator {
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
