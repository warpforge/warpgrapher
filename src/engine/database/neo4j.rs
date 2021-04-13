//! Provides database interface types and functions for Neo4J databases.

use crate::engine::context::RequestContext;
use crate::engine::database::{
    env_string, env_u16, Comparison, DatabaseClient, DatabaseEndpoint, DatabasePool, NodeQueryVar,
    Operation, QueryFragment, RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::Info;
use crate::engine::schema::NodeType;
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use bolt_client::{Metadata, Params};
use bolt_proto::error::ConversionError;
use bolt_proto::message::{Message, Record};
use log::{debug, trace};
use mobc::{Connection, Pool};
use mobc_boltrs::BoltConnectionManager;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;

/// A Neo4J endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ne = Neo4jEndpoint::new(
///         "127.0.0.1".to_string(),
///         7687,
///         "neo4j".to_string(),
///         "password".to_string()
///     );
/// #    Ok(())
/// # }
/// ```
pub struct Neo4jEndpoint {
    host: String,
    port: u16,
    user: String,
    pass: String,
}

impl Neo4jEndpoint {
    /// Returns a new [`Neo4jEndpoint`] from the provided values.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ne = Neo4jEndpoint::new(
    ///         "127.0.0.1".to_string(),
    ///         7687,
    ///         "neo4j".to_string(),
    ///         "password".to_string()
    ///     );
    /// #    Ok(())
    /// # }
    /// ```
    pub fn new(host: String, port: u16, user: String, pass: String) -> Self {
        Neo4jEndpoint {
            host,
            port,
            user,
            pass,
        }
    }

    /// Reads an variable to construct a [`Neo4jEndpoint`]. The environment variable is
    ///
    /// * WG_NEO4J_ADDR - the address for the Neo4J DB. For example, `127.0.0.1`.
    /// * WG_NEO4J_PORT - the port number for the Neo4J DB.  For example, `7687`.
    /// * WG_NEO4J_USER - the username for the Neo4J DB. For example, `neo4j`.
    /// * WG_NEO4J_PASS - the password for the Neo4J DB. For example, `my-db-pass`.
    ///
    /// [`Neo4jEndpoint`]: ./struct.Neo4jEndpoint.html
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
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ne = Neo4jEndpoint::from_env()?;
    ///     # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> Result<Self, Error> {
        Ok(Neo4jEndpoint {
            host: env_string("WG_NEO4J_HOST")?,
            port: env_u16("WG_NEO4J_PORT")?,
            user: env_string("WG_NEO4J_USER")?,
            pass: env_string("WG_NEO4J_PASS")?,
        })
    }
}

#[async_trait]
impl DatabaseEndpoint for Neo4jEndpoint {
    type PoolType = Neo4jDatabasePool;

    async fn pool(&self) -> Result<Self::PoolType, Error> {
        let manager = BoltConnectionManager::new(
            self.host.to_string() + ":" + &*self.port.to_string(),
            None,
            [4, 0, 0, 0],
            HashMap::from_iter(vec![
                ("user_agent", "warpgrapher/0.2.0"),
                ("scheme", "basic"),
                ("principal", &self.user),
                ("credentials", &self.pass),
            ]),
        )
        .await?;

        let pool = Neo4jDatabasePool::new(
            Pool::builder()
                .max_open(num_cpus::get().try_into().unwrap_or(8))
                .build(manager),
        );

        Ok(pool)
    }
}

#[derive(Clone)]
pub struct Neo4jDatabasePool {
    pool: Pool<BoltConnectionManager>,
}

impl Neo4jDatabasePool {
    fn new(pool: Pool<BoltConnectionManager>) -> Self {
        Neo4jDatabasePool { pool }
    }
}

#[async_trait]
impl DatabasePool for Neo4jDatabasePool {
    type TransactionType = Neo4jTransaction;

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(Neo4jTransaction::new(self.pool.get().await?))
    }

    async fn client(&self) -> Result<DatabaseClient, Error> {
        Ok(DatabaseClient::Neo4j(Box::new(self.pool.get().await?)))
    }
}

pub struct Neo4jTransaction {
    client: Connection<BoltConnectionManager>,
}

impl Neo4jTransaction {
    pub fn new(client: Connection<BoltConnectionManager>) -> Neo4jTransaction {
        Neo4jTransaction { client }
    }

    fn add_rel_return(query: String, src_var: &str, rel_var: &str, dst_var: &str) -> String {
        query
            + "RETURN "
            + src_var
            + ".id as src_id, labels("
            + src_var
            + ") as src_labels, "
            + rel_var
            + " as rel, "
            + dst_var
            + ".id as dst_id, labels("
            + dst_var
            + ") as dst_labels\n"
    }

