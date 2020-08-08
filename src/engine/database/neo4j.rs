//! Provides database interface types and functions for Neo4J databases.

use super::{env_string, env_u16, DatabaseEndpoint, DatabasePool, SuffixGenerator, Transaction};
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, NodeRef, Rel};
use crate::engine::schema::Info;
use crate::engine::schema::NodeType;
use crate::engine::value::Value;
use crate::Error;
use async_trait::async_trait;
use bb8::Pool;
use bb8::PooledConnection;
use bb8_bolt::BoltConnectionManager;
use bolt_client::{Client, Metadata, Params};
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

    fn add_node_return(mut query: String, node_var: &str, return_var: &str) -> String {
        query.push_str(&("RETURN ".to_string() + node_var + " as " + return_var + "\n"));
        query
    }

    fn add_rel_return(mut query: String, src_var: &str, rel_var: &str, dst_var: &str) -> String {
        query.push_str(
            &("RETURN ".to_string()
                + src_var
                + ".id as src_id, labels("
                + src_var
                + ") as src_labels, "
                + rel_var
                + " as rel, "
                + dst_var
                + ".id as dst_id, labels("
                + dst_var
                + ") as dst_labels\n"),
        );
        query
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

    #[allow(dead_code)]
    async fn single_rel_check<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        client: &mut Client,
        src_label: &str,
        src_ids: Vec<Value>,
        dst_ids: Vec<Value>,
        rel_name: &str,
        _partition_key_opt: Option<&Value>,
    ) -> Result<(), Error> {
        let query = "MATCH (src:".to_string()
            + src_label
            + ")-[rel:"
            + rel_name
            + "]->() WHERE src.id IN $src_ids RETURN COUNT(rel) as count";

        let mut hm: HashMap<String, Value> = HashMap::new();
        hm.insert("src_ids".to_string(), src_ids.into());
        let params = Params::from(hm);
        client.run_with_metadata(query, Some(params), None).await?;

        let pull_meta = Metadata::from_iter(vec![("n", -1)]);
        let (response, records) = client.pull(Some(pull_meta)).await?;
        match response {
            Message::Success(_) => (),
            message => return Err(Error::Neo4jQueryFailed { message }),
        }

        if Neo4jTransaction::extract_count(records)? > 0 || dst_ids.len() > 1 {
            Err(Error::RelDuplicated {
                rel_name: rel_name.to_string(),
            })
        } else {
            Ok(())
        }
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
        mut query: String,
        mut params: HashMap<String, Value>,
        label: &str,
        _partition_key_opt: Option<&Value>,
        props: HashMap<String, Value>,
        _info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!(
            "Neo4jTransaction::node_create_query called -- query: {}, params: {:#?}, label: {}, props: {:#?}",
            query,
            params,
            label,
            props
        );
        let node_suffix = sg.suffix();
        let props_suffix = sg.suffix();
        query.push_str(
            &("CREATE (node".to_string()
                + &node_suffix
                + ":"
                + label
                + " { id: randomUUID() })\n"
                + "SET node"
                + &node_suffix
                + " += $props"
                + &props_suffix
                + "\n"),
        );
        query =
            Neo4jTransaction::add_node_return(query, &("node".to_string() + &node_suffix), "node");

        params.insert("props".to_string() + &props_suffix, props.into());

        Ok((query, params))
    }

    fn create_node<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, Error> {
        trace!(
            "Neo4jTransaction::create_node called -- query: {}, params: {:#?}, info.name: {}",
            query,
            params,
            info.name()
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

    fn rel_create_query<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        src_query: String,
        mut params: HashMap<String, Value>,
        src_var: &str,
        dst_query: &str,
        src_label: &str,
        dst_label: &str,
        dst_var: &str,
        rel_name: &str,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_create_query called -- src_query: {}, params: {:#?}, dst_query: {}, src_label: {}, src_ids: {:#?}, dst_label: {}, dst_var: {:#?}, rel_name: {}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", 
        src_query, params, src_var, dst_query, src_label, dst_label, dst_var, rel_name, props, props_type_name, partition_key_opt);

        let src_td = info.type_def_by_name(src_label)?;
        let _src_prop = src_td.property(rel_name)?;
        let rel_var = "rel".to_string() + &sg.suffix();
        let props_var = "props".to_string() + &sg.suffix();

        /*
        if !src_prop.list() {
            self.runtime
                .block_on(Neo4jTransaction::single_rel_check::<GlobalCtx, RequestCtx>(
                    &mut self.client,
                    src_label,
                    src_ids.clone(),
                    dst_ids.clone(),
                    rel_name,
                    partition_key_opt,
                ))?;
        }
        */

        let query = src_query
            + "CALL {\n"
            + "WITH "
            + src_var
            + "\n"
            + dst_query
            + "}\n"
            + "CREATE ("
            + src_var
            + ")-["
            + &rel_var
            + ":"
            + rel_name
            + " { id: randomUUID() }]->("
            + dst_var
            + ")\n"
            + "SET "
            + &rel_var
            + " += $"
            + &props_var
            + "\n";
        let query = Neo4jTransaction::add_rel_return(query, src_var, &rel_var, dst_var);

        // params.insert("src_ids".to_string(), src_ids.into());
        // params.insert("dst_ids".to_string(), dst_ids.into());
        params.insert(props_var, props.into());

        Ok((query, params))
    }
    fn create_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        src_label: &str,
        src_ids: Vec<Value>,
        dst_label: &str,
        dst_ids: Vec<Value>,
        rel_name: &str,
        props: HashMap<String, Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        _info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::create_rels called -- query: {}, params: {:#?}, src_label: {}, src_ids: {:#?}, dst_label: {}, dst_ids: {:#?}, rel_name: {}, props: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", query, params, src_label, src_ids, dst_label, dst_ids, rel_name, props, props_type_name, partition_key_opt);

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

    fn node_read_query(
        &mut self,
        _query: String,
        rel_query_fragments: Vec<String>,
        mut params: HashMap<String, Value>,
        label: &str,
        node_var: &str,
        union_type: bool,
        return_node: Option<&str>,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_read_query called -- rel_query_fragments: {:#?}, params: {:#?}, label: {}, node_suffix: {}, untion_type: {:#?}, return_node: {:#?}, param_suffix: {}, props: {:#?}", 
        rel_query_fragments, params, label, node_var, union_type, return_node, param_suffix, props);
        let mut query = String::new();

        for rqf in rel_query_fragments {
            query.push_str(&rqf);
        }

        if union_type {
            query.push_str(&("MATCH (".to_string() + node_var + ")\n"));
        } else {
            query.push_str(&("MATCH (".to_string() + node_var + ":" + label + ")\n"));
        }

        if !props.is_empty() {
            query = props.keys().enumerate().fold(query, |mut query, (i, k)| {
                if i == 0 {
                    query.push_str("WHERE ");
                } else {
                    query.push_str(" AND ");
                }

                query.push_str(
                    &(node_var.to_string() + "." + &k + "=$" + "param" + param_suffix + "." + &k),
                );

                query
            });

            query.push_str("\n");
        }
        params.insert("param".to_string() + param_suffix, props.into());

        if let Some(return_var) = return_node {
            query = Neo4jTransaction::add_node_return(query, node_var, return_var);
        }

        Ok((query, params))
    }

    fn read_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        _partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
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

    fn rel_read_query(
        &mut self,
        _query: String,
        mut params: HashMap<String, Value>,
        src_label: &str,
        src_var: &str,
        src_query_opt: Option<String>,
        rel_name: &str,
        rel_suffix: &str,
        dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_read_query -- params: {:#?}, src_label: {}, src_var: {}, src_query_opt: {:#?}, rel_name: {}, dst_var: {}, dst_suffix: {}, dst_query_opt: {:#?}, return_rel: {:#?}, props: {:#?}", params, src_label, src_var, src_query_opt, rel_name, dst_var, dst_suffix, dst_query_opt, return_rel, props);

        let mut query = "MATCH (".to_string()
            + src_var
            + ":"
            + src_label
            + ")-[rel"
            + rel_suffix
            + ":"
            + rel_name
            + "]->("
            + dst_var
            + ")\n";

        let param_var = "param".to_string() + &sg.suffix();
        let empty_props = props.is_empty();
        if !empty_props {
            query = props.keys().enumerate().fold(query, |mut query, (i, k)| {
                if i == 0 {
                    query.push_str("WHERE ");
                } else {
                    query.push_str(" AND ");
                }

                query.push_str(
                    &("rel".to_string() + rel_suffix + "." + &k + " = $" + &param_var + "." + &k),
                );

                query
            });

            query.push_str("\n");

            params.insert(param_var, props.into());
        }

        /*
        if let Some(src_ids) = src_ids_opt {
            if empty_props {
                query.push_str("\nWHERE ");
            } else {
                query.push_str(" AND ");
            }

            query.push_str(
                &(src_var.to_string()
                    + src_suffix
                    + ".id IN $"
                    + rel_name
                    + src_suffix
                    + dst_suffix
                    + "_srcids"
                    + "."
                    + "ids"),
            );

            let mut id_map = HashMap::new();
            id_map.insert("ids".to_string(), Value::Array(src_ids));
            params.insert(
                rel_name.to_string() + src_suffix + dst_suffix + "_srcids",
                id_map.into(),
            );
        }
        query.push_str("\n");
        */

        if let Some(src_query) = src_query_opt {
            query.push_str(&src_query);
        }

        if let Some(dst_query) = dst_query_opt {
            query.push_str(&dst_query);
        }

        if return_rel {
            query = Neo4jTransaction::add_rel_return(
                query,
                src_var,
                &("rel".to_string() + rel_suffix),
                dst_var,
            );
        }

        Ok((query, params))
    }

    fn read_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
        params_opt: Option<HashMap<String, Value>>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::read_rels called -- query: {}, props_type_name: {:#?}, partition_key_opt: {:#?}, params_opt: {:#?}", query, props_type_name, partition_key_opt, params_opt);
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
        mut query: String,
        mut params: HashMap<String, Value>,
        node_suffix: &str,
        ids: Vec<Value>,
        props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::node_update_query called -- query: {}, params: {:#?}, node_suffix: {}, ids: {:#?}, props: {:#?}", 
        query, params, node_suffix, ids, props);
        let props_suffix = sg.suffix();
        query.push_str(
            &("SET node".to_string() + node_suffix + " += $props" + &props_suffix + "\n"),
        );
        query =
            Neo4jTransaction::add_node_return(query, &("node".to_string() + node_suffix), "node");

        params.insert("props".to_string() + &props_suffix, props.into());
        params.insert("ids".to_string(), ids.into());

        Ok((query, params))
    }

    fn update_nodes<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        _partition_key_opt: Option<&Value>,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, Error> {
        trace!(
            "Neo4jTransaction::update_nodes called -- query: {}, params: {:#?}, info.name: {}",
            query,
            params,
            info.name()
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
        query: String,
        mut params: HashMap<String, Value>,
        src_label: &str,
        src_suffix: &str,
        rel_name: &str,
        rel_suffix: &str,
        rel_var: &str,
        dst_suffix: &str,
        props: HashMap<String, Value>,
        _props_type_name: Option<&str>,
        _partition_key_opt: Option<&Value>,
        sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_update_query called -- query: {}, params: {:#?}, src_label: {}, rel_name: {}, props: {:#?}", 
        query, params, src_label, rel_name, props);

        let props_var = "props".to_string() + &sg.suffix();

        let mut query = query + "SET " + rel_var + " += $" + &props_var + "\n";
        query = Neo4jTransaction::add_rel_return(
            query,
            &("src".to_string() + src_suffix),
            &("rel".to_string() + rel_suffix),
            &("dst".to_string() + dst_suffix),
        );

        params.insert(props_var, props.into());

        Ok((query, params))
    }

    fn update_rels<GlobalCtx: GlobalContext, RequestCtx: RequestContext>(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_label: &str,
        _rel_name: &str,
        _rel_ids: Vec<Value>,
        props_type_name: Option<&str>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, Error> {
        trace!("Neo4jTransaction::update_rels called -- query: {}, params: {:#?}, props_type_name: {:#?}, partition_key_opt: {:#?}", query, params, props_type_name, partition_key_opt);
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
        query: String,
        params: HashMap<String, Value>,
        node_var: &str,
        label: &str,
        ids: Vec<Value>,
        _partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!(
            "Neo4jTransaction::node_delete_query called -- query: {}, params: {:#?}, node_var: {}, label: {}, ids: {:#?}",
            query,
            params,
            node_var,
            label,
            ids
        );
        let query = query + "DETACH DELETE " + node_var + "\n" + "RETURN count(*) as count\n";

        let mut hm: HashMap<String, Value> = HashMap::new();
        hm.insert("ids".to_string(), ids.into());

        Ok((query, params))
    }

    fn delete_nodes(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _label: &str,
        _ids: Vec<Value>,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        trace!(
            "Neo4jTransaction::delete_nodes called -- query: {}, params: {:#?}",
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

    fn rel_delete_query(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        src_label: &str,
        rel_name: &str,
        rel_suffix: &str,
        _partition_key_opt: Option<&Value>,
        _sg: &mut SuffixGenerator,
    ) -> Result<(String, HashMap<String, Value>), Error> {
        trace!("Neo4jTransaction::rel_delete_query called -- params: {:#?}, src_label: {}, rel_name: {}, rel_suffix: {:#?}", 
        params, src_label, rel_name, rel_suffix);
        let query = query + "DELETE rel" + rel_suffix + "\n" + "RETURN count(*) as count\n";

        Ok((query, params))
    }

    fn delete_rels(
        &mut self,
        query: String,
        params: HashMap<String, Value>,
        _src_label: &str,
        _rel_name: &str,
        _rel_ids: Vec<Value>,
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
