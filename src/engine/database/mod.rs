//! Traits and helper structs for interacting with the graph storage database

#[cfg(any(feature = "cosmos", feature = "gremlin"))]
pub mod gremlin;
#[cfg(feature = "neo4j")]
pub mod neo4j;

use crate::engine::context::RequestContext;
use crate::engine::objects::{Node, Rel};
use crate::engine::objects::resolvers::visitors;
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

#[async_trait]
pub trait Transaction<RequestCtx: RequestContext>: Send {
    async fn begin(&mut self) -> Result<(), Error>;

    async fn create_node(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<RequestCtx>, Error>;

    async fn create_rels(
        &mut self,
        src_query_fragment: QueryFragment,
        dst_query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    fn node_read_by_ids_fragment(
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

    async fn read_nodes(
        &mut self,
        node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    fn rel_read_by_ids_fragment(
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

    async fn read_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error>;

    async fn update_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error>;

    async fn update_rels(
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
pub struct RelQueryVar {
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
pub struct SuffixGenerator {
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


/*

# create node
node("Project")             -> NodeCrud
    .create(json!({}))
    .await;

# read all nodes
node("Project")
    .all()                  -> NodeMatchedCrud
    .read()
    .await;

# read matching nodes
node("Project")
    .matching(json!({}))
    .read()
    .await;

# update all nodes
node("Project")
    .all()
    .update(json!({}))
    .await;

# update matching nodes
node("Project")
    .matching(json!({}))
    .update(json!({}))
    .await;

# delete matching node
node("Project")
    .matching(json!({}))
    .delete()

# create rel
node("Project")
    .matching(json!({}))
    .rel("owner")
    .create(json!({}))
    .await


*/

pub struct WarpCrud<'a, Rctx: RequestContext> {
    //partition_key: Option<Value>,
    transaction: &'a mut dyn Transaction<Rctx>,
    info: &'a Info,
}

impl<'a, Rctx: RequestContext> WarpCrud<'a, Rctx> {

    pub fn new(
        //partition_key: Option<Value>,
        transaction: &'a mut dyn Transaction<Rctx>,
        info: &'a Info,
    ) -> Self {
        Self {
            //partition_key,
            transaction,
            info
        }
    }

    pub fn node(&mut self, type_name: &str) -> NodeCrud<'_, Rctx> {
        NodeCrud::<'_, Rctx>::new(
            //self.partition_key, 
            self.transaction, 
            self.info,
            type_name.to_string(), 
        )
    }

}

pub struct NodeCrud<'a, Rctx: RequestContext> {
    //partition_key: Option<Value>,
    transaction: &'a mut dyn Transaction<Rctx>,
    info: &'a Info,
    type_name: String,
}

impl<'a, Rctx: RequestContext> NodeCrud<'a, Rctx> {

    fn new(
        //partition_key: Option<Value>,
        transaction: &'a mut dyn Transaction<Rctx>,
        info: &'a Info,
        type_name: String,
    ) -> Self {
        Self {
            //partition_key,
            transaction,
            info,
            type_name
        }
    }

    /// # Examples
    /// 
    /// ```rust, no_run
    /// let project_orion = crud
    ///     .node("Project")
    ///     .matching(json!({"name": "ORION"}))
    ///     .read()
    ///     .await?;
    /// ```
    fn matching(&mut self, match_input: Value) -> MatchedNodeCrud<'_, Rctx> {
        MatchedNodeCrud::<'_, Rctx>::new(
            //self.partition_key, 
            self.transaction, 
            self.info,
            self.type_name.clone(),
            Some(match_input)
        )
    }

    /// # Examples
    /// 
    /// ```rust, no_run
    /// let all_projects = crud
    ///     .node("Project")
    ///     .all()
    ///     .read()
    ///     .await?;
    /// ```
    fn all(&mut self) -> MatchedNodeCrud<'_, Rctx> {
        MatchedNodeCrud::<'_, Rctx>::new(
            //self.partition_key, 
            self.transaction, 
            self.info,
            self.type_name.clone(),
            None
        )
    }

    /*
    /// # Examples
    /// 
    /// ```rust, no_run
    /// let projects = crud
    ///     .node("Project")
    ///     .create(json!({
    ///         "name": "ORION"
    ///     }))
    ///     .await?;
    /// ```
    async fn create(&mut self, input: Value) -> Result<Node<Rctx>, Error> {
        Err(Error)
    }
    */

}

pub struct MatchedNodeCrud<'a, Rctx: RequestContext> {
    //partition_key: Option<Value>,
    transaction: &'a mut dyn Transaction<Rctx>,
    info: &'a Info,
    type_name: String,
    match_input: Option<Value>,
}

impl<'a, Rctx: RequestContext> MatchedNodeCrud<'a, Rctx> {

    fn new(
        //partition_key: Option<Value>,
        transaction: &'a mut dyn Transaction<Rctx>,
        info: &'a Info,
        type_name: String,
        match_input: Option<Value>,
    ) -> Self {
        Self {
            //partition_key,
            transaction,
            info,
            type_name,
            match_input
        }
    }

    /// # Examples
    /// 
    /// ```rust, no_run
    /// let projects = crud
    ///     .node("Project")
    ///     .all()
    ///     .read()
    ///     .await;
    /// ```
    async fn read(&mut self) -> Result<Vec<Node<Rctx>>, Error> {
        let mut info = self.info.clone();
        info.name = "Query".to_string();

        let mut sg = SuffixGenerator::new();
        let node_var = NodeQueryVar::new(
            Some(self.type_name.clone()),
            "node".to_string(),
            sg.suffix(),
        );

        let query_fragment = visitors::visit_node_query_input(
            &node_var,
            self.match_input.clone(),
            &Info::new(self.type_name.clone(), info.type_defs()),
            //self.partition_key,
            None,
            &mut sg,
            self.transaction,
        )
        .await?;

        let results = self
            .transaction
            .read_nodes(
                &node_var, 
                query_fragment, 
                //self.partition_key, 
                None,
                &info
            )
            .await;
        results
    }

    fn update() -> Result<Vec<Node<Rctx>>, Error> {
        Ok(vec![])
    }

    fn delete() -> Result<i64, Error> {
        Ok(0)
    }

    /*
    fn rel() -> RelCrud {

    }
    */
}