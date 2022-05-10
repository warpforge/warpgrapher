//! Provides database interface types and functions for cypher-based databases.

use crate::engine::context::RequestContext;
use crate::engine::database::{
    env_string, env_u16, Comparison, DatabaseEndpoint, DatabasePool, NodeQueryVar, Operation,
    QueryFragment, QueryResult, RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::loader::{NodeLoaderKey, RelLoaderKey};
use crate::engine::objects::{Direction, Node, NodeRef, Options, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use bolt_client::{Metadata, Params};
use bolt_proto::error::ConversionError;
use bolt_proto::message::{Message, Record};
use log::{debug, trace};
use mobc::{Connection, Pool};
use mobc_bolt::Manager;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;
use uuid::Uuid;

/// A Cypher endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::cypher::CypherEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ne = CypherEndpoint::new(
///         "127.0.0.1".to_string(),
///         Some("127.0.0.1".to_string()),
///         7687,
///         "neo4j".to_string(),
///         "password".to_string(),
///         8
///     );
/// #    Ok(())
/// # }
/// ```
pub struct CypherEndpoint {
    host: String,
    read_host: String,
    port: u16,
    user: String,
    pass: String,
    pool_size: u16,
}

impl CypherEndpoint {
    /// Returns a new [`CypherEndpoint`] from the provided values.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::database::cypher::CypherEndpoint;
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ne = CypherEndpoint::new(
    ///         "127.0.0.1".to_string(),
    ///         Some("127.0.0.1".to_string()),
    ///         7687,
    ///         "neo4j".to_string(),
    ///         "password".to_string(),
    ///         8
    ///     );
    /// #    Ok(())
    /// # }
    /// ```
    pub fn new(
        host: String,
        read_host_opt: Option<String>,
        port: u16,
        user: String,
        pass: String,
        pool_size: u16,
    ) -> Self {
        CypherEndpoint {
            host: host.to_string(),
            read_host: read_host_opt.unwrap_or(host),
            port,
            user,
            pass,
            pool_size,
        }
    }

    /// Reads an variable to construct a [`CypherEndpoint`]. The environment variable is
    ///
    /// * WG_CYPHER_ADDR - the address for the Cypher-based DB. For example, `127.0.0.1`.
    /// * WG_CYPHER_READ_REPLICAS - the address for Cypher-based read replicas. For example `127.0.0.1`. Optional.
    /// * WG_CYPHER_PORT - the port number for the Cypher-based DB.  For example, `7687`.
    /// * WG_CYPHER_USER - the username for the Cypher-based DB. For example, `neo4j`.
    /// * WG_CYPHER_PASS - the password for the Cypher-based DB. For example, `my-db-pass`.
    /// * WG_POOL_SIZE - connection pool size. For example, `4`. Optional.
    ///
    /// [`CypherEndpoint`]: ./struct.CypherEndpoint.html
    ///
    /// # Errors
    ///
    /// * [`EnvironmentVariableNotFound`] - if an environment variable does not exist
    ///
    /// [`EnvironmentVariableNotFound`]: ../../enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::engine::database::cypher::CypherEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ne = CypherEndpoint::from_env()?;
    ///     # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<Self, Error> {
        Ok(CypherEndpoint {
            host: env_string("WG_CYPHER_HOST")?,
            read_host: env_string("WG_CYPHER_READ_REPLICAS")
                .or_else(|_| env_string("WG_CYPHER_HOST"))?,
            port: env_u16("WG_CYPHER_PORT")?,
            user: env_string("WG_CYPHER_USER")?,
            pass: env_string("WG_CYPHER_PASS")?,
            pool_size: env_u16("WG_POOL_SIZE")
                .unwrap_or_else(|_| num_cpus::get().try_into().unwrap_or(8)),
        })
    }
}

#[async_trait]
impl DatabaseEndpoint for CypherEndpoint {
    type PoolType = CypherDatabasePool;

    async fn pool(&self) -> Result<Self::PoolType, Error> {
        let rw_manager = Manager::new(
            self.host.to_string() + ":" + &*self.port.to_string(),
            None,
            [4, 0, 0, 0],
            Metadata::from_iter(vec![
                ("user_agent", "warpgrapher/0.2.0"),
                ("scheme", "basic"),
                ("principal", &self.user),
                ("credentials", &self.pass),
            ]),
        )
        .await?;

        let ro_manager = Manager::new(
            self.read_host.to_string() + ":" + &*self.port.to_string(),
            None,
            [4, 0, 0, 0],
            Metadata::from_iter(vec![
                ("user_agent", "warpgrapher/0.2.0"),
                ("scheme", "basic"),
                ("principal", &self.user),
                ("credentials", &self.pass),
            ]),
        )
        .await?;

        let pool = CypherDatabasePool::new(
            Pool::builder()
                .max_open(self.pool_size.into())
                .build(rw_manager),
            Pool::builder()
                .max_open(self.pool_size.into())
                .build(ro_manager),
        );

        Ok(pool)
    }
}

#[derive(Clone)]
pub struct CypherDatabasePool {
    rw_pool: Pool<Manager>,
    ro_pool: Pool<Manager>,
}

impl CypherDatabasePool {
    fn new(rw_pool: Pool<Manager>, ro_pool: Pool<Manager>) -> Self {
        CypherDatabasePool { rw_pool, ro_pool }
    }
}

#[async_trait]
impl DatabasePool for CypherDatabasePool {
    type TransactionType = CypherTransaction;

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(CypherTransaction::new(self.ro_pool.get().await?))
    }

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(CypherTransaction::new(self.rw_pool.get().await?))
    }
}

