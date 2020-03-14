//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::server::config::{WarpgrapherResolvers, WarpgrapherValidators};
use crate::server::database::DatabasePool;
use crate::server::extensions::WarpgrapherExtensions;
use juniper::Context;
use std::fmt::Debug;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the Neo4J database in to the resolvers.
///
/// ['GraphQLContext']: ./struct/GraphQLContext.html
pub struct GraphQLContext<GlobalCtx, ReqCtx>
where
    ReqCtx: WarpgrapherRequestContext,
{
    /// Connection pool database
    pub pool: DatabasePool,
    pub resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
    pub validators: WarpgrapherValidators,
    pub extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    pub global_ctx: Option<GlobalCtx>,
    pub req_ctx: Option<ReqCtx>,
    pub version: Option<String>,
}

impl<GlobalCtx, ReqCtx> GraphQLContext<GlobalCtx, ReqCtx>
where
    ReqCtx: WarpgrapherRequestContext,
{
    /// Takes a DatabasePool and returns a
    /// ['GraphQLContext'] containing that connection pool.
    ///
    /// ['GraphQLContext']: ./struct/GraphQLContext.html
    pub fn new(
        pool: DatabasePool,
        resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
        validators: WarpgrapherValidators,
        extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
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

impl<GlobalCtx, ReqCtx> Context for GraphQLContext<GlobalCtx, ReqCtx> where
    ReqCtx: WarpgrapherRequestContext
{
}

pub trait WarpgrapherRequestContext: Clone + Debug + Send + Sync {
    fn new() -> Self;
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "neo4j")]
    use super::GraphQLContext;
    #[cfg(feature = "neo4j")]
    use crate::server::config::{WarpgrapherResolvers, WarpgrapherValidators};
    #[cfg(feature = "neo4j")]
    use crate::server::database::neo4j::Neo4jEndpoint;
    #[cfg(feature = "neo4j")]
    use crate::server::database::DatabaseEndpoint;

    #[cfg(feature = "neo4j")]
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Passes if the pool can be created without panicking
    #[cfg(feature = "neo4j")]
    #[test]
    fn server_new() {
        init();

        let ne = Neo4jEndpoint::from_env().unwrap();
        let resolvers: WarpgrapherResolvers<(), ()> = WarpgrapherResolvers::new();
        let validators: WarpgrapherValidators = WarpgrapherValidators::new();
        let _gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
            ne.get_pool().unwrap(),
            resolvers.clone(),
            validators.clone(),
            vec![],
            Some(()),
            Some(()),
            None,
        );
    }
}
