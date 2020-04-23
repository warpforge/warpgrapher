//! This module provides the Warpgrapher engine, including supporting modules for
//! configuration, GraphQL schema generation, resolvers, and interface to the
//! database.

use actix_web::web::Json;

use super::error::{Error, ErrorKind};
use config::{Config, Prop, Validators};
use context::{GraphQLContext, RequestContext};
use extensions::WarpgrapherExtensions;
use juniper::http::GraphQLRequest;
use log::debug;
use r2d2::Pool;
use r2d2_cypher::CypherConnectionManager;
use resolvers::Resolvers;
use schema::{create_root_node, RootRef};
use serde_json;
use std::collections::HashMap;
use std::env::var_os;
use std::fmt::Debug;
use std::option::Option;

pub mod config;
pub mod context;
pub mod extensions;
pub mod neo4j;
pub mod objects;
pub mod resolvers;
pub mod schema;
pub mod visitors;

impl RequestContext for () {
    fn new() {}
}

#[derive(Clone)]
pub struct EngineBuilder<GlobalCtx = (), ReqCtx = ()>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    pub config: Config,
    pub database: String,
    pub global_ctx: Option<GlobalCtx>,
    pub resolvers: Resolvers<GlobalCtx, ReqCtx>,
    pub validators: Validators,
    pub extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    pub version: Option<String>,
}

impl<GlobalCtx, ReqCtx> EngineBuilder<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    /// Adds a global context to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::Config;
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     global_var: String    
    /// }
    ///
    /// let global_ctx = AppGlobalCtx { global_var: "Hello World".to_owned() };
    ///
    /// let config = Config::default();
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<AppGlobalCtx, ()>::new(config, db)
    ///     .with_global_ctx(global_ctx)
    ///     .build().unwrap();
    /// ```
    pub fn with_global_ctx(mut self, global_ctx: GlobalCtx) -> EngineBuilder<GlobalCtx, ReqCtx> {
        self.global_ctx = Some(global_ctx);
        self
    }

    /// Adds resolvers to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::{Config};
    /// use warpgrapher::engine::resolvers::{Resolvers};
    ///
    /// let resolvers = Resolvers::<(), ()>::new();
    ///
    /// let config = Config::default();
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, db)
    ///     .with_resolvers(resolvers)
    ///     .build().unwrap();
    /// ```
    pub fn with_resolvers(
        mut self,
        resolvers: Resolvers<GlobalCtx, ReqCtx>,
    ) -> EngineBuilder<GlobalCtx, ReqCtx> {
        self.resolvers = resolvers;
        self
    }

    /// Adds validators to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::{Config, Validators};
    ///
    /// let validators = Validators::new();
    ///
    /// let config = Config::default();
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, db)
    ///     .with_validators(validators)
    ///     .build().unwrap();
    /// ```
    pub fn with_validators(mut self, validators: Validators) -> EngineBuilder<GlobalCtx, ReqCtx> {
        self.validators = validators;
        self
    }

    /// Adds extensions to engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::{Config, Validators};
    /// use warpgrapher::engine::extensions::WarpgrapherExtensions;
    ///
    /// let extensions = WarpgrapherExtensions::<(), ()>::new();
    ///
    /// let config = Config::default();
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, db)
    ///     .with_extensions(extensions)
    ///     .build().unwrap();
    /// ```
    pub fn with_extensions(
        mut self,
        extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    ) -> EngineBuilder<GlobalCtx, ReqCtx> {
        self.extensions = extensions;
        self
    }

    /// Sets the version of the app
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::Config;
    ///
    /// let config = Config::default();
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, db)
    ///     .with_version("1.0.0".to_owned())
    ///     .build().unwrap();
    /// ```
    pub fn with_version(mut self, version: String) -> EngineBuilder<GlobalCtx, ReqCtx> {
        self.version = Some(version);
        self
    }

    /// Builds a configured [`Engine`] including generateing the data model, CRUD operations,
    /// and custom endpoints from the [`Configuration`] `c`.
    /// Returns the [`Engine`].
    ///
    /// [`Engine`]: ./struct.Engine.html
    /// [`Configuration`]: ./config/struct.Configuration.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] of kind [`CouldNotResolveType`] if
    /// there is an error in the configuration, specifically if the
    /// configuration of type A references type B, but type B cannot be found.
    ///
    /// [`Error`]: ../error/struct.Error.html
    /// [`CouldNotResolveType`]: ../error/enum.ErrorKind.html#variant.CouldNotResolveType
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::Config;
    ///
    /// let config = Config::new(1, Vec::new(), Vec::new());
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<()>::new(config, db)
    ///     .build().unwrap();
    /// ```
    pub fn build(self) -> Result<Engine<GlobalCtx, ReqCtx>, Error> {
        let manager = CypherConnectionManager {
            url: self.database.clone(),
        };

        let pool = match r2d2::Pool::builder().max_size(5).build(manager) {
            Ok(p) => p,
            Err(e) => return Err(Error::new(ErrorKind::CouldNotBuildCypherPool(e), None)),
        };

        // validate engine options
        match EngineBuilder::validate_engine(&self.resolvers, &self.validators, &self.config) {
            Ok(_) => (),
            Err(e) => return Err(e),
        };

        let root_node = create_root_node(&self.config)?;

        let engine = Engine::<GlobalCtx, ReqCtx> {
            config: self.config.clone(),
            database: self.database,
            pool,
            global_ctx: self.global_ctx,
            resolvers: self.resolvers,
            validators: self.validators,
            extensions: self.extensions,
            version: self.version,
            root_node,
        };

        Ok(engine)
    }

    fn validate_engine(
        resolvers: &Resolvers<GlobalCtx, ReqCtx>,
        validators: &Validators,
        config: &Config,
    ) -> Result<(), Error> {
        match config.validate() {
            Ok(_) => (),
            Err(e) => {
                println!("Config validation failed: {:#?}", e);
                return Err(e);
            }
        };

        //Validate Custom Endpoint defined in Config exists as a Resolver
        for e in config.endpoints.iter() {
            if !resolvers.contains_key(&e.name) {
                return Err(Error::new(
                    ErrorKind::ResolverNotFound(
                        format!("Engine could not find a Resolver for the Custom Endpoint: {endpoint_name}.", endpoint_name=e.name),
                        e.name.clone()
                    ),
                    None,
                ));
            }
        }

        //Validate Custom Prop defined in Config exists as a Resolver
        let mut dyn_scalar_props: Vec<Prop> = Vec::new();
        let mut props_with_validator: Vec<Prop> = Vec::new();

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
                            "Engine could not find a resolver for the custom prop: {prop_name}.",
                            prop_name = dsp.name
                        ),
                        dsp.name.clone(),
                    ),
                    None,
                ));
            }
        }

        //Validate Custom Input Validator defined in Config exists as Validator
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
                            "Engine could not find a validator for the custom prop: {prop_name}.",
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
}

