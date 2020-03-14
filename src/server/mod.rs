//! This module provides WarpGrapher servers, including supporting modules for
//! configuration, GraphQL schema generation, resolvers, and interface to the
//! database.

use super::error::{Error, ErrorKind};
use actix::System;
use actix_cors::Cors;
use actix_web::dev;
use actix_web::error::ErrorInternalServerError;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use config::{WarpgrapherConfig, WarpgrapherProp, WarpgrapherResolvers, WarpgrapherValidators};
use context::GraphQLContext;
use context::WarpgrapherRequestContext;
use database::DatabasePool;
use extensions::WarpgrapherExtensions;
use futures::executor::block_on;
use juniper::http::playground::playground_source;
use juniper::http::GraphQLRequest;
use log::{debug, error, trace};
use schema::{create_root_node, RootRef};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use serde_json;
use serde_json::json;
use std::collections::HashMap;
use std::env::var_os;
use std::fmt::Debug;
use std::option::Option;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread::{spawn, JoinHandle};

pub mod config;
pub mod context;
pub mod database;
pub mod extensions;
mod headers;
pub mod objects;
mod resolvers;
pub mod schema;
mod visitors;

pub fn bind_port_from_env(env_name: &str) -> String {
    let default = "5000";
    var_os(env_name)
        .unwrap_or_else(|| default.to_string().into())
        .to_str()
        .unwrap_or(default)
        .to_string()
}

pub fn bind_addr_from_env(env_name: &str) -> String {
    let default = "127.0.0.1";
    var_os(env_name)
        .unwrap_or_else(|| default.to_string().into())
        .to_str()
        .unwrap_or(default)
        .to_string()
}

#[allow(clippy::borrowed_box)]
fn graphql_error(err: &Box<dyn std::error::Error + Send + Sync>) -> String {
    match serde_json::to_string(&json!({ "message": format!("{}", err) })) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to serialize error object:  {:#?}", e);
            "INTERNAL SERVER ERROR".to_string()
        }
    }
}

impl WarpgrapherRequestContext for () {
    fn new() {}
}

#[derive(Clone)]
struct AppData<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: WarpgrapherRequestContext,
{
    gql_endpoint: String,
    pool: DatabasePool,
    root_node: RootRef<GlobalCtx, ReqCtx>,
    resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
    validators: WarpgrapherValidators,
    extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    global_ctx: Option<GlobalCtx>,
    version: Option<String>,
}

impl<GlobalCtx, ReqCtx> AppData<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: WarpgrapherRequestContext,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        gql_endpoint: String,
        pool: DatabasePool,
        root_node: RootRef<GlobalCtx, ReqCtx>,
        resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
        validators: WarpgrapherValidators,
        extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
        global_ctx: Option<GlobalCtx>,
        version: Option<String>,
    ) -> AppData<GlobalCtx, ReqCtx> {
        AppData {
            gql_endpoint,
            pool,
            root_node,
            resolvers,
            validators,
            extensions,
            global_ctx,
            version,
        }
    }
}

/// A WarpGrapher GraphQL server.
///
/// The [`Server`] struct wraps an Actix web server and a Juniper GraphQL service
/// on top of it, with an auto-generated set of resolvers that cover basic CRUD
/// operations, and potentially custom resolvers, on a set of data types and
/// the relationships between them.  The server includes handling of back-end
/// communications with Neo4J.
///
/// [`Server`]: ./struct.Server.html
///
/// # Examples
///
/// ```rust
/// use warpgrapher::Server;
/// use warpgrapher::server::config::WarpgrapherConfig;
/// use warpgrapher::server::{bind_port_from_env};
/// use warpgrapher::server::database::DatabasePool;
///
/// let config = WarpgrapherConfig::new(1, Vec::new(), Vec::new());
///
/// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
///     .build().unwrap();
///
/// server.serve(false);
/// server.shutdown();
/// ```
pub struct Server<GlobalCtx = (), ReqCtx = ()>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + WarpgrapherRequestContext,
{
    pub config: WarpgrapherConfig,
    pub db_pool: DatabasePool,
    pub global_ctx: Option<GlobalCtx>,
    pub resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
    pub validators: WarpgrapherValidators,
    pub extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    pub bind_addr: String,
    pub bind_port: String,
    pub graphql_endpoint: String,
    pub playground_endpoint: Option<String>,
    pub version: Option<String>,
    root_node: Option<RootRef<GlobalCtx, ReqCtx>>,
    handle: Option<JoinHandle<()>>,
    server: Option<dev::Server>,
}

