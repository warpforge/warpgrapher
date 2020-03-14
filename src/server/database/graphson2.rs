use super::{get_env_var, DatabaseEndpoint, DatabasePool, QueryResult, Transaction};
use crate::server::context::WarpgrapherRequestContext;
use crate::server::objects::{Node, Rel};
use crate::{Error, ErrorKind};
use juniper::FieldError;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;

pub struct Graphson2Endpoint {
    _db_url: String,
}

impl Graphson2Endpoint {
    pub fn from_env() -> Result<Graphson2Endpoint, Error> {
        Ok(Graphson2Endpoint {
            _db_url: get_env_var("WG_GRAPHSON2_URL")?,
        })
    }
}

impl DatabaseEndpoint for Graphson2Endpoint {
    fn get_pool(&self) -> Result<DatabasePool, Error> {
        Ok(DatabasePool::Graphson2)
    }
}

/*
pub struct Graphson2Client {}

impl DatabaseClient for Graphson2Client {
    type ImplTransaction = Graphson2Transaction;

    fn get_transaction(&self) -> Result<Graphson2Transaction, FieldError> {
        Ok(Graphson2Transaction {})
    }
}
*/

pub struct Graphson2Transaction {}

impl Transaction for Graphson2Transaction {
    type ImplQueryResult = Graphson2QueryResult;

    fn begin(&self) -> Result<(), FieldError> {
        Ok(())
    }
    fn commit(&mut self) -> Result<(), FieldError> {
        Ok(())
    }
    fn exec<V>(
        &mut self,
        _query: &str,
        _params: Option<&HashMap<String, V>>,
    ) -> Result<Graphson2QueryResult, FieldError>
    where
        V: Debug + Serialize,
    {
        Err(Error::new(ErrorKind::UnsupportedDatabase("test mock".to_owned()), None).into())
    }
    fn rollback(&mut self) -> Result<(), FieldError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Graphson2QueryResult {}

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

    fn get_ids(&self, _type_name: &str) -> Result<Vec<String>, FieldError> {
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