/// A Warpgrapher GraphQL engine.
///
/// The [`Engine`] struct Juniper GraphQL service
/// on top of it, with an auto-generated set of resolvers that cover basic CRUD
/// operations, and potentially custom resolvers, on a set of data types and
/// the relationships between them.  The engine includes handling of back-end
/// communications with the chosen databse.
///
/// [`Engine`]: ./struct.Engine.html
///
/// # Examples
///
/// ```rust
/// use warpgrapher::engine::Engine;
/// use warpgrapher::engine::neo4j::Neo4jEndpoint;
/// use warpgrapher::engine::config::Config;
///
/// let config = Config::default();
/// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
///
/// let mut engine = Engine::<(), ()>::new(config, db)
///     .build().unwrap();
///
/// ```
#[derive(Clone)]
pub struct Engine<GlobalCtx = (), ReqCtx = ()>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    pub config: Config,
    pub database: String,
    pub pool: Pool<CypherConnectionManager>,
    pub global_ctx: Option<GlobalCtx>,
    pub resolvers: Resolvers<GlobalCtx, ReqCtx>,
    pub validators: Validators,
    pub extensions: WarpgrapherExtensions<GlobalCtx, ReqCtx>,
    pub version: Option<String>,
    root_node: RootRef<GlobalCtx, ReqCtx>,
}

impl<GlobalCtx, ReqCtx> Engine<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + RequestContext,
{
    /// Creates a new [`Engine`], with required parameters config and database
    /// and allows optional parameters to be added using a builder pattern.
    ///
    /// [`Engine`]: ./struct.Engine.html
    /// [`Configuration`]: ./config/struct.Configuration.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::neo4j::Neo4jEndpoint;
    /// use warpgrapher::engine::config::Config;
    ///
    /// let config = Config::new(1, Vec::new(), Vec::new());
    /// let db = Neo4jEndpoint::from_env("DB_URL").unwrap();
    ///
    /// let mut engine = Engine::<()>::new(config, db)
    ///     .build().unwrap();
    /// ```

    #[allow(clippy::new_ret_no_self)]
    pub fn new(config: Config, database: String) -> EngineBuilder<GlobalCtx, ReqCtx> {
        EngineBuilder::<GlobalCtx, ReqCtx> {
            config,
            database,
            global_ctx: None,
            resolvers: HashMap::new(),
            validators: HashMap::new(),
            extensions: vec![],
            version: None,
        }
    }