impl<GlobalCtx, ReqCtx> Server<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + WarpgrapherRequestContext,
{
    /// Creates a new [`Server`], with required parameters config and database
    /// and allows optional parameters to be added using a builder pattern.
    ///
    /// [`Server`]: ./struct.Server.html
    /// [`WarpgrapherConfiguration`]: ./config/struct.WarpgrapherConfiguration.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::new(1, Vec::new(), Vec::new());
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn new(config: WarpgrapherConfig, db_pool: DatabasePool) -> Server<GlobalCtx, ReqCtx> {
        Server {
            config,
            db_pool,
            global_ctx: None,
            resolvers: HashMap::new(),
            validators: HashMap::new(),
            extensions: vec![],
            bind_addr: "127.0.0.1".to_string(),
            bind_port: "5000".to_string(),
            graphql_endpoint: "/graphql".to_string(),
            playground_endpoint: None,
            version: None,
            root_node: None,
            handle: None,
            server: None,
        }
    }

    /// Adds a global context to the server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     global_var: String    
    /// }
    ///
    /// let global_ctx = AppGlobalCtx { global_var: "Hello World".to_owned() };
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<AppGlobalCtx, ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_global_ctx(global_ctx)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_global_ctx(mut self, global_ctx: GlobalCtx) -> Server<GlobalCtx, ReqCtx> {
        self.global_ctx = Some(global_ctx);
        self
    }

    /// Adds resolvers to the server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::{Server, WarpgrapherResolvers};
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let resolvers = WarpgrapherResolvers::<(), ()>::new();
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_resolvers(resolvers)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_resolvers(
        mut self,
        resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
    ) -> Server<GlobalCtx, ReqCtx> {
        self.resolvers = resolvers;
        self
    }

    /// Adds validators to the server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::{Server, WarpgrapherValidators};
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let validators = WarpgrapherValidators::new();
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_validators(validators)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_validators(
        mut self,
        validators: WarpgrapherValidators,
    ) -> Server<GlobalCtx, ReqCtx> {
        self.validators = validators;
        self
    }

    /// Adds extensions to server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::{Server, WarpgrapherExtensions};
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let extensions = WarpgrapherExtensions::<(), ()>::new();
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_extensions(extensions)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_extensions(
        mut self,
        extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    ) -> Server<GlobalCtx, ReqCtx> {
        self.extensions = extensions;
        self
    }

    /// Sets bind address on server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_addr("127.0.0.1".to_owned())
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_bind_addr(mut self, bind_addr: String) -> Server<GlobalCtx, ReqCtx> {
        self.bind_addr = bind_addr;
        self
    }

    /// Sets bind port on server
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn with_bind_port(mut self, bind_port: String) -> Server<GlobalCtx, ReqCtx> {
        self.bind_port = bind_port;
        self
    }

    /// Sets the endpoint that will handle the graphql queries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_graphql_endpoint("/graphql".to_owned())
    ///     .build().unwrap();
    /// ```
    pub fn with_graphql_endpoint(mut self, graphql_endpoint: String) -> Server<GlobalCtx, ReqCtx> {
        self.graphql_endpoint = graphql_endpoint;
        self
    }

    /// Sets the endpoint where the UI playground in hosted
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_playground_endpoint("/playground".to_owned())
    ///     .build().unwrap();
    /// ```
    pub fn with_playground_endpoint(
        mut self,
        playground_endpoint: String,
    ) -> Server<GlobalCtx, ReqCtx> {
        self.playground_endpoint = Some(playground_endpoint);
        self
    }

    /// Sets the version of the app
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_version("1.0.0".to_owned())
    ///     .build().unwrap();
    /// ```
    pub fn with_version(mut self, version: String) -> Server<GlobalCtx, ReqCtx> {
        self.version = Some(version);
        self
    }

