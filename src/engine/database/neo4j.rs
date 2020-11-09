//! Provides database interface types and functions for Neo4J databases.

use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::database::{
    env_string, env_u16, ClauseType, DatabaseEndpoint, DatabasePool, NodeQueryVar, RelQueryVar,
    SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::Info;
use crate::engine::schema::NodeType;
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use bb8::Pool;
use bb8::PooledConnection;
use bb8_bolt::BoltConnectionManager;
use bolt_client::{Metadata, Params};
use bolt_proto::error::ConversionError;
use bolt_proto::message::{Message, Record};
use log::{debug, trace};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;
use tokio::runtime::Runtime;

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
    pub fn from_env() -> Result<Neo4jEndpoint, Error> {
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
    async fn pool(&self) -> Result<DatabasePool, Error> {
        let manager = BoltConnectionManager::new(
            self.host.to_string() + ":" + &self.port.to_string(),
            None,
            [4, 0, 0, 0],
            HashMap::from_iter(vec![
                ("user_agent", "warpgrapher/0.2.0"),
                ("scheme", "basic"),
                ("principal", &self.user),
                ("credentials", &self.pass),
            ]),
        )?;

        let pool = DatabasePool::Neo4j(
            Pool::builder()
                .max_size(num_cpus::get().try_into().unwrap_or(8))
                .build(manager)
                .await?,
        );

        trace!("Neo4jEndpoint::pool -- pool: {:#?}", pool);
        Ok(pool)
    }
}

#[derive(Debug)]
pub(crate) struct Neo4jTransaction<'t> {
    client: PooledConnection<'t, BoltConnectionManager>,
    runtime: &'t mut Runtime,
}

impl<'t> Neo4jTransaction<'t> {
    pub fn new(
        client: PooledConnection<'t, BoltConnectionManager>,
        runtime: &'t mut Runtime,
    ) -> Neo4jTransaction<'t> {
        Neo4jTransaction { client, runtime }
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
            .ok_or_else(|| Error::ResponseSetNotFound)
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

    fn nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        records: Vec<Record>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
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

    fn rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        records: Vec<Record>,
        partition_key_opt: Option<&Value>,
        props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
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
            .collect::<Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error>>()
    }
}

