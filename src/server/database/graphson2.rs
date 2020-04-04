use super::{
    get_env_string, get_env_u16, DatabaseEndpoint, DatabasePool, QueryResult, Transaction,
};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
use crate::server::value::Value;
use crate::{Error, ErrorKind};
#[cfg(feature = "graphson2")]
// use gremlin_client::process::traversal::traversal;
#[cfg(feature = "graphson2")]
use gremlin_client::{ConnectionOptions, GValue, GraphSON, GremlinClient, GremlinResult, ToGValue};
use juniper::FieldError;
use log::trace;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::Debug;
// use uuid::Uuid;

pub struct Graphson2Endpoint {
    host: String,
    port: u16,
    login: String,
    pass: String,
}

impl Graphson2Endpoint {
    pub fn from_env() -> Result<Graphson2Endpoint, Error> {
        Ok(Graphson2Endpoint {
            host: get_env_string("WG_GRAPHSON2_HOST")?,
            port: get_env_u16("WG_GRAPHSON2_PORT")?,
            login: get_env_string("WG_GRAPHSON2_LOGIN")?,
            pass: get_env_string("WG_GRAPHSON2_PASS")?,
        })
    }
}

impl DatabaseEndpoint for Graphson2Endpoint {
    fn get_pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Graphson2(
            GremlinClient::connect(
                ConnectionOptions::builder()
                    .host(&self.host)
                    .port(self.port)
                    .pool_size(num_cpus::get().try_into().unwrap_or(8))
                    .ssl(true)
                    .serializer(GraphSON::V2)
                    .credentials(&self.login, &self.pass)
                    .build(),
            )
            .map_err(|e| Error::new(ErrorKind::CouldNotBuildGraphson2Pool(e), None))?,
        ))
    }
}

pub struct Graphson2Transaction {
    client: GremlinClient,
}

impl Graphson2Transaction {
    pub fn new(client: GremlinClient) -> Graphson2Transaction {
        Graphson2Transaction { client }
    }
}

impl Transaction for Graphson2Transaction {
    type ImplQueryResult = Graphson2QueryResult;

    fn begin(&self) -> Result<(), FieldError> {
        Ok(())
    }

    fn commit(&mut self) -> Result<(), FieldError> {
        Ok(())
    }

    fn create_node(
        &mut self,
        label: &str,
        partition_key_opt: &Option<String>,
        props: HashMap<String, Value>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        let mut query = String::from("g.addV('") + label + "')";
        query.push_str(".property('partitionKey', partitionKey)");
        for (k, _v) in props.iter() {
            query.push_str(".property('");
            query.push_str(k);
            query.push_str("', ");
            query.push_str(k);
            query.push_str(")");
        }

        self.exec(&query, partition_key_opt, Some(props))
    }

    fn exec(
        &mut self,
        query: &str,
        partition_key_opt: &Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<Graphson2QueryResult, FieldError> {
        trace!(
            "Graphson2Transaction::exec called -- query: {}, partition_key: {:#?}, param: {:#?}",
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

            let results = self.client.execute(query, param_list.as_slice())?;

            let mut v = Vec::new();
            for r in results {
                v.push(r);
            }
            trace!("Graphson2Transaction::exec query results: {:#?}", v);

            Ok(Graphson2QueryResult::new(v))
        } else {
            Err(Error::new(ErrorKind::MissingPartitionKey, None).into())
        }
    }

    fn rollback(&mut self) -> Result<(), FieldError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Graphson2QueryResult {
    results: Vec<GremlinResult<GValue>>,
}

impl Graphson2QueryResult {
    pub fn new(results: Vec<GremlinResult<GValue>>) -> Graphson2QueryResult {
        Graphson2QueryResult { results }
    }
}

impl QueryResult for Graphson2QueryResult {
    fn get_nodes<GlobalCtx, ReqCtx>(
        &self,
        _type_name: &str,
    ) -> Result<Vec<Node<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        Err(Error::new(ErrorKind::UnsupportedDatabase("test mock".to_owned()), None).into())
    }

    fn get_rels<GlobalCtx, ReqCtx>(
        &self,
        _src_name: &str,
        _src_suffix: &str,
        _rel_name: &str,
        _dst_name: &str,
        _dst_suffix: &str,
        _props_type_name: Option<&str>,
    ) -> Result<Vec<Rel<GlobalCtx, ReqCtx>>, FieldError>
    where
        GlobalCtx: Debug,
        ReqCtx: WarpgrapherRequestContext + Debug,
    {
        Err(Error::new(ErrorKind::UnsupportedDatabase("test mock".to_owned()), None).into())
    }

    fn get_ids(&self, _type_name: &str) -> Result<Value, FieldError> {
        Err(Error::new(ErrorKind::UnsupportedDatabase("test mock".to_owned()), None).into())
    }

    fn get_count(&self) -> Result<i32, FieldError> {
        Err(Error::new(ErrorKind::UnsupportedDatabase("test mock".to_owned()), None).into())
    }

    fn len(&self) -> i32 {
        0
    }

    fn is_empty(&self) -> bool {
        true
    }
}