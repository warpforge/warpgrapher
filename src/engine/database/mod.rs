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
use juniper::FieldError;
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

pub trait Transaction {
    type ImplQueryResult: QueryResult + Debug;
    fn begin(&self) -> Result<(), FieldError>;
    fn commit(&mut self) -> Result<(), FieldError>;
    fn create_node<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, FieldError>
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
        partition_key_opt: &Option<String>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Value,
        partition_key_opt: &Option<String>,
    ) -> Result<i32, FieldError>;
    fn delete_rels(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        info: &Info,
    ) -> Result<i32, FieldError>;
    fn exec(
        &mut self,
        query: &str,
        partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    fn update_nodes<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        ids: Value,
        props: HashMap<String, Value>,
        partition_key_opt: &Option<String>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
    #[allow(clippy::too_many_arguments)]
    fn update_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;

    #[allow(clippy::too_many_arguments)]
    fn node_query_string(
        &mut self,
        rel_query_fragments: Vec<String>,
        params: &mut HashMap<String, Value>,
        label: &str,
        var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<String, FieldError>;

    #[allow(clippy::too_many_arguments)]
    fn rel_query_string(
        &mut self,
        // query: &str,
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
        params: &mut HashMap<String, Value>,
    ) -> Result<String, FieldError>;

    fn rollback(&mut self) -> Result<(), FieldError>;
}

pub trait QueryResult: Debug {
    fn nodes<GlobalCtx, RequestCtx>(
        self,
        name: &str,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;

    #[allow(clippy::too_many_arguments)]
    fn rels<GlobalCtx, RequestCtx>(
        self,
        src_name: &str,
        src_suffix: &str,
        rel_name: &str,
        dst_name: &str,
        dst_suffix: &str,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext;
    fn ids(&self, column_name: &str) -> Result<Value, FieldError>;
    fn count(&self) -> Result<i32, FieldError>;
    fn len(&self) -> i32;
    fn is_empty(&self) -> bool;
}
