//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::server::config::{WarpgrapherResolvers, WarpgrapherValidators};
use crate::server::extensions::WarpgrapherExtensions;
use juniper::Context;
use r2d2::Pool;
use r2d2_cypher::CypherConnectionManager;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the Neo4J database in to the resolvers.
///
/// ['GraphQLContext']: ./struct/GraphQLContext.html
pub struct GraphQLContext<GlobalCtx, ReqCtx: WarpgrapherRequestContext>
where
    ReqCtx: WarpgrapherRequestContext,
{
    /// Connection pool for the Neo4J database
    pub pool: Pool<CypherConnectionManager>,
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
    /// Takes an r2d2 Pool of CypherConnectionManager structs and returns a
    /// ['GraphQLContext'] containing that connection pool.
    ///
    /// ['GraphQLContext']: ./struct/GraphQLContext.html
    pub fn new(
        pool: Pool<CypherConnectionManager>,
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

impl<GlobalCtx, ReqCtx: WarpgrapherRequestContext> Context for GraphQLContext<GlobalCtx, ReqCtx> {}

pub trait WarpgrapherRequestContext {
    fn new() -> Self;
}

#[cfg(test)]
mod tests {

    use super::GraphQLContext;
    use crate::server::config::{WarpgrapherResolvers, WarpgrapherValidators};
    use r2d2_cypher::CypherConnectionManager;
    use std::env::var_os;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Passes if the pool can be created without panicking
    #[test]
    fn server_new() {
        init();

        let db_url = match var_os("DB_URL") {
            None => "http://neo4j:testpass@127.0.0.1:7474/db/data".to_owned(),
            Some(os) => os
                .to_str()
                .unwrap_or("http://neo4j:testpass@127.0.0.1:7474/db/data")
                .to_owned(),
        };

        let manager = CypherConnectionManager { url: db_url };
        let pool = r2d2::Pool::builder().max_size(5).build(manager).unwrap();
        let resolvers: WarpgrapherResolvers<(), ()> = WarpgrapherResolvers::new();
        let validators: WarpgrapherValidators = WarpgrapherValidators::new();
        let _gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
            pool,
            resolvers.clone(),
            validators.clone(),
            vec![],
            Some(()),
            Some(()),
            None,
        );
    }
}
