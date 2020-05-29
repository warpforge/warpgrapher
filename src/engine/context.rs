//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::engine::config::Validators;
use crate::engine::database::DatabasePool;
use crate::engine::extensions::{Extension, Extensions};
use crate::engine::objects::resolvers::{ResolverFunc, Resolvers};
use crate::Error;
use juniper::Context;
use std::fmt::Debug;
use std::slice::Iter;
use std::sync::Arc;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the Neo4J database in to the resolvers.
///
/// ['GraphQLContext']: ./struct/GraphQLContext.html
pub struct GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    /// Connection pool database
    pool: DatabasePool,
    resolvers: Resolvers<GlobalCtx, RequestCtx>,
    validators: Validators,
    extensions: Extensions<GlobalCtx, RequestCtx>,
    global_ctx: Option<GlobalCtx>,
    req_ctx: Option<RequestCtx>,
    version: Option<String>,
}

impl<GlobalCtx, RequestCtx> GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    /// Takes a DatabasePool and returns a
    /// ['GraphQLContext'] containing that connection pool.
    ///
    /// ['GraphQLContext']: ./struct/GraphQLContext.html
    pub fn new(
        pool: DatabasePool,
        resolvers: Resolvers<GlobalCtx, RequestCtx>,
        validators: Validators,
        extensions: Extensions<GlobalCtx, RequestCtx>,
        global_ctx: Option<GlobalCtx>,
        req_ctx: Option<RequestCtx>,
        version: Option<String>,
    ) -> GraphQLContext<GlobalCtx, RequestCtx> {
        GraphQLContext {
            pool,
            resolvers,
            validators,
            extensions,
            global_ctx,
            req_ctx,
            version,
        }
    }

    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn resolver(&self, name: &str) -> Result<&ResolverFunc<GlobalCtx, RequestCtx>, Error> {
        self.resolvers
            .get(name)
            .ok_or_else(|| Error::ResolverNotFound {
                name: name.to_owned(),
            })
            .and_then(|b| Ok(b.as_ref()))
    }

    pub fn validators(&self) -> &Validators {
        &self.validators
    }

    pub fn version(&self) -> &Option<String> {
        &self.version
    }

    pub fn extensions(&self) -> Iter<Arc<dyn Extension<GlobalCtx, RequestCtx> + Send + Sync>> {
        self.extensions.iter()
    }

    pub fn global_context(&self) -> &Option<GlobalCtx> {
        &self.global_ctx
    }

    pub fn request_context(&self) -> &Option<RequestCtx> {
        &self.req_ctx
    }
}

impl<GlobalCtx, RequestCtx> Context for GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
}

pub trait GlobalContext: 'static + Clone + Debug + Send + Sync {}

impl GlobalContext for () {}

pub trait RequestContext: 'static + Clone + Debug + Send + Sync {
    fn new() -> Self;
}

impl RequestContext for () {
    fn new() {}
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "neo4j")]
    use super::GraphQLContext;
    #[cfg(feature = "neo4j")]
    use crate::engine::config::Validators;
    #[cfg(feature = "neo4j")]
    use crate::engine::database::neo4j::Neo4jEndpoint;
    #[cfg(feature = "neo4j")]
    use crate::engine::database::DatabaseEndpoint;
    #[cfg(feature = "neo4j")]
    use crate::engine::objects::resolvers::Resolvers;

    #[cfg(feature = "neo4j")]
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Passes if the pool can be created without panicking
    #[cfg(feature = "neo4j")]
    #[test]
    fn engine_new() {
        init();

        let ne = Neo4jEndpoint::from_env().unwrap();
        let resolvers: Resolvers<(), ()> = Resolvers::new();
        let validators: Validators = Validators::new();
        let _gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
            ne.pool().unwrap(),
            resolvers,
            validators,
            vec![],
            Some(()),
            Some(()),
            None,
        );
    }
}
