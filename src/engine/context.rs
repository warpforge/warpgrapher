//! This module provides a Juniper Context for Warpgrapher GraphQL queries. The
//! context contains a connection pool for the Neo4J database.
use crate::engine::database::DatabasePool;
use crate::engine::extensions::{Extension, Extensions};
use crate::engine::resolvers::{ResolverFunc, Resolvers};
use crate::engine::validators::Validators;
use crate::Error;
use juniper::Context;
use std::collections::HashMap;
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
/// ```rust,no_run
/// # use std::collections::HashMap;
/// # #[cfg(feature = "neo4j")]
/// # use tokio::runtime::Runtime;
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::DatabaseEndpoint;
/// # #[cfg(feature = "neo4j")]
/// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
/// # use warpgrapher::engine::resolvers::Resolvers;
/// # use warpgrapher::engine::validators::Validators;
/// # use warpgrapher::engine::context::GraphQLContext;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "neo4j")]
/// let mut runtime = Runtime::new()?;
/// # #[cfg(feature = "neo4j")]
/// let ne = Neo4jEndpoint::from_env()?;
/// let resolvers: Resolvers<()> = Resolvers::new();
/// let validators: Validators = Validators::new();
/// # #[cfg(feature = "neo4j")]
/// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
///     runtime.block_on(ne.pool())?,
///     resolvers,
///     validators,
///     vec![],
///     Some(()),
///     None,
///     HashMap::new()
/// );
/// # Ok(())
/// # }
/// ```
pub struct GraphQLContext<RequestCtx>
where
    RequestCtx: RequestContext,
{
    pool: DatabasePool,
    resolvers: Resolvers<RequestCtx>,
    validators: Validators,
    extensions: Extensions<RequestCtx>,
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
    /// * extensions - the [`Extensions`] structure containing any pre- or post-request hooks
    /// * request_ctx - an optional per-request context, implementing the [`RequestContext`] trait,
    /// provided by the application using the Warpgrapher framework to pass application-specific,
    /// request-specific context to custom resolvers
    /// * version - an optional version of the application service using the Warpgrapher framework,
    /// used to respond to the version static endpoint
    ///
    /// [`DatabasePool`]: ../database/enum.DatabasePool.html
    /// [`Extensions`]: ../extensions/type.Extensions.html
    /// [`RequestContext`]: ./trait.RequestContext.html
    /// [`Resolvers`]: ../resolvers/type.Resolvers.html
    /// [`Validators`]: ../validators/type.Validators.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     None,
    ///     HashMap::new()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: DatabasePool,
        resolvers: Resolvers<RequestCtx>,
        validators: Validators,
        extensions: Extensions<RequestCtx>,
        request_ctx: Option<RequestCtx>,
        version: Option<String>,
        metadata: HashMap<String, String>,
    ) -> GraphQLContext<RequestCtx> {
        GraphQLContext {
            pool,
            resolvers,
            validators,
            extensions,
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
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     None,
    ///     HashMap::new()
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
    /// ```rust,no_run
    /// # #[cfg(feature = "neo4j")]
    /// use bolt_proto::Message;
    /// use std::collections::HashMap;
    /// use std::iter::FromIterator;
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::{DatabaseEndpoint, DatabasePool};
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::{Resolvers, ResolverFacade};
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    /// # use warpgrapher::engine::resolvers::ExecutionResult;
    /// # use warpgrapher::Error;
    ///
    /// # #[cfg(feature = "neo4j")]
    /// pub fn project_count(facade: ResolverFacade<()>) -> ExecutionResult {
    ///     if let DatabasePool::Neo4j(p) = facade.executor().context().pool() {
    ///         let mut runtime = Runtime::new()?;
    ///         let mut db = runtime.block_on(p.get())?;
    ///         let query = "MATCH (n:Project) RETURN (n)";
    ///         runtime.block_on(db.run_with_metadata(query, None, None))
    ///             .expect("Expected successful query run.");
    ///
    ///         let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
    ///         let (response, records) = runtime.block_on(db.pull(Some(pull_meta)))?;
    ///         match response {
    ///             Message::Success(_) => (),
    ///             message => return Err(Error::Neo4jQueryFailed { message }.into()),
    ///         }
    ///
    ///         facade.resolve_scalar(records.len() as i32)
    ///     } else {
    ///         panic!("Unsupported database.");
    ///     }
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut resolvers: Resolvers<()> = Resolvers::new();
    /// # #[cfg(feature = "neo4j")]
    /// resolvers.insert("ProjectCount".to_string(), Box::new(project_count));
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
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
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
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

    /// Returns an optional string for the version of the GraphQL service
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
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

    /// Returns an iterator over the registered extensions, each offering potentially a pre-request
    /// and a post-request hook
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "neo4j")]
    /// # use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::GraphQLContext;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # #[cfg(feature = "neo4j")]
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<()> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<()> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
    ///     vec![],
    ///     Some(()),
    ///     Some("0.0.0".to_string()),
    ///     HashMap::new()
    /// );
    ///
    /// # #[cfg(feature = "neo4j")]
    /// let extensions = gqlctx.extensions();
    /// # Ok(())
    /// # }
    /// ```
    pub fn extensions(&self) -> Iter<Arc<dyn Extension<RequestCtx>>> {
        self.extensions.iter()
    }

    /// Returns the request-specific context
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::collections::HashMap;
    /// # #[cfg(feature = "neo4j")]
    /// use tokio::runtime::Runtime;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::DatabaseEndpoint;
    /// # #[cfg(feature = "neo4j")]
    /// # use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    /// # use warpgrapher::engine::validators::Validators;
    /// # use warpgrapher::engine::context::{GraphQLContext, RequestContext};
    ///
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
    /// let mut runtime = Runtime::new()?;
    /// # #[cfg(feature = "neo4j")]
    /// let ne = Neo4jEndpoint::from_env()?;
    /// let resolvers: Resolvers<AppRequestCtx> = Resolvers::new();
    /// let validators: Validators = Validators::new();
    /// # #[cfg(feature = "neo4j")]
    /// let gqlctx: GraphQLContext<AppRequestCtx> = GraphQLContext::new(
    ///     runtime.block_on(ne.pool())?,
    ///     resolvers,
    ///     validators,
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
            .field("pool", &self.pool)
            .field("extensions", &self.extensions)
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
    use crate::engine::resolvers::Resolvers;
    use crate::engine::validators::Validators;
    use std::collections::HashMap;
    use tokio::runtime::Runtime;

    /// Passes if the pool can be created without panicking
    #[test]
    fn engine_new() {
        let runtime = Runtime::new().expect("Expected new runtime.");

        let ne = Neo4jEndpoint::from_env().expect("Couldn't build database pool from env vars.");
        let resolvers: Resolvers<()> = Resolvers::new();
        let validators: Validators = Validators::new();
        let _gqlctx: GraphQLContext<()> = GraphQLContext::new(
            runtime
                .block_on(ne.pool())
                .expect("Expected to unwrap Neo4J database pool."),
            resolvers,
            validators,
            vec![],
            Some(()),
            None,
            HashMap::<String, String>::new(),
        );
    }
}
