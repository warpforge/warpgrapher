#[cfg(feature = "graphson2")]
pub mod graphson2;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::error::Error;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use crate::error::ErrorKind;
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
use crate::server::value::Value;
#[cfg(feature = "graphson2")]
use gremlin_client::GremlinClient;
use juniper::FieldError;
#[cfg(feature = "neo4j")]
use r2d2::Pool;
#[cfg(feature = "neo4j")]
use r2d2_cypher::CypherConnectionManager;
use std::collections::HashMap;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use std::env::var_os;
use std::fmt::Debug;

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
fn get_env_string(var_name: &str) -> Result<String, Error> {
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

#[cfg(any(feature = "graphson2"))]
fn get_env_u16(var_name: &str) -> Result<u16, Error> {
    Ok(get_env_string(var_name)?
        .parse::<u16>()
        .map_err(|_| Error::new(ErrorKind::EnvironmentVariableParseError, None))?)
}

#[derive(Clone, Debug)]
pub enum DatabasePool {
    #[cfg(feature = "neo4j")]
    Neo4j(Pool<CypherConnectionManager>),
    #[cfg(feature = "graphson2")]
    Graphson2(GremlinClient),
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
    fn create_node(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    #[allow(clippy::too_many_arguments)]
    fn create_rels(
        &mut self,
        src_label: &str,
        src_ids: Value,
        dst_label: &str,
        dst_ids: Value,
        rel_name: &str,
        params: &mut HashMap<String, Value>, // TODO Pass props instead of params
        partition_key_opt: &Option<String>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Value,
        partition_key_opt: &Option<String>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    fn exec(
        &mut self,
        query: &str,
        partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    fn update_nodes(
        &mut self,
        label: &str,
        ids: Value,
        props: HashMap<String, Value>,
        partition_key_opt: &Option<String>,
    ) -> Result<Self::ImplQueryResult, FieldError>;
    fn update_rels(
        &mut self,
        src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
    ) -> Result<Self::ImplQueryResult, FieldError>;

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
    fn get_nodes<GlobalCtx, ReqCtx>(
        self,
        name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug;
    fn get_rels<GlobalCtx, ReqCtx>(
        self,
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
    fn get_ids(&self, column_name: &str) -> Result<Value, FieldError>;
    fn get_count(&self) -> Result<i32, FieldError>;
    fn len(&self) -> i32;
    fn is_empty(&self) -> bool;
}
