//! Provides database interface types and functions for Cosmos DB

use super::{env_string, env_u16, DatabaseEndpoint, DatabasePool, QueryResult, Transaction};
use crate::engine::context::{GlobalContext, RequestContext};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::Error;
use gremlin_client::{ConnectionOptions, GKey, GValue, GraphSON, GremlinClient, ToGValue};
use juniper::FieldError;
use log::trace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;

/// A Cosmos DB endpoint collects the information necessary to generate a connection string and
/// build a database connection pool.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::cosmos::CosmosEndpoint;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ce = CosmosEndpoint::from_env()?;
/// #    Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CosmosEndpoint {
    host: String,
    port: u16,
    login: String,
    pass: String,
}

impl CosmosEndpoint {
    /// Reads a set of environment variables to construct a [`CosmosEndpoint`]. The environment
    /// variables are:actix
    ///
    /// * WG_COSMOS_HOST - the hostname for the Cosmos DB. For example,
    /// *my-db*.gremlin.cosmos.azure.com
    /// * WG_COSMOS_PORT - the port number for the Cosmos DB. For example, 443
    /// * WG_COSMOS_LOGIN - the database and collection of the Cosmos DB. For example,
    /// /dbs/*my-db-name*/colls/*my-collection-name*
    /// * WG_COSMOS_PASS - the read/write key for the Cosmos DB.
    ///
    /// [`CosmosEndpoint`]: ./struct.CosmosEndpoint.html
    ///
    /// # Errors
    ///
    /// * [`EnvironmentVariableNotFound`] - if an environment variable does not exist
    /// * [`EnvironmentVariableParseError`] - if an environment variable has the wrong type,
    /// typically meaning that the WG_COSMOS_PORT variable cannot be parsed from a strign into an
    /// integer
    ///
    /// [`EnvironmentVariableNotFound`]: ../../enum.ErrorKind.html
    /// [`EnvironmentVariableParseError`]: ../../enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use warpgrapher::engine::database::cosmos::CosmosEndpoint;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let ce = CosmosEndpoint::from_env()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn from_env() -> Result<CosmosEndpoint, Error> {
        Ok(CosmosEndpoint {
            host: env_string("WG_COSMOS_HOST")?,
            port: env_u16("WG_COSMOS_PORT")?,
            login: env_string("WG_COSMOS_LOGIN")?,
            pass: env_string("WG_COSMOS_PASS")?,
        })
    }
}

impl DatabaseEndpoint for CosmosEndpoint {
    fn pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Cosmos(GremlinClient::connect(
            ConnectionOptions::builder()
                .host(&self.host)
                .port(self.port)
                .pool_size(num_cpus::get().try_into().unwrap_or(8))
                .ssl(true)
                .serializer(GraphSON::V1)
                .deserializer(GraphSON::V1)
                .credentials(&self.login, &self.pass)
                .build(),
        )?))
    }
}

/// A Cosmos DB transaction is the interface for making queries against the back-end database.
/// Notably, this struct is a little mis-named, at the moment, in that it doesn't provide any
/// transactional ACID capabilities. It's just a placeholder for query capability, to follow the
/// trait signatures. Later there may be an effort to support Cosmos DB optimitistic concurrency
/// model.
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
/// # use warpgrapher::engine::database::cosmos::{CosmosEndpoint, CosmosTransaction};
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let ce = CosmosEndpoint::from_env()?;
///     if let DatabasePool::Cosmos(client) = ce.pool()? {
///         let transaction = CosmosTransaction::new(client.clone());
///     }
/// #   Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CosmosTransaction {
    client: GremlinClient,
}

impl CosmosTransaction {
    pub fn new(client: GremlinClient) -> CosmosTransaction {
        CosmosTransaction { client }
    }
}

impl Transaction for CosmosTransaction {
    type ImplQueryResult = CosmosQueryResult;
    fn begin(&self) -> Result<(), FieldError> {
        Ok(())
    }

    fn commit(&mut self) -> Result<(), FieldError> {
        Ok(())
    }

