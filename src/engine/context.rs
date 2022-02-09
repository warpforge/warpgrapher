//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the database.
use crate::engine::database::no_database::NoDatabaseEndpoint;
use crate::engine::database::DatabaseEndpoint;
use crate::engine::events::EventHandlerBag;
use crate::engine::loader::{NodeLoader, RelLoader};
use crate::engine::resolvers::{ResolverFunc, Resolvers};
use crate::engine::schema::Info;
use crate::engine::validators::Validators;
use crate::Error;
use juniper::Context;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;
use ultra_batch::Batcher;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the database in to the resolvers.
///
/// ['GraphQLContext']: ./struct.GraphQLContext.html
#[allow(clippy::upper_case_acronyms)]
pub struct GraphQLContext<RequestCtx: RequestContext> {
    pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    node_batcher: Batcher<NodeLoader<RequestCtx>>,
    rel_batcher: Batcher<RelLoader<RequestCtx>>,
    resolvers: Resolvers<RequestCtx>,
    validators: Validators,
    event_handlers: EventHandlerBag<RequestCtx>,
    request_ctx: Option<RequestCtx>,
    version: Option<String>,
    metadata: HashMap<String, String>,
}

impl<RequestCtx> GraphQLContext<RequestCtx>
where
    RequestCtx: RequestContext,
{
    /// Creates a new context, used for providing additional information for use in Warpgrapher
    /// resolvers
    ///
    /// # Arguments
    ///
    /// * pool - the [`DatabasePool`] that provides connections to the graph storage back-end
    /// * resolvers - the [`Resolvers`] structure containing any custom resolvers provided as
    /// part of the Warpgrapher configuration
    /// * validators - the [`Validators`] structure containing any custom input validators
    /// provided as part of the Warpgrapher configuration
    /// * event_handlers - the [`EventHandlerBag`] structure containing business logic
    /// * extensions - the [`Extensions`] structure containing any pre- or post-request hooks
    /// * request_ctx - an optional per-request context, implementing the [`RequestContext`] trait,
    /// provided by the application using the Warpgrapher framework to pass application-specific,
    /// request-specific context to custom resolvers
    /// * version - an optional version of the application service using the Warpgrapher framework,
    /// used to respond to the version static endpoint
    ///
    /// [`DatabasePool`]: ../database/trait.DatabasePool.html
    /// [`EventHandlerBag`]: ../events/struct.EventHandlerBag.html
    /// [`Extensions`]: ../extensions/type.Extensions.html
    /// [`RequestContext`]: ./trait.RequestContext.html
    /// [`Resolvers`]: ../resolvers/type.Resolvers.html
    /// [`Validators`]: ../validators/type.Validators.html
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
        resolvers: Resolvers<RequestCtx>,
        validators: Validators,
        event_handlers: EventHandlerBag<RequestCtx>,
        request_ctx: Option<RequestCtx>,
        version: Option<String>,
        metadata: HashMap<String, String>,
        info: Info,
    ) -> GraphQLContext<RequestCtx> {
        let node_batcher = Batcher::new(NodeLoader::<RequestCtx>::new(pool.clone(), info)).build();
        let rel_batcher = Batcher::new(RelLoader::<RequestCtx>::new(pool.clone())).build();
        GraphQLContext {
            pool,
            node_batcher,
            rel_batcher,
            resolvers,
            validators,
            event_handlers,
            request_ctx,
            version,
            metadata,
        }
    }

    /// Returns a pool of database connections
    pub fn pool(
        &self,
    ) -> &<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType {
        &self.pool
    }

    /// Takes the name of a custom resolver and returns the function implementing that resolver
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ResolverNotFound`] if the context does not contain a
    /// resolver function associated with the name argument
    ///
    /// [`Error`]: ../../enum.Error.html
    /// [`ResolverNotFound`]: ../../enum.Error.html#variant.ResolverNotFound
    pub fn resolver(&self, name: &str) -> Result<&ResolverFunc<RequestCtx>, Error> {
        self.resolvers
            .get(name)
            .map(|b| b.as_ref())
            .ok_or_else(|| Error::ResolverNotFound {
                name: name.to_owned(),
            })
    }

    /// Returns the set of custom input validation functions
    pub fn validators(&self) -> &Validators {
        &self.validators
    }

    /// Returns the collection of event handlers providing business logic for before and after
    /// Warpgrapher's auto-generated CRUD operations.
    pub fn event_handlers(&self) -> &EventHandlerBag<RequestCtx> {
        &self.event_handlers
    }

    /// Returns an optional string for the version of the GraphQL service
    pub fn version(&self) -> Option<&String> {
        self.version.as_ref()
    }

    /// Returns the request-specific context
    pub fn request_context(&self) -> Option<&RequestCtx> {
        self.request_ctx.as_ref()
    }

    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    pub fn node_batcher(&self) -> &Batcher<NodeLoader<RequestCtx>> {
        &self.node_batcher
    }

    pub fn rel_batcher(&self) -> &Batcher<RelLoader<RequestCtx>> {
        &self.rel_batcher
    }
}

impl<RequestCtx> Context for GraphQLContext<RequestCtx> where RequestCtx: RequestContext {}

impl<RequestCtx> Debug for GraphQLContext<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("GraphQLContext")
            .field("request_ctx", &self.request_ctx)
            .field("version", &self.version)
            .finish()
    }
}

/// Trait that, when implemented, marks a struct as a request context, used to pass data to custom
/// extensions and resolvers on a per-request basis
///
/// # Examples
///
/// ```rust,no_run
/// # use warpgrapher::engine::context::RequestContext;
/// # use warpgrapher::engine::database::no_database::NoDatabaseEndpoint;
///
/// #[derive(Clone, Debug)]
/// struct AppRequestContext {
///     session_token: String
/// }
///
/// impl RequestContext for AppRequestContext {
///     type DBEndpointType = NoDatabaseEndpoint;
///     fn new() -> Self {
///         AppRequestContext { session_token: "".to_string() }
///     }
/// }
///
/// let ac = AppRequestContext { session_token: "".to_string() };
/// ```
pub trait RequestContext: 'static + Clone + Debug + Send + Sync {
    type DBEndpointType: DatabaseEndpoint;
    fn new() -> Self;
}

impl RequestContext for () {
    type DBEndpointType = NoDatabaseEndpoint;
    fn new() {}
}

#[cfg(test)]
mod tests {

    use super::GraphQLContext;
    use crate::engine::database::no_database::NoDatabaseEndpoint;
    use crate::engine::database::DatabaseEndpoint;
    use crate::engine::events::EventHandlerBag;
    use crate::engine::resolvers::Resolvers;
    use crate::engine::schema::Info;
    use crate::engine::validators::Validators;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Passes if the pool can be created without panicking
    #[tokio::test]
    async fn engine_new() {
        let ne = NoDatabaseEndpoint {};
        let resolvers: Resolvers<()> = Resolvers::new();
        let validators: Validators = Validators::new();
        let _gqlctx: GraphQLContext<()> = GraphQLContext::new(
            ne.pool()
                .await
                .expect("Expected to unwrap Cypher database pool."),
            resolvers,
            validators,
            EventHandlerBag::new(),
            Some(()),
            None,
            HashMap::<String, String>::new(),
            Info::new(String::new(), Arc::new(HashMap::new())),
        );
    }
}
