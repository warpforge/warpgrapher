//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::engine::config::{Resolvers, Validators};
use crate::engine::database::DatabasePool;
use crate::engine::extensions::Extensions;
use juniper::Context;
use std::fmt::Debug;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the Neo4J database in to the resolvers.
///
/// ['GraphQLContext']: ./struct/GraphQLContext.html
pub struct GraphQLContext<GlobalCtx, ReqCtx>
where
    ReqCtx: RequestContext,
{
    /// Connection pool database
    pub pool: DatabasePool,
    pub resolvers: Resolvers<GlobalCtx, ReqCtx>,
    pub validators: Validators,
    pub extensions: Extensions<GlobalCtx, ReqCtx>,
    pub global_ctx: Option<GlobalCtx>,
    pub req_ctx: Option<ReqCtx>,
    pub version: Option<String>,
}

impl<GlobalCtx, ReqCtx> GraphQLContext<GlobalCtx, ReqCtx>
where
    ReqCtx: RequestContext,
{
    /// Takes a DatabasePool and returns a
    /// ['GraphQLContext'] containing that connection pool.
    ///
    /// ['GraphQLContext']: ./struct/GraphQLContext.html
    pub fn new(
        pool: DatabasePool,
        resolvers: Resolvers<GlobalCtx, ReqCtx>,
        validators: Validators,
        extensions: Extensions<GlobalCtx, ReqCtx>,
        global_ctx: Option<GlobalCtx>,
        req_ctx: Option<ReqCtx>,
        version: Option<String>,
    ) -> GraphQLContext<GlobalCtx, ReqCtx> {
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
}

impl<GlobalCtx, ReqCtx> Context for GraphQLContext<GlobalCtx, ReqCtx> where ReqCtx: RequestContext {}

pub trait RequestContext: Clone + Debug + Send + Sync {
    fn new() -> Self;
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "neo4j")]
    use super::GraphQLContext;
    #[cfg(feature = "neo4j")]
    use crate::engine::config::{Resolvers, Validators};
    #[cfg(feature = "neo4j")]
    use crate::engine::database::neo4j::Neo4jEndpoint;
    #[cfg(feature = "neo4j")]
    use crate::engine::database::DatabaseEndpoint;

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
            ne.get_pool().unwrap(),
            resolvers,
            validators,
            vec![],
            Some(()),
            Some(()),
            None,
        );
    }
}
