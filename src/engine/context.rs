//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::engine::database::DatabasePool;
use crate::engine::extensions::{Extension, Extensions};
use crate::engine::objects::resolvers::{ResolverFunc, Resolvers};
use crate::engine::validators::Validators;
use crate::Error;
use juniper::Context;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::slice::Iter;
use std::sync::Arc;

/// Juniper Context for Warpgrapher's GraphQL queries. The ['GraphQLContext'] is
/// used to pass a connection pool for the database in to the resolvers.
///
/// ['GraphQLContext']: ./struct.GraphQLContext.html
///
/// # Examples
///
/// ```rust,norun
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::DatabaseEndpoint;
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
/// # use warpgrapher::engine::objects::resolvers::Resolvers;
/// # use warpgrapher::engine::validators::Validators;
/// # use warpgrapher::engine::context::GraphQLContext;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "neo4j")]
/// let ne = Neo4jEndpoint::from_env()?;
/// let resolvers: Resolvers<(), ()> = Resolvers::new();
/// let validators: Validators = Validators::new();
/// # #[cfg(feature = "neo4j")]
/// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
///     ne.pool()?,
///     resolvers,
///     validators,
///     vec![],
///     Some(()),
///     Some(()),
///     None,
/// );
/// # Ok(())
/// # }
/// ```
pub struct GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    pool: DatabasePool,
    resolvers: Resolvers<GlobalCtx, RequestCtx>,
    validators: Validators,
    extensions: Extensions<GlobalCtx, RequestCtx>,
    global_ctx: Option<GlobalCtx>,
    request_ctx: Option<RequestCtx>,
    version: Option<String>,
}

impl<GlobalCtx, RequestCtx> GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
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
    /// * extensions - the [`Extensions`] structure containing any pre- or post-request hooks
    /// * global_ctx - an optional global context, implementing the [`GlobalContext`] trait,
    /// provided by the application using the Warpgrapher framework to pass application-specific
    /// global context to custom resolvers
    /// * request_ctx - an optional per-request context, implementing the [`RequestContext`] trait,
    /// provided by the application using the Warpgrapher framework to pass application-specific,
    /// request-specific context to custom resolvers
    /// * version - an optional version of the application service using the Warpgrapher framework,
    /// used to respond to the version static endpoint
    ///
    /// [`DatabasePool`]: ../database/enum.DatabasePool.html
    /// [`Resolvers`]: ../objects/resolvers/type.Resolvers.html
    /// [`Validators`]: ../validators/type.Validators.html
    /// [`Extensions`]: ../extensions/type.Extensions.html
    /// [`GlobalContext`]: ./trait.GlobalContext.html
    /// [`RequestContext`]: ./trait.RequestContext.html
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<(), ()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     None,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        pool: DatabasePool,
        resolvers: Resolvers<GlobalCtx, RequestCtx>,
        validators: Validators,
        extensions: Extensions<GlobalCtx, RequestCtx>,
        global_ctx: Option<GlobalCtx>,
        request_ctx: Option<RequestCtx>,
        version: Option<String>,
    ) -> GraphQLContext<GlobalCtx, RequestCtx> {
        GraphQLContext {
            pool,
            resolvers,
            validators,
            extensions,
            global_ctx,
            request_ctx,
            version,
        }
    }

    /// Returns a pool of database connections
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<(), ()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     None,
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let db_pool = gqlctx.pool();
    /// # Ok(())
    /// # }
    /// ```
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    /// Takes the name of a custom resolver and returns the function implementing that resolver
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ResolverNotFound`] if the context does not contain a
    /// resolver function associated with the name argument
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::{Resolvers, ResolverContext};
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::ExecutionResult;
    ///
    /// # #[cfg(feature = "neo4j")]
    /// pub fn project_count(context: ResolverContext<(), ()>) -> ExecutionResult {
    ///     if let DatabasePool::Neo4j(p) = context.executor().context().pool() {
    ///         let db = p.get()?;
    ///         let query = "MATCH (n:Project) RETURN (n)";
    ///         let results = db.exec(query)?;
    ///         context.resolve_scalar(results.data.len() as i32)
    ///     } else {
    ///         panic!("Unsupported database.");
    ///     }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut resolvers: Resolvers<(), ()> = Resolvers::new();
    /// # #[cfg(feature = "neo4j")]
    /// resolvers.insert("ProjectCount".to_string(), Box::new(project_count));
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     None,
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx = gqlctx.resolver("CustomResolver");
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolver(&self, name: &str) -> Result<&ResolverFunc<GlobalCtx, RequestCtx>, Error> {
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
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<(), ()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     None,
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

    /// Returns an optional string for the version of the GraphQL service
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<(), ()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     Some("0.0.0".to_string()),
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// assert_eq!(&Some("0.0.0".to_string()), gqlctx.version());
    /// # Ok(())
    /// # }
    /// ```
    pub fn version(&self) -> &Option<String> {
        &self.version
    }

    /// Returns an iterator over the registered extensions, each offering potentially a pre-request
    /// and a post-request hook
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<(), ()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some(()),
    ///     Some("0.0.0".to_string()),
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let extensions = gqlctx.extensions();
    /// # Ok(())
    /// # }
    /// ```
    pub fn extensions(&self) -> Iter<Arc<dyn Extension<GlobalCtx, RequestCtx>>> {
        self.extensions.iter()
    }

    /// Returns the global request context
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::{GlobalContext, GraphQLContext, RequestContext};
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     version: String,
    /// }
    ///
    /// impl GlobalContext for AppGlobalCtx {}
    /// #[derive(Clone, Debug)]
    /// pub struct AppRequestCtx {
    ///     request_id: String,
    /// }
    ///
    /// impl RequestContext for AppRequestCtx {
    ///    fn new() -> AppRequestCtx {
    ///        AppRequestCtx {
    ///            request_id: "".to_string()
    ///        }
    ///    }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<AppGlobalCtx, AppRequestCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppGlobalCtx, AppRequestCtx> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(AppGlobalCtx { version: "0.0.0".to_string() }),
    ///     Some(AppRequestCtx::new()),
    ///     Some("0.0.0".to_string()),
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let global_context = gqlctx.global_context();
    /// # Ok(())
    /// # }
    /// ```
    pub fn global_context(&self) -> &Option<GlobalCtx> {
        &self.global_ctx
    }

    /// Returns the request-specific context
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::{GlobalContext, GraphQLContext, RequestContext};
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     version: String,
    /// }
    ///
    /// impl GlobalContext for AppGlobalCtx {}
    /// #[derive(Clone, Debug)]
    /// pub struct AppRequestCtx {
    ///     request_id: String,
    /// }
    ///
    /// impl RequestContext for AppRequestCtx {
    ///    fn new() -> AppRequestCtx {
    ///        AppRequestCtx {
    ///            request_id: "".to_string()    
    ///        }
    ///    }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<AppGlobalCtx, AppRequestCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppGlobalCtx, AppRequestCtx> = GraphQLContext::new(
    ///     ne.pool()?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(AppGlobalCtx { version: "0.0.0".to_string() }),
    ///     Some(AppRequestCtx::new()),
    ///     Some("0.0.0".to_string()),
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let request_context = gqlctx.request_context();
    /// # Ok(())
    /// # }
    /// ```
    pub fn request_context(&self) -> &Option<RequestCtx> {
        &self.request_ctx
    }
}