    /// Builds a configured [`Server`] including generateing the data model, CRUD operations,
    /// and custom endpoints from the [`WarpgrapherConfiguration`] `c`. Sets
    /// the GraphQL endpoint to `/graphql` and the GraphIQL endpoint to
    /// `/graphiql`. Returns the [`Server`].
    ///
    /// [`Server`]: ./struct.Server.html
    /// [`WarpgrapherConfiguration`]: ./config/struct.WarpgrapherConfiguration.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of kind [`CouldNotResolveWarpgrapherType`] if
    /// there is an error in the configuration, specifically if the
    /// configuration of type A references type B, but type B cannot be found.
    ///
    /// [`Error`]: ../error/struct.Error.html
    /// [`CouldNotResolveWarpgrapherType`]: ../error/enum.ErrorKind.html#variant.CouldNotResolveWarpgrapherType
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::new(1, Vec::new(), Vec::new());
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    /// ```
    pub fn build(mut self) -> Result<Server<GlobalCtx, ReqCtx>, Error> {
        // validate server options
        match Server::<GlobalCtx, ReqCtx>::validate_server(
            &self.resolvers,
            &self.validators,
            &self.config,
        ) {
            Ok(_) => (),
            Err(e) => return Err(e),
        }

        // create graphql root node
        self.root_node = Some(create_root_node(&self.config)?);

        Ok(self)
    }

    fn validate_server(
        resolvers: &WarpgrapherResolvers<GlobalCtx, ReqCtx>,
        validators: &WarpgrapherValidators,
        config: &WarpgrapherConfig,
    ) -> Result<(), Error> {
        match config.validate() {
            Ok(_) => (),
            Err(e) => {
                println!("Config validation failed: {:#?}", e);
                return Err(e);
            }
        };

        //Validate Custom Endpoint defined in WarpgrapherConfig exists as a WarpgrapherResolver
        for e in config.endpoints.iter() {
            if !resolvers.contains_key(&e.name) {
                return Err(Error::new(
                    ErrorKind::ResolverNotFound(
                        format!("Server could not find a Resolver for the Custom Endpoint: {endpoint_name}.", endpoint_name=e.name),
                        e.name.clone()
                    ),
                    None,
                ));
            }
        }

        //Validate Custom Prop defined in WarpgrapherConfig exists as a WarpgrapherResolver
        let mut dyn_scalar_props: Vec<WarpgrapherProp> = Vec::new();
        let mut props_with_validator: Vec<WarpgrapherProp> = Vec::new();

        for m in config.model.iter() {
            for p in m.props.iter() {
                p.resolver
                    .as_ref()
                    .map_or((), |_| dyn_scalar_props.push(p.clone()));
                p.validator
                    .as_ref()
                    .map_or((), |_| props_with_validator.push(p.clone()));
            }
        }
        for dsp in dyn_scalar_props.iter() {
            let resolver_name = dsp.resolver.as_ref().ok_or_else(|| {
                Error::new(
                    ErrorKind::ResolverNotFound(
                        format!(
                            "Failed to resolve custom prop: {prop_name}. Missing resolver name.",
                            prop_name = dsp.name
                        ),
                        dsp.name.to_string(),
                    ),
                    None,
                )
            })?;
            if !resolvers.contains_key(resolver_name) {
                return Err(Error::new(
                    ErrorKind::ResolverNotFound(
                        format!(
                            "Server could not find a resolver for the custom prop: {prop_name}.",
                            prop_name = dsp.name
                        ),
                        dsp.name.clone(),
                    ),
                    None,
                ));
            }
        }

        //Validate Custom Input Validator defined in WarpgrapherConfig exists as WarpgrapherValidator
        for pwv in props_with_validator.iter() {
            let validator_name = pwv.validator.as_ref().ok_or_else(|| {
                Error::new(
                    ErrorKind::ValidatorNotFound(
                        format!(
                            "Failed to find custom validator for prop: {prop_name}.",
                            prop_name = pwv.name
                        ),
                        pwv.name.to_string(),
                    ),
                    None,
                )
            })?;
            if !validators.contains_key(validator_name) {
                return Err(Error::new(
                    ErrorKind::ValidatorNotFound(
                        format!(
                            "Server could not find a validator for the custom prop: {prop_name}.",
                            prop_name = pwv.name
                        ),
                        pwv.name.clone(),
                    ),
                    None,
                ));
            }
        }

        // validation passed
        Ok(())
    }