    fn create_node<GlobalCtx, RequestCtx>(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
        info: &Info,
    ) -> Result<Node<GlobalCtx, RequestCtx>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        let mut params = HashMap::new();
        let mut query = String::from("g.addV('") + label + "')";
        query.push_str(".property('partitionKey', partitionKey)");
        for (k, v) in props.into_iter() {
            if let Value::Array(a) = v {
                for (i, val) in a.into_iter().enumerate() {
                    query.push_str(
                        &(String::from(".property(list, '")
                            + &k
                            + "', "
                            + &k
                            + &i.to_string()
                            + ")"),
                    );
                    params.insert(k.to_owned() + &i.to_string(), val);
                }
            } else {
                query.push_str(".property(");
                query.push_str("'");
                query.push_str(&k);
                query.push_str("', ");
                query.push_str(&k);
                query.push_str(")");
                params.insert(k, v);
            }
        }
        query.push_str(".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())");

        Ok(self
            .exec(&query, partition_key_opt, Some(params))?
            .nodes(label, info)?
            .into_iter()
            .next()
            .unwrap())
    }

    fn create_rels<GlobalCtx, RequestCtx>(
        &mut self,
        src_label: &str,
        src_ids: Value,
        dst_label: &str,
        dst_ids: Value,
        rel_name: &str,
        params: &mut HashMap<String, Value>,
        partition_key_opt: &Option<String>,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        trace!("CosmosTransaction::create_rels called -- src_label: {}, src_ids: {:#?}, dst_label: {}, dst_ids: {:#?}, rel_name: {}, params: {:#?}, partition_key_opt: {:#?}.", src_label, src_ids, dst_label, dst_ids, rel_name, params, partition_key_opt);

        let mut props = HashMap::new();
        if let Some(Value::Map(pm)) = params.remove("props") {
            // remove rather than get to take ownership
            for (k, v) in pm.into_iter() {
                props.insert(k.to_owned(), v);
            }
        }

        if let (Value::Array(src_id_vec), Value::Array(dst_id_vec)) = (src_ids, dst_ids) {
            let src_td = info.type_def_by_name(src_label)?;
            let src_prop = src_td.prop(rel_name)?;

            if !src_prop.list() {
                let mut check_query = String::from("g.V()");
                check_query.push_str(&(String::from(".has('partitionKey', partitionKey)")));
                check_query.push_str(&(String::from(".has('label', '") + src_label + "')"));
                check_query.push_str(".hasId(");
                for (i, id) in src_id_vec.iter().enumerate() {
                    if let Value::String(id_str) = id {
                        if i == 0 {
                            check_query.push_str(&(String::from("'") + &id_str + "'"));
                        } else {
                            check_query.push_str(&(String::from(", '") + &id_str + "'"));
                        }
                    } else {
                        trace!("src_id_vec element not a  string");
                        return Err(Error::TypeNotExpected.into());
                    }
                }
                check_query.push_str(&(String::from(").outE('") + rel_name + "').count()"));
                let check_results =
                    self.exec(&check_query, partition_key_opt, Some(props.clone()))?; // TODO -- remove cloning
                if check_results.count()? > 0 || dst_id_vec.len() > 1 {
                    return Err(Error::RelDuplicated {
                        rel_name: rel_name.to_string(),
                    }
                    .into()); // TODO -- the multi-dst condition should have its own error kind for selecting too many destination nodes
                }
            }

            let mut query = String::from("g.V()");
            query.push_str(".has('partitionKey', partitionKey)");
            query.push_str(&(String::from(".hasLabel('") + dst_label + "')"));
            query.push_str(&String::from(".hasId("));

            for (i, id) in dst_id_vec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        query.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        query.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    trace!("dst_id_vec element not a  string");
                    return Err(Error::TypeNotExpected.into());
                }
            }

            query.push_str(&(String::from(").as('dst')")));
            query.push_str(&(String::from(".V()")));
            query.push_str(&(String::from(".has('partitionKey', partitionKey)")));
            query.push_str(&(String::from(".hasLabel('") + src_label + "')"));
            query.push_str(&(String::from(".hasId(")));

            for (i, id) in src_id_vec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        query.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        query.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    trace!("src_id_vec element not a string");
                    return Err(Error::TypeNotExpected.into());
                }
            }

            query.push_str(&String::from(")"));
            query.push_str(&(String::from(".addE('") + rel_name + "').to('dst')"));
            query.push_str(".property('partitionKey', partitionKey)");
            for (k, _v) in props.iter() {
                query.push_str(".property('");
                query.push_str(k);
                query.push_str("', ");
                query.push_str(k);
                query.push_str(")");
            }
            query.push_str(".project('rID', 'rLabel', 'rProps', 'srcID', 'srcLabel', 'srcProps', 'dstID', 'dstLabel', 'dstProps')");
            query.push_str(".by(id()).by(label()).by(valueMap())");
            query.push_str(".by(outV().id()).by(outV().label()).by(outV().valueMap())");
            query.push_str(".by(inV().id()).by(inV().label()).by(inV().valueMap())");

            trace!("CosmosTransaction::rel_create about to exec query -- query: {}, partition_key_opt: {:#?}, props: {:#?}", query, partition_key_opt, props);

            self.exec(&query, partition_key_opt, Some(props))?.rels(
                src_label,
                "",
                rel_name,
                dst_label,
                "",
                props_type_name,
                info,
            )
        } else {
            trace!("src or dst argument not an array of strings");
            Err(Error::TypeNotExpected.into())
        }
    }

    fn delete_nodes(
        &mut self,
        label: &str,
        ids: Value,
        partition_key_opt: &Option<String>,
    ) -> Result<i32, FieldError> {
        if let Value::Array(idvec) = ids {
            let mut qs = String::from("g.V().hasLabel('") + label + "')";
            qs.push_str(".has('partitionKey', partitionKey)");

            let length = idvec.len();
            qs.push_str(".hasId(");
            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            }

            qs.push_str(").drop()");

            self.exec(&qs, partition_key_opt, None)?;

            let mut v: Vec<GValue> = Vec::new();
            v.push(GValue::Int32(length as i32));
            Ok(length as i32)
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }

    fn delete_rels(
        &mut self,
        _src_label: &str,
        rel_name: &str,
        rel_ids: Value,
        partition_key_opt: &Option<String>,
        _info: &Info,
    ) -> Result<i32, FieldError> {
        if let Value::Array(idvec) = rel_ids {
            let mut qs = String::from("g.E().hasLabel('") + rel_name + "')";
            qs.push_str(".has('partitionKey', partitionKey)");

            let length = idvec.len();
            qs.push_str(".hasId(");
            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            }

            qs.push_str(").drop()");

            self.exec(&qs, partition_key_opt, None)?;

            let mut v: Vec<GValue> = Vec::new();
            v.push(GValue::Int32(length as i32));
            Ok(length as i32)
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }

    fn exec(
        &mut self,
        query: &str,
        partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<CosmosQueryResult, FieldError> {
        trace!(
            "CosmosTransaction::exec called -- query: {}, partition_key: {:#?}, param: {:#?}",
            query,
            partition_key_opt,
            params
        );

        if let Some(pk) = partition_key_opt {
            let mut param_list: Vec<(&str, &dyn ToGValue)> = Vec::new();
            let pms = params.unwrap_or_else(HashMap::new);
            for (k, v) in pms.iter() {
                param_list.push((k.as_str(), v))
            }
            param_list.push(("partitionKey", pk));

            let raw_results = self.client.execute(query, param_list.as_slice());
            let results = raw_results?;

            let mut v = Vec::new();
            for r in results {
                v.push(r?);
            }

            Ok(CosmosQueryResult::new(v))
        } else {
            Err(Error::PartitionKeyNotFound.into())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn node_query_string(
        &mut self,
        // query_string: &str,
        rel_query_fragments: Vec<String>,
        params: &mut HashMap<String, Value>,
        label: &str,
        _var_suffix: &str,
        union_type: bool,
        return_node: bool,
        param_suffix: &str,
        props: HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        trace!(
            "transaction::node_query_string called, label: {}, union_type: {:#?}, return_node: {:#?}, param_suffix: {}",
            label, union_type, return_node, param_suffix
        );

        let mut qs = String::new();

        if return_node {
            qs.push_str("g.V()");
        }

        if !union_type {
            qs.push_str(&(String::from(".hasLabel('") + label + "')"));
        }

        qs.push_str(".has('partitionKey', partitionKey)");

        for (k, v) in props.into_iter() {
            qs.push_str(&(String::from(".has('") + &k + "', " + &k + param_suffix + ")"));
            params.insert(k + param_suffix, v);
        }

        if !rel_query_fragments.is_empty() {
            qs.push_str(".where(");

            if rel_query_fragments.len() > 1 {
                qs.push_str("and(");
            }

            for (i, rqf) in rel_query_fragments.iter().enumerate() {
                if i == 0 {
                    qs.push_str(&(String::from("outE()") + rqf));
                } else {
                    qs.push_str(&(String::from(", outE()") + rqf));
                }
            }

            if rel_query_fragments.len() > 1 {
                qs.push_str(")");
            }

            qs.push_str(")");
        }

        if return_node {
            qs.push_str(".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())");
        }

        trace!("node_query_string -- query_string: {}", qs);
        Ok(qs)
    }

    fn rel_query_string(
        &mut self,
        // query: &str,
        src_label: &str,
        src_suffix: &str,
        src_ids_opt: Option<Value>,
        src_query_opt: Option<String>,
        rel_name: &str,
        _dst_var: &str,
        dst_suffix: &str,
        dst_query_opt: Option<String>,
        return_rel: bool,
        props: HashMap<String, Value>,
        params: &mut HashMap<String, Value>,
    ) -> Result<String, FieldError> {
        let mut qs = String::new();

        if return_rel {
            qs.push_str("g.E()");
        }

        qs.push_str(&(String::from(".hasLabel('") + rel_name + "')"));
        qs.push_str(".has('partitionKey', partitionKey)");

        for (k, v) in props.into_iter() {
            qs.push_str(
                &(String::from(".has('") + &k + "', " + &k + src_suffix + dst_suffix + ")"),
            );
            params.insert(k + src_suffix + dst_suffix, v);
        }
        qs.push_str(".where(");

        if dst_query_opt.is_some() {
            qs.push_str("and(");
        }

        if let Some(src_query) = src_query_opt {
            qs.push_str(&(String::from("outV()") + &src_query));
        } else {
            qs.push_str(&(String::from("outV().hasLabel('") + src_label + "')"));
        }

        if let Some(Value::String(id)) = src_ids_opt {
            qs.push_str(&(String::from(".hasId('") + &id + "')"));
        } else if let Some(Value::Array(idvec)) = src_ids_opt {
            qs.push_str(".hasId(");

            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            }

            qs.push_str(")");
        }

        if let Some(dst_query) = dst_query_opt {
            qs.push_str(", ");
            qs.push_str(&(String::from("inV()") + &dst_query));
            qs.push_str(")");
        }

        qs.push_str(")");

        if return_rel {
            qs.push_str(".project('rID', 'rLabel', 'rProps', 'srcID', 'srcLabel', 'srcProps', 'dstID', 'dstLabel', 'dstProps')");
            qs.push_str(".by(id()).by(label()).by(valueMap())");
            qs.push_str(".by(outV().id()).by(outV().label()).by(outV().valueMap())");
            qs.push_str(".by(inV().id()).by(inV().label()).by(inV().valueMap())");
        }

        trace!("rel_query_string -- query_string: {}", qs);
        Ok(qs)
    }

    fn rollback(&mut self) -> Result<(), FieldError> {
        Ok(())
    }

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
        RequestCtx: RequestContext,
    {
        trace!("transaction::update_nodes called, label: {}, ids: {:#?}, props: {:#?}, partition_key_opt: {:#?}", label, ids, props, partition_key_opt);

        if let Value::Array(idvec) = ids {
            let mut qs = String::from("g.V().hasLabel('") + label + "')";
            qs.push_str(".has('partitionKey', partitionKey)");

            qs.push_str(".hasId(");
            for (i, id) in idvec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            }

            qs.push_str(")");

            let mut params = HashMap::new();
            for (k, v) in props.into_iter() {
                if let Value::Array(a) = v {
                    for (i, val) in a.into_iter().enumerate() {
                        qs.push_str(
                            &(String::from(".property(list, '")
                                + &k
                                + "', "
                                + &k
                                + &i.to_string()
                                + ")"),
                        );
                        params.insert(k.to_owned() + &i.to_string(), val);
                    }
                } else {
                    qs.push_str(".property(");
                    qs.push_str("'");
                    qs.push_str(&k);
                    qs.push_str("', ");
                    qs.push_str(&k);
                    qs.push_str(")");
                    params.insert(k, v);
                }
            }
            qs.push_str(".project('nID', 'nLabel', 'nProps').by(id()).by(label()).by(valueMap())");

            self.exec(&qs, partition_key_opt, Some(params))?
                .nodes(label, info)
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }

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
        RequestCtx: RequestContext,
    {
        trace!("CosmosTransaction::update_rels called, src_label: {}, rel_name: {}, rel_ids: {:#?}, partition_key_opt: {:#?}, props: {:#?}, props_type_name: {:#?}", src_label, rel_name, rel_ids, partition_key_opt, props, props_type_name);

        if let Value::Array(rel_id_vec) = rel_ids {
            let mut qs = String::from("g.E().hasLabel('") + rel_name + "')";
            qs.push_str(".has('partitionKey', partitionKey)");

            qs.push_str(".hasId(");
            for (i, id) in rel_id_vec.iter().enumerate() {
                if let Value::String(id_str) = id {
                    if i == 0 {
                        qs.push_str(&(String::from("'") + &id_str + "'"));
                    } else {
                        qs.push_str(&(String::from(", '") + &id_str + "'"));
                    }
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            }
            qs.push_str(")");

            for k in props.keys() {
                qs.push_str(&(String::from(".property('") + k + "', " + k + ")"));
            }
            qs.push_str(".project('rID', 'rLabel', 'rProps', 'srcID', 'srcLabel', 'srcProps', 'dstID', 'dstLabel', 'dstProps')");
            qs.push_str(".by(id()).by(label()).by(valueMap())");
            qs.push_str(".by(outV().id()).by(outV().label()).by(outV().valueMap())");
            qs.push_str(".by(inV().id()).by(inV().label()).by(inV().valueMap())");

            self.exec(&qs, partition_key_opt, Some(props))?.rels(
                src_label,
                "",
                rel_name,
                "",
                "",
                props_type_name,
                info,
            )
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }
}

#[derive(Debug)]
pub struct CosmosQueryResult {
    results: Vec<GValue>,
}

impl CosmosQueryResult {
    fn new(results: Vec<GValue>) -> CosmosQueryResult {
        CosmosQueryResult { results }
    }
}

impl QueryResult for CosmosQueryResult {
    fn nodes<GlobalCtx, RequestCtx>(
        self,
        _name: &str,
        info: &Info,
    ) -> Result<Vec<Node<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        /*
        trace!(
            "CosmosQueryResult::nodes self.results: {:#?}",
            self.results
        );
        */

        let mut v = Vec::new();
        for result in self.results {
            if let GValue::Map(map) = result {
                let mut hm = HashMap::new();
                for (k, v) in map.into_iter() {
                    if let GKey::String(s) = k {
                        hm.insert(s, v);
                    } else {
                        return Err(Error::TypeNotExpected.into());
                    }
                }

                if let (
                    Some(GValue::String(id)),
                    Some(GValue::String(label)),
                    Some(GValue::Map(props)),
                ) = (hm.remove("nID"), hm.remove("nLabel"), hm.remove("nProps"))
                {
                    let type_def = info.type_def_by_name(&label)?;

                    let mut fields = HashMap::new();
                    fields.insert("id".to_string(), Value::String(id.to_owned()));

                    for (key, property_list) in props.into_iter() {
                        if let (GKey::String(k), GValue::List(plist)) = (key, property_list) {
                            if k == "partitionKey" || !type_def.prop(&k)?.list() {
                                fields.insert(
                                    k.to_owned(),
                                    plist
                                        .into_iter()
                                        .next()
                                        .ok_or_else(|| Error::ResponseItemNotFound {
                                            name: k.to_string(),
                                        })?
                                        .try_into()?,
                                );
                            } else {
                                let mut prop_vals = Vec::new();
                                for val in plist.into_iter() {
                                    prop_vals.push(val.try_into()?);
                                }
                                fields.insert(k.to_owned(), Value::Array(prop_vals));
                            }
                        } else {
                            return Err(Error::TypeNotExpected.into());
                        }
                    }

                    v.push(Node::new(label.to_owned(), fields))
                } else {
                    return Err(Error::TypeNotExpected.into());
                }
            } else {
                return Err(Error::TypeNotExpected.into());
            }
        }

        trace!("CosmosQueryResult::nodes returning {:#?}", v);

        Ok(v)
    }

    fn rels<GlobalCtx, RequestCtx>(
        self,
        _src_name: &str,
        _src_suffix: &str,
        _rel_name: &str,
        _dst_name: &str,
        _dst_suffix: &str,
        props_type_name: Option<&str>,
        info: &Info,
    ) -> Result<Vec<Rel<GlobalCtx, RequestCtx>>, FieldError>
    where
        GlobalCtx: GlobalContext,
        RequestCtx: RequestContext,
    {
        trace!(
            "CosmosQueryResult::rels -- self.results: {:#?}, props_type_name: {:#?}",
            self.results,
            props_type_name
        );

        let mut v = Vec::new();
        for result in self.results {
            if let GValue::Map(m) = result {
                let mut hm = HashMap::new();
                for (k, v) in m.into_iter() {
                    if let GKey::String(s) = k {
                        hm.insert(s, v);
                    } else {
                        return Err(Error::TypeNotExpected.into());
                    }
                }

                if let (
                    Some(GValue::String(rel_id)),
                    Some(GValue::String(_rel_label)),
                    Some(GValue::Map(rel_props)),
                    Some(GValue::String(src_id)),
                    Some(GValue::String(src_label)),
                    Some(GValue::Map(src_props)),
                    Some(GValue::String(dst_id)),
                    Some(GValue::String(dst_label)),
                    Some(GValue::Map(dst_props)),
                ) = (
                    hm.remove("rID"),
                    hm.remove("rLabel"),
                    hm.remove("rProps"),
                    hm.remove("srcID"),
                    hm.remove("srcLabel"),
                    hm.remove("srcProps"),
                    hm.remove("dstID"),
                    hm.remove("dstLabel"),
                    hm.remove("dstProps"),
                ) {
                    let src_type_def = info.type_def_by_name(&src_label)?;

                    let mut src_fields = HashMap::new();
                    src_fields.insert("id".to_string(), Value::String(src_id.to_owned()));

                    for (key, property_list) in src_props.into_iter() {
                        if let (GKey::String(k), GValue::List(plist)) = (key, property_list) {
                            if k == "partitionKey" || !src_type_def.prop(&k)?.list() {
                                src_fields.insert(
                                    k.to_owned(),
                                    plist
                                        .into_iter()
                                        .next()
                                        .ok_or_else(|| Error::ResponseItemNotFound {
                                            name: k.to_owned(),
                                        })?
                                        .try_into()?,
                                );
                            } else {
                                let mut prop_vals = Vec::new();
                                for val in plist.into_iter() {
                                    prop_vals.push(val.try_into()?);
                                }
                                src_fields.insert(k.to_owned(), Value::Array(prop_vals));
                            }
                        } else {
                            return Err(Error::TypeNotExpected.into());
                        }
                    }

                    let dst_type_def = info.type_def_by_name(&dst_label)?;

                    let mut dst_fields = HashMap::new();
                    dst_fields.insert("id".to_string(), Value::String(dst_id.to_owned()));

                    for (key, property_list) in dst_props.into_iter() {
                        if let (GKey::String(k), GValue::List(plist)) = (key, property_list) {
                            if k == "partitionKey" || !dst_type_def.prop(&k)?.list() {
                                dst_fields.insert(
                                    k.to_owned(),
                                    plist
                                        .into_iter()
                                        .next()
                                        .ok_or_else(|| Error::ResponseItemNotFound {
                                            name: k.to_string(),
                                        })?
                                        .try_into()?,
                                );
                            } else {
                                let mut prop_vals = Vec::new();
                                for val in plist.into_iter() {
                                    prop_vals.push(val.try_into()?);
                                }
                                dst_fields.insert(k.to_owned(), Value::Array(prop_vals));
                            }
                        } else {
                            return Err(Error::TypeNotExpected.into());
                        }
                    }

                    let mut rel_fields = HashMap::new();
                    rel_fields.insert("id".to_string(), Value::String(rel_id.to_owned()));
                    for (key, v) in rel_props.into_iter() {
                        if let GKey::String(k) = key {
                            rel_fields.insert(k.to_owned(), v.try_into()?);
                        } else {
                            return Err(Error::TypeNotExpected.into());
                        }
                    }

                    v.push(Rel::new(
                        Value::String(rel_id),
                        match props_type_name {
                            Some(p_type_name) => {
                                Some(Node::new(p_type_name.to_string(), rel_fields))
                            }
                            None => None,
                        },
                        Node::new(src_label, src_fields),
                        Node::new(dst_label, dst_fields),
                    ));
                } else {
                    return Err(Error::ResponseItemNotFound {
                        name: "Rel, src, or dst".to_string(),
                    }
                    .into());
                }
            } else {
                return Err(Error::TypeNotExpected.into());
            }
        }

        trace!("CosmosQueryResult::rels returning {:#?}", v);

        Ok(v)
    }

    fn ids(&self, _type_name: &str) -> Result<Value, FieldError> {
        /*
        trace!(
            "CosmosQueryResult::ids self.results: {:#?}",
            self.results
        );
        */
        let mut v = Vec::new();
        for result in &self.results {
            if let GValue::Map(map) = result {
                if let Some(GValue::String(id)) = map.get("nID") {
                    v.push(Value::String(id.to_string()));
                } else if let Some(GValue::String(id)) = map.get("rID") {
                    v.push(Value::String(id.to_string()));
                } else {
                    return Err(Error::ResponseItemNotFound {
                        name: "ID".to_string(),
                    }
                    .into());
                }
            } else {
                return Err(Error::TypeNotExpected.into());
            }
        }
        Ok(Value::Array(v))
    }

    fn count(&self) -> Result<i32, FieldError> {
        trace!("CosmosQueryResult::count self.results: {:#?}", self.results);

        if let Some(GValue::Int32(i)) = self.results.get(0) {
            Ok(*i)
        } else if let Some(GValue::Int64(i)) = self.results.get(0) {
            Ok(*i as i32)
        } else {
            Err(Error::TypeNotExpected.into())
        }
    }

    fn len(&self) -> i32 {
        trace!("CosmosQueryResult::len self.results: {:#?}", self.results);
        0
    }

    fn is_empty(&self) -> bool {
        trace!(
            "CosmosQueryResult::is_empty self.results: {:#?}",
            self.results
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{CosmosEndpoint, CosmosTransaction};
    #[test]
    fn test_endpoint_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosEndpoint>();
    }

    #[test]
    fn test_endpoint_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosEndpoint>();
    }

    #[test]
    fn test_transaction_send() {
        fn assert_send<T: Send>() {}
        assert_send::<CosmosTransaction>();
    }

    #[test]
    fn test_transaction_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<CosmosTransaction>();
    }
}