    fn extract_count(records: Vec<Record>) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::extract_count called -- records: {:#?}",
            records
        );

        records
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)
            .and_then(|r| r.fields()[0].clone().try_into().map_err(Error::from))
    }

    fn extract_node_properties(
        props: HashMap<String, bolt_proto::value::Value>,
        type_def: &NodeType,
    ) -> Result<HashMap<String, Value>, Error> {
        trace!("Neo4jTransaction::extract_node_properties called");

        props
            .into_iter()
            .map(|(k, v)| {
                if type_def.property(&k)?.list() {
                    if let bolt_proto::value::Value::List(_) = v {
                        Ok((k, v.try_into()?))
                    } else {
                        Ok((k, Value::Array(vec![(v.try_into()?)])))
                    }
                } else {
                    Ok((k, v.try_into()?))
                }
            })
            .collect::<Result<HashMap<String, Value>, Error>>()
    }

    fn nodes<RequestCtx: RequestContext>(
        records: Vec<Record>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!("Neo4jTransaction::nodes called -- records: {:#?}", records);

        records
            .into_iter()
            .map(|r| {
                if let bolt_proto::value::Value::Node(n) = &r.fields()[0] {
                    Ok(Node::new(
                        n.labels()[0].to_string(),
                        Neo4jTransaction::extract_node_properties(
                            n.properties().clone(),
                            info.type_def_by_name(&n.labels()[0])?,
                        )?,
                    ))
                } else {
                    Err(Error::ResponseItemNotFound {
                        name: "node".to_string(),
                    })
                }
            })
            .collect()
    }

    fn rels<RequestCtx: RequestContext>(
        records: Vec<Record>,
        partition_key_opt: Option<&Value>,
        props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("Neo4jTransaction::rels called -- records: {:#?}", records);

        records
            .into_iter()
            .map(|r| {
                let src_id = r.fields()[0].clone().try_into()?;
                let src_label = TryInto::<Vec<String>>::try_into(r.fields()[1].clone())?
                    .pop()
                    .ok_or_else(|| Error::ResponseItemNotFound {
                        name: "src_labels".to_string(),
                    })?;
                let dst_id = r.fields()[3].clone().try_into()?;
                let dst_label = TryInto::<Vec<String>>::try_into(r.fields()[4].clone())?
                    .pop()
                    .ok_or_else(|| Error::ResponseItemNotFound {
                        name: "dst_labels".to_string(),
                    })?;
                let mut props =
                    if let bolt_proto::value::Value::Relationship(rel) = r.fields()[2].clone() {
                        rel.properties()
                            .iter()
                            .map(|(k, v)| Ok((k.to_string(), v.clone().try_into()?)))
                            .collect::<Result<HashMap<String, Value>, bolt_proto::error::Error>>()?
                    } else {
                        return Err(Error::ResponseItemNotFound {
                            name: "rel".to_string(),
                        });
                    };

                Ok(Rel::new(
                    props
                        .remove("id")
                        .ok_or_else(|| Error::ResponseItemNotFound {
                            name: "id".to_string(),
                        })?,
                    partition_key_opt.cloned(),
                    props_type_name.map(|ptn| Node::new(ptn.to_string(), props)),
                    NodeRef::Identifier {
                        id: src_id,
                        label: src_label,
                    },
                    NodeRef::Identifier {
                        id: dst_id,
                        label: dst_label,
                    },
                ))
            })
            .collect::<Result<Vec<Rel<RequestCtx>>, Error>>()
    }
}