    async fn graphql(
        data: Data<AppData<GlobalCtx, ReqCtx>>,
        req: Json<GraphQLRequest>,
        headers: headers::Headers,
    ) -> impl Responder {
        debug!("\nRequest: {:#?}\n", req);

        // initialize empty request context
        let mut req_ctx = ReqCtx::new();

        // run pre request plugin hooks
        for extension in &data.extensions {
            match &extension.pre_request_hook(
                data.global_ctx.clone(),
                Some(&mut req_ctx),
                &headers.data,
            ) {
                Ok(_) => {}
                Err(e) => {
                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(graphql_error(e));
                }
            }
        }

        // execute graphql query
        let res = req.execute(
            &data.root_node,
            &GraphQLContext::<GlobalCtx, ReqCtx>::new(
                data.pool.clone(),
                data.resolvers.clone(),
                data.validators.clone(),
                data.extensions.clone(),
                data.global_ctx.clone(),
                Some(req_ctx.clone()),
                data.version.clone(),
            ),
        );

        // convert graphql response (json) to mutable serde_json::Value
        let res_str: String = match serde_json::to_string(&res) {
            Ok(s) => s,
            Err(e) => return ErrorInternalServerError(e).into(),
        };
        let mut res_value: serde_json::Value = match serde_json::from_str(&res_str) {
            Ok(v) => v,
            Err(e) => return ErrorInternalServerError(e).into(),
        };

        // run post request plugin hooks
        for extension in &data.extensions {
            match &extension.post_request_hook(
                data.global_ctx.clone(),
                Some(&req_ctx),
                &mut res_value,
            ) {
                Ok(_) => {}
                Err(e) => {
                    return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(graphql_error(e));
                }
            }
        }

        // convert graphql response to string
        let body = match serde_json::to_string(&res_value) {
            Ok(b) => b,
            Err(e) => return ErrorInternalServerError(e).into(),
        };

        HttpResponse::Ok()
            .content_type("application/json")
            .body(body)
    }

    async fn graphiql(data: Data<AppData<GlobalCtx, ReqCtx>>) -> impl Responder {
        let html = playground_source(&data.gql_endpoint);
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    }

