//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::engine::database::no_database::NoDatabaseEndpoint;
use crate::engine::database::DatabaseEndpoint;
use crate::engine::events::EventHandlerBag;
use crate::engine::resolvers::{ResolverFunc, Resolvers};
use crate::engine::validators::Validators;
use crate::Error;
use juniper::Context;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the database in to the resolvers.
///
/// ['GraphQLContext']: ./struct.GraphQLContext.html
///
/// # Examples
///
/// ```rust,no_run
/// # use std::collections::HashMap;
/// # use tokio::main;
/// # use warpgrapher::engine::context::RequestContext;
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::DatabaseEndpoint;
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
/// # use warpgrapher::engine::resolvers::Resolvers;
/// # use warpgrapher::engine::validators::Validators;
/// # use warpgrapher::engine::context::GraphQLContext;
/// # use warpgrapher::engine::events::EventHandlerBag;
///
/// # #[derive(Clone, Debug)]
/// # struct AppCtx {}
/// #
/// # #[cfg(feature = "neo4j")]
/// # impl RequestContext for AppCtx {
/// #   type DBEndpointType = Neo4jEndpoint;
/// #   fn new() -> Self {AppCtx{}}
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "neo4j")]
/// let ne = Neo4jEndpoint::from_env()?;
/// # #[cfg(feature = "neo4j")]
/// let resolvers: Resolvers<AppCtx> = Resolvers::new();
/// let validators: Validators = Validators::new();
/// # #[cfg(feature = "neo4j")]
/// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
///     ne.pool().await?,
///     resolvers,
///     validators,
///     EventHandlerBag::<AppCtx>::new(),
///     vec![],
///     Some(AppCtx::new()),
///     None,
///     HashMap::new()
/// );
/// # Ok(())
/// # }
/// ```
pub struct GraphQLContext<RequestCtx: RequestContext> {
    pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
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
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    /// #
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #   type DBEndpointType = Neo4jEndpoint;
    /// #   fn new() -> Self {AppCtx{}}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     None,
    ///     HashMap::new()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
        resolvers: Resolvers<RequestCtx>,
        validators: Validators,
        event_handlers: EventHandlerBag<RequestCtx>,
        request_ctx: Option<RequestCtx>,
        version: Option<String>,
        metadata: HashMap<String, String>,
    ) -> GraphQLContext<RequestCtx> {
        GraphQLContext {
            pool,
            resolvers,
            validators,
            event_handlers,
            request_ctx,
            version,
            metadata,
        }
    }

    /// Returns a pool of database connections
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    /// #
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #   type DBEndpointType = Neo4jEndpoint;
    /// #   fn new() -> Self {AppCtx{}}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     None,
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let db_pool = gqlctx.pool();
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "neo4j")]
    /// use bolt_proto::Message;
    /// use std::collections::HashMap;
    /// use std::iter::FromIterator;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::{Resolvers, ResolverFacade};
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::resolvers::ExecutionResult;
    /// # use warpgrapher::juniper::BoxFuture;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    ///
    /// # #[derive(Clone, Debug)]
    /// # pub struct AppCtx {}
    ///
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #    type DBEndpointType = Neo4jEndpoint;
    /// #    fn new() -> Self {AppCtx{}}
    /// # }
    ///
    /// # #[cfg(feature = "neo4j")]
    /// pub fn project_count(facade: ResolverFacade<AppCtx>) -> BoxFuture<ExecutionResult> {
    ///     Box::pin(async move {
    ///         let mut db = facade.db_into_neo4j().await?;
    ///         let query = "MATCH (n:Project) RETURN (n)";
    ///         db.run_with_metadata(query, None, None).await?;
    ///
    ///         let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
    ///         let (response, records) = db.pull(Some(pull_meta)).await?;
    ///         match response {
    ///             Message::Success(_) => facade.resolve_scalar(records.len() as i32),
    ///             message => Err(Error::Neo4jQueryFailed { message }.into()),
    ///         }
    ///     })
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// # #[cfg(feature = "neo4j")]
    /// resolvers.insert("ProjectCount".to_string(), Box::new(project_count));
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     None,
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx = gqlctx.resolver("CustomResolver");
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolver(&self, name: &str) -> Result<&ResolverFunc<RequestCtx>, Error> {
        self.resolvers
            .get(name)
            .map(|b| b.as_ref())
            .ok_or_else(|| Error::ResolverNotFound {
                name: name.to_owned(),
            })
    }

    /// Returns the set of custom input validation functions
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    /// #
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #   type DBEndpointType = Neo4jEndpoint;
    /// #   fn new() -> Self {AppCtx{}}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     None,
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let validators = gqlctx.validators();
    /// # Ok(())
    /// # }
    /// ```
    pub fn validators(&self) -> &Validators {
        &self.validators
    }

    /// Returns the collection of event handlers providing business logic for before and after
    /// Warpgrapher's auto-generated CRUD operations.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    /// #
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #   type DBEndpointType = Neo4jEndpoint;
    /// #   fn new() -> Self {AppCtx{}}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     None,
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let event_handlers = gqlctx.event_handlers();
    /// # Ok(())
    /// # }
    /// ```
    pub fn event_handlers(&self) -> &EventHandlerBag<RequestCtx> {
        &self.event_handlers
    }

    /// Returns an optional string for the version of the GraphQL service
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # use tokio::main;
    /// # use warpgrapher::engine::context::RequestContext;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # #[derive(Clone, Debug)]
    /// # struct AppCtx {}
    /// #
    /// # #[cfg(feature = "neo4j")]
    /// # impl RequestContext for AppCtx {
    /// #   type DBEndpointType = Neo4jEndpoint;
    /// #   fn new() -> Self {AppCtx{}}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppCtx>::new(),
    ///     vec![],
    ///     Some(AppCtx::new()),
    ///     Some("0.0.0".to_string()),
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// assert_eq!(Some(&"0.0.0".to_string()), gqlctx.version());
    /// # Ok(())
    /// # }
    /// ```
    pub fn version(&self) -> Option<&String> {
        self.version.as_ref()
    }

    /// Returns the request-specific context
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// use tokio::main;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::{GraphQLContext, RequestContext};
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppRequestCtx {
    ///     request_id: String,
    /// }
    ///
    /// # #[cfg(feature = "neo4j")]
    /// impl RequestContext for AppRequestCtx {
    ///    type DBEndpointType = Neo4jEndpoint;
    ///    fn new() -> AppRequestCtx {
    ///        AppRequestCtx {
    ///            request_id: "".to_string()    
    ///        }
    ///    }
    /// }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// # #[cfg(feature = "neo4j")]
    /// let resolvers: Resolvers<AppRequestCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppRequestCtx> = GraphQLContext::new(
    ///     ne.pool().await?,
    ///     resolvers,
    ///     validators,
    ///     EventHandlerBag::<AppRequestCtx>::new(),
    ///     vec![],
    ///     Some(AppRequestCtx::new()),
    ///     Some("0.0.0".to_string()),
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let request_context = gqlctx.request_context();
    /// # Ok(())
    /// # }
    /// ```
    pub fn request_context(&self) -> Option<&RequestCtx> {
        self.request_ctx.as_ref()
    }

    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
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
    use crate::engine::validators::Validators;
    use std::collections::HashMap;

    /// Passes if the pool can be created without panicking
    #[tokio::test]
    async fn engine_new() {
        let ne = NoDatabaseEndpoint {};
        let resolvers: Resolvers<()> = Resolvers::new();
        let validators: Validators = Validators::new();
        let _gqlctx: GraphQLContext<()> = GraphQLContext::new(
            ne.pool()
                .await
                .expect("Expected to unwrap Neo4J database pool."),
            resolvers,
            validators,
            EventHandlerBag::new(),
            vec![],
            Some(()),
            None,
            HashMap::<String, String>::new(),
        );
    }
}