#[async_trait]
impl Transaction for Neo4jTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        debug!("Neo4jTransaction::begin called");

        let response = self.client.begin(None).await;
        match response {
            Ok(Message::Success(_)) => Ok(()),
            Ok(message) => Err(Error::Neo4jQueryFailed { message }),
            Err(e) => Err(Error::from(e)),
        }
    }

    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error> {
        trace!(
            "Neo4jTransaction::create_node called -- node_var: {:#?}, props: {:#?}",
            node_var,
            props
        );

        let query = "CREATE (n:".to_string()
            + node_var.label()?
            + " { id: randomUUID() })\n"
            + "SET n += $props\n"
            + "RETURN n\n";

        let mut params: HashMap<&str, Value> = HashMap::new();
        params.insert("props", props.into());

        trace!(
            "Neo4jTransaction::create_node -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run_with_metadata(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)?
            .into_iter()
            .next()
            .ok_or(Error::ResponseSetNotFound)
    }

    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        src_fragment: QueryFragment,
        dst_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("Neo4jTransaction::create_rels called -- src_query: {:#?}, dst_query: {:#?}, rel_var: {:#?}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        src_fragment, dst_fragment, rel_var, props, props_type_name, partition_key_opt);

        let query = src_fragment.match_fragment().to_string()
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
            + " { id: randomUUID() }]->("
            + rel_var.dst().name()
            + ")\n"
            + "SET "
            + rel_var.name()
            + " += $props\n";

        let q = Neo4jTransaction::add_rel_return(
            query,
            rel_var.src().name(),
            rel_var.name(),
            rel_var.dst().name(),
        );

        let mut params = src_fragment.params();
        params.extend(dst_fragment.params());
        params.insert("props".to_string(), props.into());

        trace!(
            "Neo4jTransaction::create_rels -- q: {}, params: {:#?}",
            q,
            params
        );

        let p = Params::from(params);
        self.client.run_with_metadata(q, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
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
        trace!("Neo4jTransaction::node_read_fragment called -- rel_query_fragment: {:#?}, node_var: {:#?}, props: {:#?}, sg: {:#?}",
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
                        + &*neo4j_comparison_operator(&c.operation)
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
        trace!("Neo4jTransaction::node_read_fragment returning {:#?}", qf);

        Ok(qf)
    }

    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        node_var: &NodeQueryVar,
        query_fragment: QueryFragment,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!(
            "Neo4jTransaction::read_nodes called -- node_var: {:#?}, query_fragment: {:#?}, info.name: {}",
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

        let query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "RETURN "
            + "DISTINCT "
            + node_var.name()
            + "\n";
        let params = query_fragment.params();

        trace!(
            "Neo4jTransaction::read_nodes -- query: {}, params: {:#?}",
            query,
            params
        );
        self.client
            .run_with_metadata(query, Some(params.into()), None)
            .await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)
    }

    fn rel_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        rel_var: &RelQueryVar,
        rels: &[Rel<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        trace!(
            "Neo4jTransaction::rel_read_by_ids_query called -- rel_var: {:#?}, rels: {:#?}",
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
            .collect::<Vec<&Value>>()
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
        trace!("Neo4jTransaction::rel_read_fragment called -- src_fragment_opt: {:#?}, dst_fragment_opt: {:#?}, rel_var: {:#?}, props: {:#?}",
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
                        + &*neo4j_comparison_operator(&c.operation)
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
        trace!("Neo4jTransaction::rel_read_fragment returning -- {:#?}", qf);
        Ok(qf)
    }

    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("Neo4jTransaction::read_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, props_type_name, partition_key_opt);

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let mut query = query_fragment.match_fragment().to_string() + &*where_clause + "\n";
        query = Neo4jTransaction::add_rel_return(
            query,
            rel_var.src().name(),
            rel_var.name(),
            rel_var.dst().name(),
        );
        let params = query_fragment.params();

        trace!(
            "Neo4jTransaction::read_rels -- query: {}, params: {:#?}",
            query,
            params
        );
        self.client
            .run_with_metadata(query, Some(params.into()), None)
            .await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
    }

    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        trace!(
            "Neo4jTransaction::update_nodes called: query_fragment: {:#?}, node_var: {:#?}, props: {:#?}, info.name: {}",
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

        let query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "SET "
            + node_var.name()
            + " += $props\n"
            + "RETURN "
            + node_var.name()
            + "\n";
        let mut params = query_fragment.params();
        params.insert("props".to_string(), props.into());

        trace!(
            "Neo4jTransaction::update_nodes -- query: {}, params: {:#?}",
            query,
            params
        );

        let p = Params::from(params);
        self.client.run_with_metadata(query, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)
    }

    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        trace!("Neo4jTransaction::update_rels called -- query_fragment: {:#?}, rel_var: {:#?}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query_fragment, rel_var, props, props_type_name, partition_key_opt);

        let where_fragment = query_fragment.where_fragment().to_string();
        let where_clause = if !where_fragment.is_empty() {
            "WHERE ".to_string() + &*where_fragment + "\n"
        } else {
            String::new()
        };

        let query = query_fragment.match_fragment().to_string()
            + &*where_clause
            + "SET "
            + rel_var.name()
            + " += $props\n";

        let mut params = query_fragment.params();
        params.insert("props".to_string(), props.into());

        let q = Neo4jTransaction::add_rel_return(
            query,
            rel_var.src().name(),
            rel_var.name(),
            rel_var.dst().name(),
        );

        trace!(
            "Neo4jTransaction::update_rels -- q: {}, params: {:#?}",
            q,
            params
        );

        let p = Params::from(params);
        self.client.run_with_metadata(q, Some(p), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
    }

    async fn delete_nodes(
        &mut self,
        query_fragment: QueryFragment,
        node_var: &NodeQueryVar,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::delete_nodes called -- query_fragment: {:#?}, node_var: {:#?}",
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
            "Neo4jTransaction::delete_nodes -- query: {}, params: {:#?}",
            query,
            params
        );

        self.client
            .run_with_metadata(query, Some(params.into()), None)
            .await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::extract_count(records)
    }

    async fn delete_rels(
        &mut self,
        query_fragment: QueryFragment,
        rel_var: &RelQueryVar,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::delete_rels called -- query_fragment: {:#?}, rel_var: {:#?}",
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
            "Neo4jTransaction::delete_rels -- query: {}, params: {:#?}",
            query,
            params
        );

        self.client
            .run_with_metadata(query, Some(params.into()), None)
            .await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::extract_count(records)
    }

    async fn commit(&mut self) -> Result<(), Error> {
        debug!("transaction::commit called");
        Ok(self.client.commit().await.map(|_| ())?)
    }

    async fn rollback(&mut self) -> Result<(), Error> {
        debug!("transaction::rollback called");
        Ok(self.client.rollback().await.map(|_| ())?)
    }
}

impl TryFrom<bolt_proto::value::Value> for Value {
    type Error = bolt_proto::error::Error;

    fn try_from(bv: bolt_proto::value::Value) -> Result<Value, bolt_proto::error::Error> {
        match bv {
            bolt_proto::value::Value::Boolean(_) => Ok(Value::Bool(bv.try_into()?)),
            bolt_proto::value::Value::Integer(_) => Ok(Value::Int64(bv.try_into()?)),
            bolt_proto::value::Value::Float(_) => Ok(Value::Float64(bv.try_into()?)),
            bolt_proto::value::Value::Bytes(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::List(_) => Ok(Value::Array(bv.try_into()?)),
            bolt_proto::value::Value::Map(_) => Ok(Value::Map(bv.try_into()?)),
            bolt_proto::value::Value::Null => Ok(Value::Null),
            bolt_proto::value::Value::String(_) => Ok(Value::String(bv.try_into()?)),
            bolt_proto::value::Value::Node(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::Relationship(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::Path(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::UnboundRelationship(_) => {
                Err(ConversionError::FromValue(bv).into())
            }
            bolt_proto::value::Value::Date(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::Time(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::DateTimeOffset(_) => {
                Err(ConversionError::FromValue(bv).into())
            }
            bolt_proto::value::Value::DateTimeZoned(_) => {
                Err(ConversionError::FromValue(bv).into())
            }
            bolt_proto::value::Value::LocalTime(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::LocalDateTime(_) => {
                Err(ConversionError::FromValue(bv).into())
            }
            bolt_proto::value::Value::Duration(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::Point2D(_) => Err(ConversionError::FromValue(bv).into()),
            bolt_proto::value::Value::Point3D(_) => Err(ConversionError::FromValue(bv).into()),
        }
    }
}

impl From<Value> for bolt_proto::value::Value {
    fn from(v: Value) -> bolt_proto::value::Value {
        match v {
            Value::Array(a) => a.into(),
            Value::Bool(b) => b.into(),
            Value::Float64(f) => f.into(),
            Value::Int64(i) => i.into(),
            Value::Map(m) => m.into(),
            Value::Null => bolt_proto::value::Value::Null,
            Value::String(s) => s.into(),
            // This last conversion may be lossy, but interoperability with bolt_proto doesn't
            // allow for a TryFrom conversion here.
            Value::UInt64(u) => (u as i64).into(),
            Value::Uuid(uuid) => uuid.to_hyphenated().to_string().into(),
        }
    }
}

impl<RequestCtx: RequestContext> TryFrom<bolt_proto::value::Value> for Node<RequestCtx> {
    type Error = crate::Error;

    fn try_from(value: bolt_proto::value::Value) -> Result<Self, Error> {
        match value {
            bolt_proto::value::Value::Node(n) => {
                let type_name = &n.labels()[0];
                let properties: &HashMap<String, bolt_proto::value::Value> = &n.properties();
                let props_value = Value::try_from(properties.clone())?;
                let props = HashMap::<String, Value>::try_from(props_value)?;
                Ok(Node::new(type_name.to_string(), props))
            }
            _ => Err(Error::TypeConversionFailed {
                src: format!("{:#?}", value),
                dst: "Node".to_string(),
            }),
        }
    }
}

impl TryFrom<HashMap<String, bolt_proto::value::Value>> for Value {
    type Error = Error;

    fn try_from(hm: HashMap<String, bolt_proto::value::Value>) -> Result<Value, Error> {
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

fn neo4j_comparison_operator(op: &Operation) -> String {
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