pub struct CypherTransaction {
    client: Connection<Manager>,
}

impl CypherTransaction {
    pub fn new(client: Connection<Manager>) -> CypherTransaction {
        CypherTransaction { client }
    }

    fn add_sort_to_query(
        query: String,
        options: Options,
        name: &str,
        dst_name: Option<&str>,
    ) -> String {
        options
            .sort()
            .iter()
            .enumerate()
            .fold(query, |mut q, (i, sort)| {
                if i == 0 {
                    q += "ORDER BY"
                } else {
                    q += ","
                }

                if sort.dst_property() {
                    q += &(" ".to_string() + dst_name.unwrap_or("") + "." + sort.property());
                } else {
                    q += &(" ".to_string() + name + "." + sort.property());
                }

                if sort.direction() == &Direction::Descending {
                    q += " DESC";
                }

                q
            })
            + "\n"
    }
}

#[async_trait]
impl Transaction for CypherTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        debug!("CypherTransaction::begin called");

        let response = self.client.begin(None).await;
        match response {
            Ok(Message::Success(_)) => Ok(()),
            Ok(message) => Err(Error::CypherQueryFailed { message }),
            Err(e) => Err(Error::from(e)),
        }
    }

    #[tracing::instrument(name = "wg-cypher-execute-query", skip(self, query, params))]
    async fn execute_query<RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
    ) -> Result<QueryResult, Error> {
        trace!(
            "CypherTransaction::execute_query called -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        Ok(QueryResult::Cypher(records))
    }

    #[tracing::instrument(
        name = "wg-cypher-create-node",
        skip(self, node_var, props, options, _info, _sg)
    )]
    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        mut props: HashMap<String, Value>,
        options: Options,
        _info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error> {
        trace!(
            "CypherTransaction::create_node called -- node_var: {:#?}, props: {:#?}",
            node_var,
            props
        );

        if !props.contains_key("id") {
            props.insert(
                "id".to_string(),
                Value::String(Uuid::new_v4().to_hyphenated().to_string()),
            );
        }

        let mut query = "CREATE (n:".to_string()
            + node_var.label()?
            + ")\n"
            + "SET n += $props\n"
            + "RETURN n\n";
        query = CypherTransaction::add_sort_to_query(query, options, "n", None);

        let mut params: HashMap<&str, Value> = HashMap::new();
        params.insert("props", props.into());

        trace!(
            "CypherTransaction::create_node -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (mut records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        records.pop().ok_or(Error::ResponseSetNotFound)?.try_into()
    }

    #[tracing::instrument(
        name = "wg-cypher-create-rels",
        skip(self, src_fragment, dst_fragment, rel_var, props, options, _sg)
    )]
    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_fragment: QueryFragment,
        dst_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        id_opt: Option<Value>,
        mut props: HashMap<String, Value>,
        options: Options,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("CypherTransaction::create_rels called -- src_query: {:#?}, dst_query: {:#?}, rel_var: {:#?}, props: {:#?}, options: {:#?}",
        src_fragment, dst_fragment, rel_var, props, options);

        let mut query = src_fragment.match_fragment().to_string()
            + dst_fragment.match_fragment()
            + "MATCH ("
            + rel_var.src().name()
            + ":"
            + rel_var.src().label()?
            + "), ("
            + rel_var.dst.name()
            + ")\n"
            + "WHERE "
            + src_fragment.where_fragment()
            + " AND "
            + dst_fragment.where_fragment()
            + "\n"
            + "CREATE ("
            + rel_var.src().name()
            + ")-["
            + rel_var.name()
            + ":"
            + rel_var.label()
            + if id_opt.is_none() {
                " { id: randomUUID() }]->("
            } else {
                "]->("
            }
            + rel_var.dst().name()
            + ")\n"
            + "SET "
            + rel_var.name()
            + " += $props\n"
            + "RETURN "
            + rel_var.src.name()
            + " {.id} "
            + " as src, "
            + rel_var.name()
            + " as rel, "
            + rel_var.dst.name()
            + " {.id} "
            + " as dst\n";

        query = CypherTransaction::add_sort_to_query(query, options, "rel", Some("dst"));

        if let Some(id_val) = id_opt {
            props.insert("id".to_string(), id_val);
        }

        let mut params = src_fragment.params();
        params.extend(dst_fragment.params());
        params.insert("props".to_string(), props.into());

        trace!(
            "CypherTransaction::create_rels -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        trace!("Reached record pull");
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!("Rel Records: {:#?}", records);
        records
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
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

        let match_query = "MATCH (".to_string() + node_var.name() + ":" + node_var.label()? + ")\n";
        let where_query = node_var.name().to_string() + ".id IN $id_list";

        let ids = nodes
            .iter()
            .map(|n| n.id())
            .collect::<Result<Vec<&Value>, Error>>()?
            .into_iter()
            .cloned()
            .collect();
        let mut params = HashMap::new();
        params.insert("id_list".to_string(), Value::Array(ids));

        Ok(QueryFragment::new(match_query, where_query, params))
    }

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<QueryFragment>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        trace!("CypherTransaction::node_read_fragment called -- rel_query_fragment: {:#?}, node_var: {:#?}, props: {:#?}, sg: {:#?}",
        rel_query_fragments, node_var, props, sg);

        let param_suffix = sg.suffix();
        let mut match_fragment = String::new();
        let mut where_fragment = String::new();
        let mut params = HashMap::new();

        if rel_query_fragments.is_empty() {
            if node_var.label().is_ok() {
                match_fragment.push_str(
                    &("MATCH (".to_string() + node_var.name() + ":" + node_var.label()? + ")\n"),
                );
            } else {
                match_fragment.push_str(&("MATCH (".to_string() + node_var.name() + ")\n"));
            }
        }

        if !props.is_empty() {
            let mut value_props: HashMap<String, Value> = HashMap::new();
            props.into_iter().enumerate().for_each(|(i, (k, c))| {
                if i > 0 {
                    where_fragment.push_str(" AND ");
                }
                if c.negated {
                    where_fragment.push_str(" NOT ")
                }
                where_fragment.push_str(
                    &(node_var.name().to_string()
                        + "."
                        + &*k
                        + " "
                        + &*cypher_comparison_operator(&c.operation)
                        + " "
                        + "$param"
                        + &*param_suffix
                        + "."
                        + &*k),
                );
                value_props.insert(k, c.operand);
            });
            params.insert("param".to_string() + &*param_suffix, value_props.into());
        }

        rel_query_fragments.into_iter().for_each(|rqf| {
            match_fragment.push_str(rqf.match_fragment());
            if !where_fragment.is_empty() {
                where_fragment.push_str(" AND ");
            }
            where_fragment.push_str(rqf.where_fragment());

            params.extend(rqf.params());
        });

        let qf = QueryFragment::new(match_fragment, where_fragment, params);
        trace!("CypherTransaction::node_read_fragment returning {:#?}", qf);

        Ok(qf)
    }

    #[tracing::instrument(level = "info", name = "wg-cypher-load-nodes", skip(self, _info))]
    async fn load_nodes<RequestCtx: RequestContext>(
        &mut self,
        keys: &[NodeLoaderKey],
        _info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("CypherTransaction::load_nodes called -- keys: {:#?}", keys);

        let mut query = String::new();
        let mut params: HashMap<String, Vec<String>> = HashMap::new();

        query.push_str("MATCH (n)\n");
        query.push_str("WHERE n.id IN $id_list\n");
        query.push_str("RETURN n\n");

        params.insert(
            "id_list".to_string(),
            keys.iter().map(|nlk| nlk.id().to_string()).collect(),
        );

        trace!(
            "CypherTransaction::load_nodes -- query: {}, params: {:#?}",
            query,
            params
        );
        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!(
            "CypherTransaction::load_nodes -- node records: {:#?}",
            records
        );

        records
            .into_iter()
            .map(|n| n.try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        name = "wg-cypher-read-nodes",
        skip(self, query_fragment, node_var, options, info)
    )]
    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        options: Options,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!(
            "CypherTransaction::read_nodes called -- node_var: {:#?}, query_fragment: {:#?}, info.name: {}",
            node_var,
            query_fragment,
            info.name()
        );

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let mut query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "RETURN "
            + "DISTINCT "
            + node_var.name()
            + "\n";
        query = CypherTransaction::add_sort_to_query(query, options, node_var.name(), None);
        let params = query_fragment.params();

        trace!(
            "CypherTransaction::read_nodes -- query: {}, params: {:#?}",
            query,
            params
        );
        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!("Rel Records: {:#?}", records);

        records
            .into_iter()
            .map(|n| n.try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
    }

    fn rel_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        rel_var: &RelQueryVar,
        rels: &[Rel<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        trace!(
            "CypherTransaction::rel_read_by_ids_query called -- rel_var: {:#?}, rels: {:#?}",
            rel_var,
            rels
        );

        let match_query = "MATCH (".to_string()
            + rel_var.src().name()
            + ")-["
            + rel_var.name()
            + ":"
            + rel_var.label()
            + "]->("
            + rel_var.dst().name()
            + ")\n";

        let where_query = rel_var.name().to_string() + ".id IN $id_list\n";

        let ids = rels
            .iter()
            .map(|r| r.id())
            .collect::<Result<Vec<&Value>, Error>>()?
            .into_iter()
            .cloned()
            .collect();
        let mut params = HashMap::new();
        params.insert("id_list".to_string(), Value::Array(ids));

        Ok(QueryFragment::new(match_query, where_query, params))
    }

    fn rel_read_fragment(
        &mut self,
        src_fragment_opt: Option<QueryFragment>,
        dst_fragment_opt: Option<QueryFragment>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Comparison>,
        sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        trace!("CypherTransaction::rel_read_fragment called -- src_fragment_opt: {:#?}, dst_fragment_opt: {:#?}, rel_var: {:#?}, props: {:#?}",
        src_fragment_opt, dst_fragment_opt, rel_var, props);

        let mut match_fragment = String::new();
        let mut where_fragment = String::new();
        let mut params = HashMap::new();

        if let Some(src_fragment) = src_fragment_opt {
            match_fragment.push_str(src_fragment.match_fragment());
            where_fragment.push_str(src_fragment.where_fragment());
            params.extend(src_fragment.params());

            if dst_fragment_opt.is_some() || !props.is_empty() {
                where_fragment.push_str(" AND ");
            }
        }

        if let Some(dst_fragment) = dst_fragment_opt {
            match_fragment.push_str(dst_fragment.match_fragment());
            where_fragment.push_str(dst_fragment.where_fragment());
            params.extend(dst_fragment.params());
        }

        match_fragment.push_str(
            &("MATCH (".to_string()
                + rel_var.src().name()
                + ":"
                + rel_var.src().label()?
                + ")-["
                + rel_var.name()
                + ":"
                + rel_var.label()
                + "]->("
                + rel_var.dst().name()
                + ")\n"),
        );

        let param_var = "param".to_string() + &*sg.suffix();
        if !props.is_empty() {
            let mut value_props: HashMap<String, Value> = HashMap::new();
            props.into_iter().enumerate().for_each(|(i, (k, c))| {
                if i > 0 {
                    where_fragment.push_str(" AND ");
                }
                if c.negated {
                    where_fragment.push_str(" NOT ")
                }
                where_fragment.push_str(
                    &(rel_var.name().to_string()
                        + "."
                        + &*k
                        + " "
                        + &*cypher_comparison_operator(&c.operation)
                        + " "
                        + "$"
                        + &*param_var
                        + "."
                        + &*k),
                );
                value_props.insert(k, c.operand);
            });
            params.insert(param_var, value_props.into());
        }

        let qf = QueryFragment::new(match_fragment, where_fragment, params);
        trace!(
            "CypherTransaction::rel_read_fragment returning -- {:#?}",
            qf
        );
        Ok(qf)
    }

    #[tracing::instrument(level = "info", name = "wg-cypher-load-rels", skip(self))]
    async fn load_rels<RequestCtx: RequestContext>(
        &mut self,
        keys: &[RelLoaderKey],
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("CypherTransaction::load_rels called -- keys: {:#?}", keys);

        let mut sg = SuffixGenerator::new();
        let mut query = String::new();
        let mut params = HashMap::new();

        for (i, rlk) in keys.iter().enumerate() {
            let suffix = sg.suffix();
            if i > 0 {
                query.push_str("UNION ALL ");
            }
            query.push_str(&("MATCH (src)-[rel:".to_string() + rlk.rel_name() + "]->(dst)\n"));
            query.push_str(&("WHERE src.id = $id".to_string() + suffix.as_str() + "\n"));
            query.push_str("RETURN src {.id} as src, rel, dst {.id} as dst\n");
            params.insert("id".to_string() + suffix.as_str(), rlk.src_id());
        }

        trace!(
            "GremlinTransaction::load_rels -- query: {}, params: {:#?}",
            query,
            params
        );

        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!("Rel Records: {:#?}", records);

        records
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        name = "wg-cypher-read-rels",
        skip(self, query_fragment, rel_var, options)
    )]
    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        options: Options,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("CypherTransaction::read_rels called -- query_fragment: {:#?}, rel_var: {:#?}, options: {:#?}",
        query_fragment, rel_var, options);

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let mut query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "\n"
            + "RETURN "
            + rel_var.src.name()
            + " {.id} "
            + " as src, "
            + rel_var.name()
            + " as rel, "
            + rel_var.dst.name()
            + " {.id} "
            + " as dst\n";
        query = CypherTransaction::add_sort_to_query(
            query,
            options,
            rel_var.name(),
            Some(rel_var.dst.name()),
        );
        let params = query_fragment.params();

        trace!(
            "CypherTransaction::read_rels -- query: {}, params: {:#?}",
            query,
            params
        );
        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!("Rel Records: {:#?}", records);

        records
            .into_iter()
            .map(|r| r.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        name = "wg-cypher-update-nodes",
        skip(self, query_fragment, node_var, props, options, info, _sg)
    )]
    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        options: Options,
        info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!(
            "CypherTransaction::update_nodes called: query_fragment: {:#?}, node_var: {:#?}, props: {:#?}, info.name: {}",
            query_fragment,
            node_var,
            props,
            info.name()
        );

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let mut query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "SET "
            + node_var.name()
            + " += $props\n"
            + "RETURN "
            + node_var.name()
            + "\n";
        query = CypherTransaction::add_sort_to_query(query, options, node_var.name(), None);
        let mut params = query_fragment.params();
        params.insert("props".to_string(), props.into());

        trace!(
            "CypherTransaction::update_nodes -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        records
            .into_iter()
            .map(|n| n.try_into())
            .collect::<Result<Vec<Node<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        name = "wg-cypher-update-rels",
        skip(self, query_fragment, rel_var, props, options, _sg)
    )]
    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        options: Options,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("CypherTransaction::update_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, options: {:#?}",
        query_fragment, rel_var, props, options);

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let mut query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "SET "
            + rel_var.name()
            + " += $props\n"
            + "RETURN "
            + rel_var.src.name()
            + " {.id} "
            + " as src, "
            + rel_var.name()
            + " as rel, "
            + rel_var.dst.name()
            + " {.id} "
            + " as dst\n";
        query = CypherTransaction::add_sort_to_query(
            query,
            options,
            rel_var.name(),
            Some(rel_var.dst.name()),
        );

        let mut params = query_fragment.params();
        params.insert("props".to_string(), props.into());

        trace!(
            "CypherTransaction::update_rels -- q: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        trace!("Rel Records: {:#?}", records);
        records
            .into_iter()
            .map(|n| n.try_into())
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }

    #[tracing::instrument(
        name = "wg-cypher-delete-nodes",
        skip(self, query_fragment, node_var, _options)
    )]
    async fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        _options: Options,
    ) -> Result<i32, Error> {
        trace!(
            "CypherTransaction::delete_nodes called -- query_fragment: {:#?}, node_var: {:#?}",
            query_fragment,
            node_var
        );

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "DETACH DELETE "
            + node_var.name()
            + "\n"
            + "RETURN count(*) as count\n";
        let params = query_fragment.params();

        trace!(
            "CypherTransaction::delete_nodes -- query: {}, params: {:#?}",
            query,
            params
        );

        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        records
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)?
            .fields()[0]
            .clone()
            .try_into()
            .map_err(|e: ConversionError| e.into())
    }

    #[tracing::instrument(
        name = "wg-cypher-delete-rels",
        skip(self, query_fragment, rel_var, _options)
    )]
    async fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        _options: Options,
    ) -> Result<i32, Error> {
        trace!(
            "CypherTransaction::delete_rels called -- query_fragment: {:#?}, rel_var: {:#?}",
            query_fragment,
            rel_var
        );

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "DELETE "
            + rel_var.name()
            + "\n"
            + "RETURN count(*) as count\n";
        let params = query_fragment.params();

        trace!(
            "CypherTransaction::delete_rels -- query: {}, params: {:#?}",
            query,
            params
        );

        self.client.run(query, Some(params.into()), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1i8)]);
        let (records, response) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::CypherQueryFailed { message }),
        }

        records
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)?
            .fields()[0]
            .clone()
            .try_into()
            .map_err(|e: ConversionError| e.into())
    }

    #[tracing::instrument(name = "wg-cypher-commit-tx", skip(self))]
    async fn commit(&mut self) -> Result<(), Error> {
        debug!("transaction::commit called");
        Ok(self.client.commit().await.map(|_| ())?)
    }

    #[tracing::instrument(name = "wg-cypher-rollback-tx", skip(self))]
    async fn rollback(&mut self) -> Result<(), Error> {
        debug!("transaction::rollback called");
        Ok(self.client.rollback().await.map(|_| ())?)
    }
}

