#[cfg(feature = "cosmos")]
pub mod cosmos;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
#[cfg(feature = "cosmos")]
use gremlin_client::GremlinClient;
#[cfg(feature = "neo4j")]
use r2d2::Pool;
#[cfg(feature = "neo4j")]
use r2d2_cypher::CypherConnectionManager;
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

#[cfg(any(feature = "cosmos"))]
fn env_u16(var_name: &str) -> Result<u16, Error> {
    Ok(env_string(var_name)?.parse::<u16>()?)
}

#[derive(Clone, Debug)]
pub enum DatabasePool {
    #[cfg(feature = "neo4j")]
    Neo4j(Pool<CypherConnectionManager>),
    #[cfg(feature = "cosmos")]
    Cosmos(GremlinClient),
    // Used to serve the schema without a database backend
    NoDatabase,
}

impl Default for DatabasePool {
    fn default() -> Self {
        DatabasePool::NoDatabase
    }
}

pub trait DatabaseEndpoint {
    fn pool(&self) -> Result<DatabasePool, Error>;
}

pub(crate) trait Transaction {
    type ImplDeleteQueryResponse: DeleteQueryResponse;
    type ImplNodeQueryResponse: NodeQueryResponse;
    type ImplRelQueryResponse: RelQueryResponse;

    fn begin(&self) -> Result<(), Error>;

    fn create_node<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Self::ImplNodeQueryResponse, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
    #[allow(clippy::too_many_arguments)]
    fn create_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        src_ids: Value,
        dst_label: &str,
        dst_ids: Value,
        rel_name: &str,
        params: &mut HashMap<String, Value>, // TODO Pass props instead of params
        partition_key_opt: Option<&Value>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Self::ImplRelQueryResponse, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
    #[allow(clippy::too_many_arguments)]
    fn node_query(
        &mut self,
        rel_query_fragments: Vec<String>,
        params: HashMap<String, Value>,
        label: &str,
        var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    #[allow(clippy::too_many_arguments)]
    fn rel_query(
        &mut self,
        src_label: &str,
        src_suffix: &str,
        src_ids_opt: Option<Value>,
        src_query: Option<String>,
        rel_name: &str,
        dst_var: &str,
        dst_suffix: &str,
        dst_query: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
        params: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error>;

    fn read_nodes(
        &mut self,
        query: &str,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Self::ImplNodeQueryResponse, Error>;
    fn read_rels(
        &mut self,
        query: &str,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Self::ImplRelQueryResponse, Error>;
    fn update_nodes<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        ids: Value,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Self::ImplNodeQueryResponse, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;

    #[allow(clippy::too_many_arguments)]
    fn update_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Self::ImplRelQueryResponse, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;

    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Value,
        partition_key_opt: Option<&Value>,
    ) -> Result<Self::ImplDeleteQueryResponse, Error>;
    fn delete_rels(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Self::ImplDeleteQueryResponse, Error>;

    fn commit(&mut self) -> Result<(), Error>;
    fn rollback(&mut self) -> Result<(), Error>;
}

pub(crate) trait DeleteQueryResponse: Debug {
    fn count(&self) -> Result<i32, Error>;
}

pub(crate) trait NodeQueryResponse: Debug {
    fn ids(&self, column_name: &str) -> Result<Value, Error>;
    fn nodes<GlobalCtx, RequestCtx>(
        self,
        name: &str,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
}

pub(crate) trait RelQueryResponse: Debug {
    fn ids(&self, column_name: &str) -> Result<Value, Error>;
    fn merge(&mut self, r: Self);
    fn rels<GlobalCtx, RequestCtx>(
        &mut self,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
}