impl<GlobalCtx, RequestCtx> Context for GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
}

impl<GlobalCtx, RequestCtx> Debug for GraphQLContext<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("GraphQLContext")
            .field("pool", &self.pool)
            .field("extensions", &self.extensions)
            .field("global_ctx", &self.global_ctx)
            .field("request_ctx", &self.request_ctx)
            .field("version", &self.version)
            .finish()
    }
}

/// Trait that, when implemented, marks a struct as a global context, used to pass data to custom
/// extensions and resolvers
///
/// # Examples
///
/// ```rust,norun
/// # use warpgrapher::engine::context::GlobalContext;
///
/// #[derive(Clone, Debug)]
/// struct AppContext {
///     app_specific_config: String
/// }
///
/// impl GlobalContext for AppContext {}
///
/// let ac = AppContext { app_specific_config: "".to_string() };
/// ```
pub trait GlobalContext: 'static + Clone + Debug + Send + Sync {}

impl GlobalContext for () {}

/// Trait that, when implemented, marks a struct as a request context, used to pass data to custom
/// extensions and resolvers on a per-request basis
///
/// # Examples
///
/// ```rust,norun
/// # use warpgrapher::engine::context::RequestContext;
///
/// #[derive(Clone, Debug)]
/// struct AppRequestContext {
///     session_token: String
/// }
///
/// impl RequestContext for AppRequestContext {
///     fn new() -> Self {
///         AppRequestContext { session_token: "".to_string() }
///     }
/// }
///
/// let ac = AppRequestContext { session_token: "".to_string() };
/// ```
pub trait RequestContext: 'static + Clone + Debug + Send + Sync {
    fn new() -> Self;
}

impl RequestContext for () {
    fn new() {}
}

#[cfg(feature = "neo4j")]
#[cfg(test)]
mod tests {

    use super::GraphQLContext;
    use crate::engine::database::neo4j::Neo4jEndpoint;
    use crate::engine::database::DatabaseEndpoint;
    use crate::engine::objects::resolvers::Resolvers;
    use crate::engine::validators::Validators;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Passes if the pool can be created without panicking
    #[test]
    fn engine_new() {
        init();

        let ne = Neo4jEndpoint::from_env().expect("Couldn't build database pool from env vars.");
        let resolvers: Resolvers<(), ()> = Resolvers::new();
        let validators: Validators = Validators::new();
        let _gqlctx: GraphQLContext<(), ()> = GraphQLContext::new(
            ne.pool().expect("Expected to unwrap Neo4J database pool."),
            resolvers,
            validators,
            vec![],
            Some(()),
            Some(()),
            None,
        );
    }
}