impl Transaction for Neo4jTransaction<'_> {
    fn begin(&mut self) -> Result<(), Error> {
        debug!("Neo4jTransaction::begin called");

        let response = self.runtime.block_on(self.client.begin(None));
        match response {
            Ok(Message::Success(_)) => Ok(()),
            Ok(message) => Err(Error::Neo4jQueryFailed { message }),
            Err(e) => Err(Error::from(e)),
        }
    }

    fn node_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        rel_create_fragments: Vec<String>,
        mut params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_create_query called -- rel_create_fragments: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}, clause: {:#?}", 
        rel_create_fragments, params, node_var, props, clause);
        let props_suffix = sg.suffix();
        let mut query = "CREATE (".to_string()
            + node_var.name()
            + ":"
            + node_var.label()?
            + " { id: randomUUID() })\n"
            + "SET "
            + node_var.name()
            + " += $props"
            + &props_suffix
            + "\n";

        if !rel_create_fragments.is_empty() {
            query.push_str(&("WITH ".to_string() + node_var.name() + "\n"));
            query.push_str(&rel_create_fragments.into_iter().fold(
                String::new(),
                |mut acc, fragment| {
                    acc.push_str(
                        &("CALL {\nWITH ".to_string() + node_var.name() + "\n" + &fragment + "}\n"),
                    );
                    acc
                },
            ));
        }

        match clause {
            ClauseType::Parameter => (),
            ClauseType::FirstSubQuery | ClauseType::SubQuery | ClauseType::Query => {
                query.push_str(&("RETURN ".to_string() + node_var.name() + "\n"));
            }
        };

        params.insert("props".to_string() + &props_suffix, props.into());

        Ok((query, params))
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        trace!(
            "Neo4jTransaction::create_node called -- query: {}, params: {:#?}",
            query,
            params
        );
        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::ResponseSetNotFound)
    }

    fn rel_create_fragment<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        dst_query: &str,
        mut params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_create_fragment called -- dst_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}, clause: {:#?}", 
        dst_query, params, rel_var, props, clause);

        let props_var = "props".to_string() + &sg.suffix();

        let q = "CALL {\n".to_string()
            + "WITH "
            + rel_var.src().name()
            + "\n"
            + dst_query
            + "}\n"
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
            + " += $"
            + &props_var
            + "\n";

        params.insert(props_var, props.into());

        let query = match clause {
            ClauseType::Parameter => q,
            ClauseType::FirstSubQuery | ClauseType::SubQuery => {
                q + "RETURN " + rel_var.name() + "\n"
            }
            ClauseType::Query => Neo4jTransaction::add_rel_return(
                q,
                rel_var.src().name(),
                rel_var.name(),
                rel_var.dst().name(),
            ),
        };

        Ok((query, params))
    }

    fn rel_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query_opt: Option<String>,
        rel_create_fragments: Vec<String>,
        params: HashMap<String, Value>,
        rel_vars: Vec<RelQueryVar>,
        _clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_create_query called -- src_query_opt: {:#?}, rel_create_fragments: {:#?}, params: {:#?}, rel_vars: {:#?}",
        src_query_opt, rel_create_fragments, params, rel_vars);

        let src_var = rel_vars.get(0).ok_or_else(|| Error::LabelNotFound)?.src();
        let mut query = src_query_opt.unwrap_or_else(String::new);

        rel_create_fragments.iter().for_each(|rcf| {
            query.push_str(&("CALL {\nWITH ".to_string() + src_var.name() + "\n" + &rcf + "}\n"));
        });

        query.push_str(&("WITH collect(".to_string() + src_var.name() + ".id) AS src_ids"));
        rel_vars.iter().for_each(|rel_var| {
            query.push_str(
                &(", collect(".to_string() + rel_var.name() + ".id) AS " + rel_var.name() + "_ids"),
            );
        });
        query.push_str("\n");

        query.push_str(&("MATCH (src:".to_string() + src_var.label()? + ")-[rel]->(dst)\n"));
        query.push_str("WHERE src.id IN src_ids AND rel.id IN (");
        rel_vars.iter().enumerate().for_each(|(i, rel_var)| {
            if i > 0 {
                query.push_str(" + ");
            }
            query.push_str(&(rel_var.name().to_string() + "_ids"));
        });
        query.push_str(")\n");

        Ok((
            Neo4jTransaction::add_rel_return(query, "src", "rel", "dst"),
            params,
        ))
    }

    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::create_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query, params, props_type_name, partition_key_opt);

        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
    }

    fn node_read_fragment(
        &mut self,
        rel_query_fragments: Vec<(String, String)>,
        mut params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_read_fragment called -- rel_query_fragment: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}, clause: {:#?}",
        rel_query_fragments, params, node_var, props, clause);

        let param_suffix = sg.suffix();
        let mut match_fragment = String::new();
        let mut where_fragment = String::new();

        if rel_query_fragments.is_empty() {
            match clause {
                ClauseType::Parameter => (),
                ClauseType::FirstSubQuery | ClauseType::SubQuery | ClauseType::Query => {
                    if node_var.label().is_ok() {
                        match_fragment.push_str(
                            &("MATCH (".to_string()
                                + node_var.name()
                                + ":"
                                + node_var.label()?
                                + ")\n"),
                        );
                    } else {
                        match_fragment.push_str(&("MATCH (".to_string() + node_var.name() + ")\n"));
                    }
                }
            }
        }

        if !props.is_empty() {
            props.keys().enumerate().for_each(|(i, k)| {
                if i > 0 {
                    where_fragment.push_str(" AND ");
                }

                where_fragment.push_str(
                    &(node_var.name().to_string()
                        + "."
                        + &k
                        + "=$"
                        + "param"
                        + &param_suffix
                        + "."
                        + &k),
                );
            });
        }
        params.insert("param".to_string() + &param_suffix, props.into());

        rel_query_fragments.iter().for_each(|rqf| {
            match_fragment.push_str(&rqf.0);
            if !where_fragment.is_empty() {
                where_fragment.push_str(" AND ");
            }
            where_fragment.push_str(&rqf.1);
        });

        Ok((match_fragment, where_fragment, params))
    }

    fn node_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, clause: {:#?}",
        match_fragment, where_fragment, params, clause);
        let mut query = match_fragment.to_string();

        if !where_fragment.is_empty() {
            query.push_str(&("WHERE ".to_string() + where_fragment + "\n"));
        }

        match clause {
            ClauseType::Parameter => (),
            ClauseType::FirstSubQuery | ClauseType::SubQuery | ClauseType::Query => {
                query.push_str(&("RETURN ".to_string() + node_var.name() + "\n"));
            }
        };

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!(
            "Neo4jTransaction::read_nodes called -- query: {}, params_opt: {:#?}, info.name: {}",
            query,
            params_opt,
            info.name()
        );
        self.runtime.block_on(self.client.run_with_metadata(
            query,
            params_opt.map(Params::from),
            None,
        ))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)
    }

    fn rel_read_fragment(
        &mut self,
        src_query_opt: Option<(String, String)>,
        dst_query_opt: Option<(String, String)>,
        mut params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_read_fragment called -- src_query_opt: {:#?}, dst_query_opt: {:#?}, params: {:#?}, rel_var: {:#?}, props: {:#?}",
        src_query_opt, dst_query_opt, params, rel_var, props);

        let mut match_fragment = String::new();
        let mut where_fragment = String::new();

        if let Some(src_query) = src_query_opt {
            match_fragment.push_str(&src_query.0);
            where_fragment.push_str(&src_query.1);

            if dst_query_opt.is_some() {
                where_fragment.push_str(" AND ");
            }
        }

        if let Some(dst_query) = dst_query_opt {
            match_fragment.push_str(&dst_query.0);
            where_fragment.push_str(&dst_query.1);
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

        let param_var = "param".to_string() + &sg.suffix();
        if !props.is_empty() {
            props.keys().enumerate().for_each(|(i, k)| {
                if i >= 1 {
                    where_fragment.push_str(" AND ");
                }

                where_fragment.push_str(
                    &(rel_var.name().to_string() + "." + &k + " = $" + &param_var + "." + &k),
                );
            });

            params.insert(param_var, props.into());
        }

        Ok((match_fragment, where_fragment, params))
    }

    fn rel_read_query(
        &mut self,
        match_fragment: &str,
        where_fragment: &str,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        clause: ClauseType,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_read_query called -- match_fragment: {}, where_fragment: {}, params: {:#?}, rel_var: {:#?}, clause: {:#?}",
        match_fragment, where_fragment, params, rel_var, clause);

        let mut query = match_fragment.to_string();

        if !where_fragment.is_empty() {
            query.push_str(&("WHERE ".to_string() + where_fragment + "\n"));
        }

        if let ClauseType::Query = clause {
            Ok((
                Neo4jTransaction::add_rel_return(
                    query,
                    rel_var.src().name(),
                    rel_var.name(),
                    rel_var.dst().name(),
                ),
                params,
            ))
        } else {
            Ok((query, params))
        }
    }

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params_opt: Option<HashMap<String, Value>>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::read_rels called -- query: {}, props_type_name: {:#?}, partition_key_opt: {:#?}, params_opt: {:#?}",
        query, props_type_name, partition_key_opt, params_opt);

        self.runtime.block_on(self.client.run_with_metadata(
            query,
            params_opt.map(Params::from),
            None,
        ))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
    }

    fn node_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        change_queries: Vec<String>,
        mut params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        props: HashMap<String, Value>,
        _clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_update_query called: match_query: {}, change_queries: {:#?}, params: {:#?}, node_var: {:#?}, props: {:#?}",
        match_query, change_queries, params, node_var, props);

        let props_suffix = sg.suffix();
        let mut query = match_query;

        query.push_str(
            &("SET ".to_string() + node_var.name() + " += $props" + &props_suffix + "\n"),
        );
        params.insert("props".to_string() + &props_suffix, props.into());

        if !change_queries.is_empty() {
            query.push_str(&("WITH ".to_string() + node_var.name() + "\n"));
            for cq in change_queries.iter() {
                query
                    .push_str(&("CALL {\nWITH ".to_string() + node_var.name() + "\n" + cq + "}\n"));
            }
        }
        query.push_str(&("RETURN ".to_string() + node_var.name() + "\n"));

        Ok((query, params))
    }

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!(
            "Neo4jTransaction::update_nodes called: query: {}, params: {:#?}",
            query,
            params,
        );
        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::nodes(records, info)
    }

    fn rel_update_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        match_query: String,
        mut params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        props: HashMap<String, Value>,
        _clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_update_query called -- match_query: {}, params: {:#?}, rel_var: {:#?}, props: {:#?}",
        match_query, params, rel_var, props);

        let props_var = "props".to_string() + &sg.suffix();

        let q = match_query + "SET " + rel_var.name() + " += $" + &props_var + "\n";

        params.insert(props_var, props.into());
        Ok((
            Neo4jTransaction::add_rel_return(
                q,
                rel_var.src().name(),
                rel_var.name(),
                rel_var.dst().name(),
            ),
            params,
        ))
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::update_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}",
        query, params, props_type_name, partition_key_opt);
        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::rels(records, partition_key_opt, props_type_name)
    }

    fn node_delete_query(
        &mut self,
        match_query: String,
        rel_delete_fragments: Vec<String>,
        params: HashMap<String, Value>,
        node_var: &NodeQueryVar,
        _clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4JTransaction::node_delete_query -- match_query: {}, rel_delete_fragments: {:#?}, params: {:#?}, node_var: {:#?}",
        match_query, rel_delete_fragments, params, node_var);

        let mut query = match_query;

        rel_delete_fragments.iter().for_each(|rdf| {
            query.push_str(&("CALL {\nWITH ".to_string() + node_var.name() + "\n" + rdf + "}\n"));
        });

        let query = query
            + "DETACH DELETE "
            + node_var.name()
            + "\n"
            + "RETURN count(*) as count"
            + &sg.suffix()
            + "\n";

        Ok((query, params))
    }

    fn delete_nodes(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::delete_nodes called -- query: {}, params: {:#?}",
            query,
            params,
        );
        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::extract_count(records)
    }

    fn rel_delete_query(
        &mut self,
        query: String,
        src_delete_query_opt: Option<String>,
        dst_delete_query_opt: Option<String>,
        params: HashMap<String, Value>,
        rel_var: &RelQueryVar,
        _clause: ClauseType,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_delete_query called -- query: {}, src_delete_query_opt: {:#?}, dst_delete_query_opt: {:#?}, params: {:#?}, rel_var: {:#?}",
        query, src_delete_query_opt, dst_delete_query_opt, params, rel_var);
        let mut q = query;

        if let Some(sdq) = src_delete_query_opt {
            q.push_str(&("CALL {\n".to_string() + &sdq + "}\n"));
        }

        if let Some(ddq) = dst_delete_query_opt {
            q.push_str(&("CALL {\n".to_string() + &ddq + "}\n"));
        }

        q.push_str(
            &("DELETE ".to_string()
                + rel_var.name()
                + "\n"
                + "RETURN count(*) as count"
                + &sg.suffix()
                + "\n"),
        );

        Ok((q, params))
    }

    fn delete_rels(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::delete_rels called -- query: {}, params: {:#?}",
            query,
            params
        );
        let p = Params::from(params);
        self.runtime
            .block_on(self.client.run_with_metadata(query, Some(p), None))?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = self.runtime.block_on(self.client.pull(Some(pull_meta)))?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        Neo4jTransaction::extract_count(records)
    }

    fn commit(&mut self) -> Result<(), Error> {
        debug!("transaction::commit called");

        Ok(self.runtime.block_on(self.client.commit()).map(|_| ())?)
    }

    fn rollback(&mut self) -> Result<(), Error> {
        debug!("transaction::rollback called");
        Ok(self.runtime.block_on(self.client.rollback()).map(|_| ())?)
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

impl<GlobalCtx, RequestCtx> TryFrom<bolt_proto::value::Value> for Node<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
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