impl TryFrom<bolt_proto::Value> for Value {
    type Error = bolt_proto::error::ConversionError;

    fn try_from(bv: bolt_proto::Value) -> Result<Value, bolt_proto::error::ConversionError> {
        match bv {
            bolt_proto::Value::Boolean(_) => Ok(Value::Bool(bv.try_into()?)),
            bolt_proto::Value::Integer(_) => Ok(Value::Int64(bv.try_into()?)),
            bolt_proto::Value::Float(_) => Ok(Value::Float64(bv.try_into()?)),
            bolt_proto::Value::Bytes(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::List(_) => Ok(Value::Array(bv.try_into()?)),
            bolt_proto::Value::Map(_) => Ok(Value::Map(bv.try_into()?)),
            bolt_proto::Value::Null => Ok(Value::Null),
            bolt_proto::Value::String(_) => Ok(Value::String(bv.try_into()?)),
            bolt_proto::Value::Node(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Relationship(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Path(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::UnboundRelationship(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Date(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Time(_, _) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::DateTimeOffset(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::DateTimeZoned(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::LocalTime(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::LocalDateTime(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Duration(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Point2D(_) => Err(ConversionError::FromValue(bv)),
            bolt_proto::Value::Point3D(_) => Err(ConversionError::FromValue(bv)),
        }
    }
}

impl From<Value> for bolt_proto::Value {
    fn from(v: Value) -> bolt_proto::Value {
        match v {
            Value::Array(a) => a.into(),
            Value::Bool(b) => b.into(),
            Value::Float64(f) => f.into(),
            Value::Int64(i) => i.into(),
            Value::Map(m) => m.into(),
            Value::Null => bolt_proto::Value::Null,
            Value::String(s) => s.into(),
            // This last conversion may be lossy, but interoperability with bolt_proto doesn't
            // allow for a TryFrom conversion here.
            Value::UInt64(u) => (u as i64).into(),
            Value::Uuid(uuid) => uuid.to_hyphenated().to_string().into(),
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<bolt_proto::value::Node> for Node<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: bolt_proto::value::Node) -> Result<Self, Error> {
        let type_name = &value.labels()[0];
        let properties: &HashMap<String, bolt_proto::Value> = value.properties();
        let props_value = Value::try_from(properties.clone())?;
        let props = HashMap::<String, Value>::try_from(props_value)?;
        Ok(Node::new(type_name.to_string(), props))
    }
}

impl<RequestCtx: RequestContext> TryFrom<Record> for Node<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: Record) -> Result<Self, Error> {
        if let bolt_proto::Value::Node(n) = value.fields()[0].clone() {
            n.try_into()
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Node".to_string(),
            })
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<bolt_proto::Value> for NodeRef<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: bolt_proto::Value) -> Result<Self, Error> {
        if let bolt_proto::Value::String(s) = value {
            Ok(NodeRef::Identifier(Value::String(s)))
        } else {
            Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Node".to_string(),
            })
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<Record> for Rel<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: Record) -> Result<Self, Error> {
        trace!("Record is: {:#?}", value);
        match (
            value.fields()[0].clone(),
            value.fields()[1].clone(),
            value.fields()[2].clone(),
        ) {
            (
                bolt_proto::Value::Map(src_id_map),
                bolt_proto::Value::Relationship(rel),
                bolt_proto::Value::Map(dst_id_map),
            ) => {
                let src_node_ref = src_id_map
                    .get("id")
                    .ok_or(Error::ResponseItemNotFound {
                        name: "id".to_string(),
                    })?
                    .clone()
                    .try_into()?;
                let dst_node_ref = dst_id_map
                    .get("id")
                    .ok_or(Error::ResponseItemNotFound {
                        name: "id".to_string(),
                    })?
                    .clone()
                    .try_into()?;
                let rel_name = rel.rel_type();
                let properties: &HashMap<String, bolt_proto::Value> = rel.properties();
                let props_value = Value::try_from(properties.clone())?;
                let props = HashMap::<String, Value>::try_from(props_value)?;
                Ok(Rel::new(
                    rel_name.to_string(),
                    props,
                    src_node_ref,
                    dst_node_ref,
                ))
            }
            (_, _, _) => Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Node".to_string(),
            }),
        }
    }
}

impl TryFrom<HashMap<String, bolt_proto::Value>> for Value {
    type Error = Error;

    fn try_from(hm: HashMap<String, bolt_proto::Value>) -> Result<Value, Error> {
        let hmv: HashMap<String, Value> = hm.into_iter().try_fold(
            HashMap::new(),
            |mut acc, (key, bolt_value)| -> Result<HashMap<String, Value>, Error> {
                let value = Value::try_from(bolt_value)?;
                acc.insert(key, value);
                Ok(acc)
            },
        )?;
        Ok(Value::Map(hmv))
    }
}

fn cypher_comparison_operator(op: &Operation) -> String {
    match op {
        Operation::EQ => "=".to_string(),
        Operation::CONTAINS => "CONTAINS".to_string(),
        Operation::IN => "IN".to_string(),
        Operation::GT => ">".to_string(),
        Operation::GTE => ">=".to_string(),
        Operation::LT => "<".to_string(),
        Operation::LTE => "<=".to_string(),
    }
}
