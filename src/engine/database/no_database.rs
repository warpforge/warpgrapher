//! Provides database interface types and functions when there is no database back-end. Mostly
//! throws errors.

use crate::engine::context::RequestContext;
use crate::engine::database::{
    Comparison, DatabaseEndpoint, DatabasePool, NodeQueryVar, QueryFragment, QueryResult,
    RelQueryVar, SuffixGenerator, Transaction,
};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::error::Error;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct NoDatabaseEndpoint {}

#[async_trait]
impl DatabaseEndpoint for NoDatabaseEndpoint {
    type PoolType = NoDatabasePool;

    async fn pool(&self) -> Result<Self::PoolType, Error> {
        Ok(NoDatabasePool {})
    }
}

#[derive(Clone)]
pub struct NoDatabasePool {}

#[async_trait]
impl DatabasePool for NoDatabasePool {
    type TransactionType = NoTransaction;

    async fn read_transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(NoTransaction {})
    }

    async fn transaction(&self) -> Result<Self::TransactionType, Error> {
        Ok(NoTransaction {})
    }
}

pub struct NoTransaction {}

#[async_trait]
impl Transaction for NoTransaction {
    async fn begin(&mut self) -> Result<(), Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn execute_query<RequestCtx: RequestContext>(
        &mut self,
        _query: String,
        _params: HashMap<String, Value>,
    ) -> Result<QueryResult, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn create_node<RequestCtx: RequestContext>(
        &mut self,
        _node_var: &NodeQueryVar,
        _props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Node<RequestCtx>, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn create_rels<RequestCtx: RequestContext>(
        &mut self,
        _src_query_fragment: QueryFragment,
        _dst_query_fragment: QueryFragment,
        _rel_var: &RelQueryVar,
        _id_opt: Option<Value>,
        _props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        Err(Error::DatabaseNotFound)
    }

    fn node_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        _node_var: &NodeQueryVar,
        _nodes: &[Node<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        Err(Error::DatabaseNotFound)
    }

    fn node_read_fragment(
        &mut self,
        _rel_query_fragments: Vec<QueryFragment>,
        _node_var: &NodeQueryVar,
        _props: HashMap<String, Comparison>,
        _sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn read_nodes<RequestCtx: RequestContext>(
        &mut self,
        _node_var: &NodeQueryVar,
        _query_fragment: QueryFragment,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        Err(Error::DatabaseNotFound)
    }

    fn rel_read_by_ids_fragment<RequestCtx: RequestContext>(
        &mut self,
        _rel_var: &RelQueryVar,
        _rels: &[Rel<RequestCtx>],
    ) -> Result<QueryFragment, Error> {
        Err(Error::DatabaseNotFound)
    }

    fn rel_read_fragment(
        &mut self,
        _src_fragment_opt: Option<QueryFragment>,
        _dst_fragment_opt: Option<QueryFragment>,
        _rel_var: &RelQueryVar,
        _props: HashMap<String, Comparison>,
        _sg: &mut SuffixGenerator,
    ) -> Result<QueryFragment, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn read_rels<RequestCtx: RequestContext>(
        &mut self,
        _query_fragment: QueryFragment,
        _rel_var: &RelQueryVar,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn update_nodes<RequestCtx: RequestContext>(
        &mut self,
        _query_fragment: QueryFragment,
        _node_var: &NodeQueryVar,
        _props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn update_rels<RequestCtx: RequestContext>(
        &mut self,
        _query_fragment: QueryFragment,
        _rel_var: &RelQueryVar,
        _props: HashMap<String, Value>,
        _partition_key_opt: Option<&Value>,
        _info: &Info,
        _sg: &mut SuffixGenerator,
    ) -> Result<Vec<Rel<RequestCtx>>, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn delete_nodes(
        &mut self,
        _query_fragment: QueryFragment,
        _node_var: &NodeQueryVar,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn delete_rels(
        &mut self,
        _query_fragment: QueryFragment,
        _rel_var: &RelQueryVar,
        _partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn commit(&mut self) -> Result<(), Error> {
        Err(Error::DatabaseNotFound)
    }

    async fn rollback(&mut self) -> Result<(), Error> {
        Err(Error::DatabaseNotFound)
    }
}