    pub fn execute(
        &self,
        req: Json<GraphQLRequest>, //TODO make generic
        metadata: HashMap<String, String>,
    ) -> Result<String, Error> {
        debug!("\nRequest: {:#?}\n", req);

        // initialize empty request context
        let mut req_ctx = ReqCtx::new();

        // run pre request plugin hooks
        for extension in &self.extensions {
            match extension.pre_request_hook(self.global_ctx.clone(), Some(&mut req_ctx), &metadata)
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(Error::new(ErrorKind::PreRequestHookExtensionError(e), None));
                }
            }
        }

        // execute graphql query
        let res = req.execute(
            &self.root_node,
            &GraphQLContext::<GlobalCtx, ReqCtx>::new(
                self.pool.clone(),
                self.resolvers.clone(),
                self.validators.clone(),
                self.extensions.clone(),
                self.global_ctx.clone(),
                Some(req_ctx.clone()),
                self.version.clone(),
            ),
        );

        // convert graphql response (json) to mutable serde_json::Value
        let res_str: String = match serde_json::to_string(&res) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::JsonStringConversionFailed(e), None)),
        };
        let mut res_value: serde_json::Value = match serde_json::from_str(&res_str) {
            Ok(v) => v,
            Err(e) => return Err(Error::new(ErrorKind::JsonStringConversionFailed(e), None)),
        };

        // run post request plugin hooks
        for extension in &self.extensions {
            match extension.post_request_hook(
                self.global_ctx.clone(),
                Some(&req_ctx),
                &mut res_value,
            ) {
                Ok(_) => {}
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::PostRequestHookExtensionError(e),
                        None,
                    ));
                }
            }
        }

        // convert graphql response to string
        let body = match serde_json::to_string(&res_value) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::JsonStringConversionFailed(e), None)),
        };

        Ok(body)
    }
}

/// Notably, the unit tests here likely seem weak. This is because testing most
/// of the functionality requires a database container to be running and
/// reachable, so most of the coverage is provided by integration tests.
#[cfg(test)]
mod tests {
    use super::config::{Config, Validators};
    use super::resolvers::{ResolverContext, Resolvers};
    use super::Engine;
    use super::EngineBuilder;
    use crate::error::Error;
    use juniper::ExecutionResult;
    use std::env::var_os;
    use std::fs::File;
    use std::io::BufReader;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[allow(dead_code)]
    pub fn test_config() -> Config {
        let cf = File::open("tests/fixtures/config.yml")
            .expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    #[allow(dead_code)]
    pub fn load_config(config: &str) -> Config {
        let cf = File::open(config).expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    /// Passes if the engine can be created.
    #[test]
    fn engine_new() {
        init();

        let db_url = match var_os("DB_URL") {
            None => "http://neo4j:testpass@127.0.0.1:7474/db/data".to_owned(),
            Some(os) => os
                .to_str()
                .unwrap_or("http://neo4j:testpass@127.0.0.1:7474/db/data")
                .to_owned(),
        };
        let config = load_config("tests/fixtures/config_minimal.yml");
        let _engine = Engine::<(), ()>::new(config, db_url).build().unwrap();
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_engine_validate_minimal() {
        //No prop resolver in config
        //No endpoint resolver in config
        //No validator in config
        //No resolver defined
        //No validator defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_ok.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let validators = Validators::new();
        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_engine_validate_custom_validators() {
        //Validator defined
        //No validator in config
        //is_ok
        let config = load_config("tests/fixtures/config_minimal.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());

        //Validator defined
        //Validator in config
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_validator.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());

        //Validator not defined
        //validator in config
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_validator.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_err());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_engine_validate_custom_endpoint() {
        //No endpoint resolvers in config
        //No resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_ok.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());

        //Endpoint resolver in config
        //No resolver defined
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_resolver.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_err());

        //Endpoint resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_resolver.yml");
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());
    }

    #[test]
    #[allow(clippy::match_wild_err_arm)]
    fn test_engine_validate_custom_prop() {
        //Prop resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/test_config_with_custom_prop_resolver.yml");
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());

        //No prop resolver in config
        //Resolver defined
        //is_ok
        let config = load_config("tests/fixtures/config_minimal.yml");
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_ok());

        //Prop resolver in config
        //No resolver defined
        //is_err
        let config = load_config("tests/fixtures/test_config_with_custom_prop_resolver.yml");
        let resolvers = Resolvers::<(), ()>::new();
        let validators = Validators::new();

        assert!(EngineBuilder::validate_engine(&resolvers, &validators, &config).is_err());
    }

    pub fn my_resolver(context: ResolverContext<(), ()>) -> ExecutionResult {
        context.return_scalar(1 as i32)
    }

    pub fn my_validator(_value: &serde_json::Value) -> Result<(), Error> {
        Ok(())
    }
}

pub fn try_from_env(env_name: &str, default: String) -> String {
    match var_os(env_name) {
        None => default,
        Some(os) => os.to_str().unwrap_or(&default).to_owned(),
    }
}