    /// Binds the server to `addr` and begins listening for and answering
    /// requests.  If the `block` boolean is `true`, the server blocks the main
    /// thread, requiring the program to be killed to exit. If `block` is
    /// `false`, the server launches in a separate thread and this function
    /// returns `()` immediately. The server may then be shut down gracefully
    /// by calling [`shutdown`].
    ///
    /// [`shutdown`]: #method.shutdown
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of kind [`ServerAlreadyRunning`] if [`serve`] was
    /// already called to start a non-blocking server.
    ///
    /// Returns an [`Error`] of kind [`AddrInUse`] if there is already another
    /// server bound to the port as a listener.
    ///
    /// Returns an [`Error`] of kind [`AddrNotAvailable`] if the address
    /// provided cannot be bound, for example if it is not local.
    ///
    /// Returns an [`Error`] of kind [`ServerStartupFailed`] if there is an
    /// internal error in setting up the server. Receiving this error would
    /// likely mean a bug in WarpGrapher.
    ///
    /// [`Error`]: ../error/struct.Error.html
    /// [`ServerAlreadyRunning`]: ../error/enum.ErrorKind.html#variant.ServerAlreadyRunning
    /// [`AddrInUse`]: ../error/enum.ErrorKind.html#variant.AddrInUse
    /// [`AddrNotAvailable`]: ../error/enum.ErrorKind.html#variant.AddrNotAvailable
    /// [`ServerStartupFailed`]: ../error/enum.ErrorKind.html#variant.ServerStartupFailed
    /// [`serve`]: #method.serve
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::default();
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    ///
    /// server.serve(false);
    /// server.shutdown();
    /// ```
    pub fn serve(&mut self, block: bool) -> Result<(), Error> {
        trace!("Server::serve, block: {:#?}", block);

        if self.handle.is_some() || self.server.is_some() {
            return Err(Error::new(ErrorKind::ServerAlreadyRunning, None));
        }

        let (tx, rx) = mpsc::channel();
        let addr = format!("{}:{}", self.bind_addr, self.bind_port);

        if block {
            Server::start(
                self.db_pool.clone(),
                addr,
                self.graphql_endpoint.clone(),
                self.playground_endpoint.clone(),
                match &self.root_node {
                    Some(rn) => rn.clone(),
                    None => panic!("Missing root node "),
                },
                self.resolvers.clone(),
                self.validators.clone(),
                self.extensions.clone(),
                match &self.global_ctx {
                    Some(value) => Some(value.clone()),
                    None => None,
                },
                tx,
                self.version.clone(),
            );
        } else {
            let db_pool = self.db_pool.clone();
            let graphql_endpoint = self.graphql_endpoint.clone();
            let playground_endpoint = self.playground_endpoint.clone();
            let root_node = self.root_node.clone();
            let resolvers = self.resolvers.clone();
            let validators = self.validators.clone();
            let extensions = self.extensions.clone();
            let global_ctx = match &self.global_ctx {
                Some(value) => Some(value.clone()),
                None => None,
            };
            let version = self.version.clone();

            self.handle = Some(spawn(move || {
                Server::start(
                    db_pool,
                    addr,
                    graphql_endpoint,
                    playground_endpoint,
                    match root_node {
                        Some(rn) => rn,
                        None => panic!("Missing root node"),
                    },
                    resolvers,
                    validators,
                    extensions,
                    global_ctx,
                    tx,
                    version,
                )
            }));
        }

        rx.recv()
            .map_err(|e| Error::new(ErrorKind::ServerStartupFailed(e), None))
            .and_then(|m| match m {
                Ok(server) => {
                    self.server = Some(server);
                    Ok(())
                }
                Err(e) => match self.handle.take() {
                    Some(h) => {
                        let _ = h.join();
                        Err(e)
                    }
                    None => Err(e),
                },
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn start(
        db_pool: DatabasePool,
        addr: String,
        graphql_endpoint: String,
        playground_endpoint: Option<String>,
        root_node: RootRef<GlobalCtx, ReqCtx>,
        resolvers: WarpgrapherResolvers<GlobalCtx, ReqCtx>,
        validators: WarpgrapherValidators,
        extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
        global_ctx: Option<GlobalCtx>,
        tx: Sender<Result<dev::Server, Error>>,
        version: Option<String>,
    ) {
        let sys = System::new("warpgrapher");

        let app_data = AppData::new(
            graphql_endpoint.clone(),
            db_pool,
            root_node.clone(),
            resolvers,
            validators,
            extensions,
            global_ctx,
            version,
        );

        let _ = HttpServer::new(move || {
            let app = App::new()
                .data(app_data.clone())
                .wrap(Logger::default())
                .wrap(Cors::default())
                .service(
                    web::resource(&graphql_endpoint.clone())
                        .route(web::post().to(Server::<GlobalCtx, ReqCtx>::graphql)),
                );

            if let Some(endpoint) = playground_endpoint.clone() {
                app.service(
                    web::resource(&endpoint)
                        .route(web::get().to(Server::<GlobalCtx, ReqCtx>::graphiql)),
                )
            } else {
                app
            }
        })
        .bind(&addr)
        .map_err(|e| {
            trace!("Error spawning server: {:?}", e);
            let k = match e.kind() {
                std::io::ErrorKind::AddrInUse => ErrorKind::AddrInUse(e),
                _ => ErrorKind::AddrNotAvailable(e),
            };
            let _ = tx.send(Err(Error::new(k, None)));
        })
        .and_then(|srv| {
            let server = srv.system_exit().run();
            let _ = tx.send(Ok(server));
            let _ = sys.run();
            Ok(())
        });
    }

    /// Shuts down a server previously started using the ['serve']:
    /// #method.serve method with a block argument of false.
    ///
    /// Returns a result of Ok(()) if the server is shutdown successfully.
    /// Returns an error of kind ['ServerNotStarted']:
    /// ../enum.ServerNotStarted.html if there was no prior call to ['serve']:
    /// #method.serve to start the server. Returns an error of kind
    /// ['ServerShutdownFailed']: ../enum.ServerShutdownFailed.html if the
    /// server could not be shutdown successfully. This shouldn't happen, as it
    /// would indicate, for example, that the signal flag used to signal to the
    /// server to shutdown was deallocated prior to shutting down the server, or
    /// that the server thread panicked prior to shutting down in an orderly
    /// fashion.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::Server;
    /// use warpgrapher::server::config::WarpgrapherConfig;
    /// use warpgrapher::server::{bind_port_from_env};
    /// use warpgrapher::server::database::DatabasePool;
    ///
    /// let config = WarpgrapherConfig::new(1, Vec::new(), Vec::new());
    ///
    /// let mut server = Server::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_bind_port(bind_port_from_env("WG_BIND_PORT"))
    ///     .build().unwrap();
    ///
    /// server.serve(false);
    /// server.shutdown();
    /// ```
    pub fn shutdown(&mut self) -> Result<(), Error> {
        trace!("Server::shutdown called.");

        let s = self
            .server
            .take()
            .ok_or_else(|| Error::new(ErrorKind::ServerNotRunning, None))?;
        let h = self
            .handle
            .take()
            .ok_or_else(|| Error::new(ErrorKind::ServerNotRunning, None))?;

        block_on(s.stop(true));

        h.join()
            .map_err(|e| {
                error!("Error shutting down server. {:?}", e);
                Error::new(ErrorKind::ServerShutdownFailed, None)
            })
            .and_then(|_| Ok(()))
    }
}

/// Notably, the unit tests here likely seem weak. This is because testing most
/// of the functionality requires a database container to be running and
/// reachable, so most of the coverage is provided by integration tests.
#[cfg(test)]
mod tests {
    use super::config::{WarpgrapherConfig, WarpgrapherResolvers, WarpgrapherValidators};
    use super::context::GraphQLContext;
    use super::schema::Info;
    use super::Server;
    use crate::error::Error;
    #[cfg(feature = "neo4j")]
    use crate::server::database::neo4j::Neo4jEndpoint;
    #[cfg(feature = "neo4j")]
    use crate::server::database::DatabaseEndpoint;
    use juniper::{Arguments, ExecutionResult, Executor, Value};
    use serde_json;
    use std::fs::File;
    use std::io::BufReader;

    #[cfg(feature = "neo4j")]
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[allow(dead_code)]
    pub fn test_config() -> WarpgrapherConfig {
        let cf = File::open("tests/fixtures/config.yml")
            .expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    #[allow(dead_code)]
    pub fn load_config(config: &str) -> WarpgrapherConfig {
        let cf = File::open(config).expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    /// Passes if the server can be created.
    #[cfg(feature = "neo4j")]
    #[test]
    fn server_new() {
        init();

        let config = load_config("tests/fixtures/config_minimal.yml");
        let _server = Server::<(), ()>::new(
            config,
            Neo4jEndpoint::from_env().unwrap().get_pool().unwrap(),
        )
        .build()
        .unwrap();
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_server_validate_minimal() {
        //No prop resolver in config
        //No endpoint resolver in config
        //No validator in config
        //No resolver defined
        //No validator defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_ok.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let validators = WarpgrapherValidators::new();
        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_server_validate_custom_validators() {
        //Validator defined
        //No validator in config
        //is_ok
        let config = load_config("tests/fixtures/config_minimal.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let mut validators = WarpgrapherValidators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());

        //Validator defined
        //Validator in config
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_validator.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let mut validators = WarpgrapherValidators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());

        //Validator not defined
        //validator in config
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_validator.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_err());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_server_validate_custom_endpoint() {
        //No endpoint resolvers in config
        //No resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_ok.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());

        //Endpoint resolver in config
        //No resolver defined
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_resolver.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_err());

        //Endpoint resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_resolver.yml");
        let mut resolvers = WarpgrapherResolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_server_validate_custom_prop() {
        //Prop resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_prop_resolver.yml");
        let mut resolvers = WarpgrapherResolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());

        //No prop resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/config_minimal.yml");
        let mut resolvers = WarpgrapherResolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_ok());

        //Prop resolver in config
        //No resolver defined
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_prop_resolver.yml");
        let resolvers = WarpgrapherResolvers::<(), ()>::new();
        let validators = WarpgrapherValidators::new();

        assert!(Server::<(), ()>::validate_server(&resolvers, &validators, &config).is_err());
    }

    pub fn my_resolver(
        _info: &Info,
        _args: &Arguments,
        _executor: &Executor<GraphQLContext<(), ()>>,
    ) -> ExecutionResult {
        Ok(Value::scalar(100 as i32))
    }

    pub fn my_validator(_value: &serde_json::Value) -> Result<(), Error> {
        Ok(())
    }
}
