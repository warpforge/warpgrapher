use crate::engine::context::RequestContext;
use crate::engine::database::{DatabaseEndpoint, DatabasePool, Transaction};
use crate::engine::objects::{Node, Options, Rel};
use crate::engine::schema::Info;
use crate::error::Error;
use async_trait::async_trait;
use log::trace;
use std::collections::HashMap;
use ultra_batch::{Cache, Fetcher};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeLoaderKey {
    id: String,
    options: Options,
}

impl NodeLoaderKey {
    pub fn new(id: String, options: Options) -> Self {
        NodeLoaderKey { id, options }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
}

pub struct NodeLoader<RequestCtx: RequestContext> {
    pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    info: Info,
}

impl<RequestCtx> NodeLoader<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub fn new(
        pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
        info: Info,
    ) -> Self {
        NodeLoader::<RequestCtx> { pool, info }
    }

    fn pool(
        &self,
    ) -> &<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType {
        &self.pool
    }
}

#[async_trait]
impl<RequestCtx> Fetcher for NodeLoader<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Key = NodeLoaderKey;
    type Value = Node<RequestCtx>;
    type Error = Error;

    async fn fetch(
        &self,
        keys: &[NodeLoaderKey],
        values: &mut Cache<'_, NodeLoaderKey, Node<RequestCtx>>,
    ) -> Result<(), Error> {
        trace!("NodeLoader::fetch called -- keys: {:#?}", keys);

        let mut transaction = self.pool().transaction().await?;
        let results = transaction
            .load_nodes::<RequestCtx>(keys, &self.info)
            .await?;

        results.into_iter().try_for_each(|n| {
            values.insert(
                NodeLoaderKey::new(n.id()?.to_string(), Options::new(Vec::new())),
                n,
            );

            Ok::<(), Error>(())
        })?;

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelLoaderKey {
    src_id: String,
    rel_name: String,
    options: Options,
}

impl RelLoaderKey {
    pub fn new(src_id: String, rel_name: String, options: Options) -> Self {
        RelLoaderKey {
            src_id,
            rel_name,
            options,
        }
    }

    pub fn src_id(&self) -> &str {
        &self.src_id
    }

    pub fn rel_name(&self) -> &str {
        &self.rel_name
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
}

pub struct RelLoader<RequestCtx: RequestContext> {
    pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
}

impl<RequestCtx> RelLoader<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub fn new(
        pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    ) -> Self {
        RelLoader::<RequestCtx> { pool }
    }

    fn pool(
        &self,
    ) -> &<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType {
        &self.pool
    }
}

#[async_trait]
impl<RequestCtx> Fetcher for RelLoader<RequestCtx>
where
    RequestCtx: RequestContext,
{
    type Key = RelLoaderKey;
    type Value = Vec<Rel<RequestCtx>>;
    type Error = Error;

    async fn fetch(
        &self,
        keys: &[RelLoaderKey],
        values: &mut Cache<'_, RelLoaderKey, Vec<Rel<RequestCtx>>>,
    ) -> Result<(), Error> {
        trace!("RelLoader::fetch called -- keys: {:#?}", keys);

        let mut transaction = self.pool().transaction().await?;
        let results = transaction.load_rels::<RequestCtx>(keys).await?;

        let mut rel_map: HashMap<RelLoaderKey, Vec<Rel<RequestCtx>>> = HashMap::new();
        let options_map: HashMap<String, Options> = keys
            .iter()
            .map(|rlk| {
                rel_map.insert(rlk.clone(), Vec::new());
                (rlk.src_id().to_string(), rlk.options().clone())
            })
            .collect();

        results.into_iter().try_for_each(|r| {
            let rlk = RelLoaderKey::new(
                r.src_id()?.to_string(),
                r.rel_name().to_string(),
                options_map
                    .get(&r.src_id()?.to_string())
                    .cloned()
                    .unwrap_or_default(),
            );
            let mut rel_list = rel_map.remove(&rlk).unwrap_or_default();
            rel_list.push(r);
            rel_map.insert(rlk, rel_list);
            Ok::<(), Error>(())
        })?;

        rel_map.into_iter().for_each(|(k, v)| values.insert(k, v));

        Ok(())
    }
}
