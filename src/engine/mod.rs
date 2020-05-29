//! This module provides the Warpgrapher engine, including supporting modules for
//! configuration, GraphQL schema generation, resolvers, and interface to the
//! database.

use super::error::Error;
use config::{Config, Prop, Validators};
use context::{GlobalContext, GraphQLContext, RequestContext};
use database::DatabasePool;
use extensions::Extensions;
use juniper::http::GraphQLRequest;
use log::debug;
use objects::resolvers::Resolvers;
use schema::{create_root_node, RootRef};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::option::Option;

pub mod config;
pub mod context;
pub mod database;
pub mod extensions;
pub mod objects;
pub mod schema;
pub mod value;

#[derive(Clone)]
pub struct EngineBuilder<GlobalCtx = (), RequestCtx = ()>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    config: Config,
    db_pool: DatabasePool,
    global_ctx: Option<GlobalCtx>,
    resolvers: Resolvers<GlobalCtx, RequestCtx>,
    validators: Validators,
    extensions: Extensions<GlobalCtx, RequestCtx>,
    version: Option<String>,
}

impl<GlobalCtx, RequestCtx> EngineBuilder<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    /// Adds a global context to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::env::var_os;
    /// use warpgrapher::engine::Engine;
    /// use warpgrapher::engine::config::Config;
    /// use warpgrapher::engine::database::DatabasePool;
    /// use warpgrapher::engine::context::GlobalContext;
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     global_var: String    
    /// }
    /// 
    /// impl GlobalContext for AppGlobalCtx {}
    ///
    /// let global_ctx = AppGlobalCtx { global_var: "Hello World".to_owned() };
    /// 
    /// let config = Config::default();
    ///
    /// let mut engine = Engine::<AppGlobalCtx, ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_global_ctx(global_ctx)
    ///     .build().unwrap();
    /// ```
    pub fn with_global_ctx(
        mut self,
        global_ctx: GlobalCtx,
    ) -> EngineBuilder<GlobalCtx, RequestCtx> {
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
    /// use warpgrapher::engine::config::Config;
    /// use warpgrapher::engine::database::DatabasePool;
    /// use warpgrapher::engine::objects::resolvers::Resolvers;
    ///
    /// let resolvers = Resolvers::<(), ()>::new();
    ///
    /// let config = Config::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_resolvers(resolvers)
    ///     .build().unwrap();
    /// ```
    pub fn with_resolvers(
        mut self,
        resolvers: Resolvers<GlobalCtx, RequestCtx>,
    ) -> EngineBuilder<GlobalCtx, RequestCtx> {
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
    /// use warpgrapher::engine::config::{Config, Validators};
    /// use warpgrapher::engine::database::DatabasePool;
    ///
    /// let validators = Validators::new();
    ///
    /// let config = Config::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_validators(validators)
    ///     .build().unwrap();
    /// ```
    pub fn with_validators(
        mut self,
        validators: Validators,
    ) -> EngineBuilder<GlobalCtx, RequestCtx> {
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
    /// use warpgrapher::engine::config::{Config, Validators};
    /// use warpgrapher::engine::database::DatabasePool;
    /// use warpgrapher::engine::extensions::Extensions;
    ///
    /// let extensions = Extensions::<(), ()>::new();
    ///
    /// let config = Config::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_extensions(extensions)
    ///     .build().unwrap();
    /// ```
    pub fn with_extensions(
        mut self,
        extensions: Extensions<GlobalCtx, RequestCtx>,
    ) -> EngineBuilder<GlobalCtx, RequestCtx> {
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
    /// use warpgrapher::engine::config::Config;
    /// use warpgrapher::engine::database::DatabasePool;
    ///
    /// let config = Config::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_version("1.0.0".to_owned())
    ///     .build().unwrap();
    /// ```
    pub fn with_version(mut self, version: String) -> EngineBuilder<GlobalCtx, RequestCtx> {
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
    /// use warpgrapher::engine::config::Config;
    /// use warpgrapher::engine::database::DatabasePool;
    ///
    /// let config = Config::new(1, Vec::new(), Vec::new());
    ///
    /// let mut engine = Engine::<()>::new(config, DatabasePool::NoDatabase)
    ///     .build().unwrap();
    /// ```
    pub fn build(self) -> Result<Engine<GlobalCtx, RequestCtx>, Error> {
        // validate engine options
        EngineBuilder::validate_engine(&self.resolvers, &self.validators, &self.config)?;

        let root_node = create_root_node(&self.config)?;

        let engine = Engine::<GlobalCtx, RequestCtx> {
            config: self.config.clone(),
            db_pool: self.db_pool,
            global_ctx: self.global_ctx,
            resolvers: self.resolvers,
            validators: self.validators,
            extensions: self.extensions,
            version: self.version,
            root_node,
        };

        Ok(engine)
    }

    pub fn validate_engine(
        resolvers: &Resolvers<GlobalCtx, RequestCtx>,
        validators: &Validators,
        config: &Config,
    ) -> Result<(), Error> {
        config.validate()?;

        //Validate Custom Endpoint defined in Config exists as a Resolver
        for e in config.endpoints() {
            if !resolvers.contains_key(e.name()) {
                return Err(Error::ResolverNotFound {
                    name: e.name().to_string(),
                });
            }
        }

        //Validate Custom Prop defined in Config exists as a Resolver
        let mut dyn_scalar_props: Vec<Prop> = Vec::new();
        let mut props_with_validator: Vec<Prop> = Vec::new();

        for t in config.types() {
            for p in t.props() {
                p.resolver()
                    .clone()
                    .map_or((), |_| dyn_scalar_props.push(p.clone()));
                p.validator()
                    .clone()
                    .map_or((), |_| props_with_validator.push(p.clone()));
            }
        }
        for dsp in dyn_scalar_props.iter() {
            let resolver_name = dsp
                .resolver()
                .clone()
                .ok_or_else(|| Error::ResolverNotFound {
                    name: dsp.name().to_string(),
                })?;
            if !resolvers.contains_key(&resolver_name) {
                return Err(Error::ResolverNotFound {
                    name: dsp.name().to_string(),
                });
            }
        }

        //Validate Custom Input Validator defined in Config exists as Validator
        for pwv in props_with_validator.iter() {
            let validator_name =
                pwv.validator()
                    .clone()
                    .ok_or_else(|| Error::ValidatorNotFound {
                        name: pwv.name().to_string(),
                    })?;
            if !validators.contains_key(&validator_name) {
                return Err(Error::ValidatorNotFound {
                    name: pwv.name().to_string(),
                });
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
/// use warpgrapher::engine::config::Config;
/// use warpgrapher::engine::database::DatabasePool;
///
/// let config = Config::default();
///
/// #[cfg(feature = "neo4j")]
/// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
///     .build().unwrap();
///
/// ```
#[derive(Clone)]
pub struct Engine<GlobalCtx = (), RequestCtx = ()>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    config: Config,
    db_pool: DatabasePool,
    global_ctx: Option<GlobalCtx>,
    resolvers: Resolvers<GlobalCtx, RequestCtx>,
    validators: Validators,
    extensions: Extensions<GlobalCtx, RequestCtx>,
    version: Option<String>,
    root_node: RootRef<GlobalCtx, RequestCtx>,
}

impl<GlobalCtx, RequestCtx> Engine<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
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
    /// use warpgrapher::engine::config::Config;
    /// use warpgrapher::engine::database::DatabasePool;
    ///
    /// let config = Config::new(1, Vec::new(), Vec::new());
    ///
    /// let mut engine = Engine::<()>::new(config, DatabasePool::NoDatabase)
    ///     .build().unwrap();
    /// ```

    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: Config,
        database_pool: DatabasePool,
    ) -> EngineBuilder<GlobalCtx, RequestCtx> {
        EngineBuilder::<GlobalCtx, RequestCtx> {
            config,
            db_pool: database_pool,
            global_ctx: None,
            resolvers: HashMap::new(),
            validators: HashMap::new(),
            extensions: vec![],
            version: None,
        }
    }

    pub fn execute(
        &self,
        req: GraphQLRequest,
        metadata: HashMap<String, String>,
    ) -> Result<serde_json::Value, Error> {
        debug!("\nRequest: {:#?}\n", req);

        // initialize empty request context
        let mut req_ctx = RequestCtx::new();

        // run pre request plugin hooks
        for extension in &self.extensions {
            extension.pre_request_hook(self.global_ctx.clone(), Some(&mut req_ctx), &metadata)?;
        }

        // execute graphql query
        let res = req.execute(
            &self.root_node,
            &GraphQLContext::<GlobalCtx, RequestCtx>::new(
                self.db_pool.clone(),
                self.resolvers.clone(),
                self.validators.clone(),
                self.extensions.clone(),
                self.global_ctx.clone(),
                Some(req_ctx.clone()),
                self.version.clone(),
            ),
        );

        // convert graphql response (json) to mutable serde_json::Value
        let res_str: String = serde_json::to_string(&res)?;
        let mut res_value: serde_json::Value = serde_json::from_str(&res_str)?;

        // run post request plugin hooks
        for extension in &self.extensions {
            extension.post_request_hook(self.global_ctx.clone(), Some(&req_ctx), &mut res_value)?;
        }

        debug!("Engine::execute -- res_value: {:#?}", res_value);
        Ok(res_value)

        // convert graphql response to string
        /*
        let body = match serde_json::to_string(&res_value) {
            Ok(s) => s,
            Err(e) => return Err(Error::new(ErrorKind::JsonStringConversionFailed(e), None)),
        };

        Ok(body)
        */
    }
}

impl<GlobalCtx, RequestCtx> Display for Engine<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:#?}", self)
    }
}

impl<GlobalCtx, RequestCtx> Debug for Engine<GlobalCtx, RequestCtx>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("Engine")
            .field("config", &self.config)
            .field("db_pool", &self.db_pool)
            .field("global_ctx", &self.global_ctx)
            .field("version", &self.version)
            .finish()
    }
}

/// Notably, the unit tests here likely seem weak. This is because testing most
/// of the functionality requires a database container to be running and
/// reachable, so most of the coverage is provided by integration tests.
#[cfg(test)]
mod tests {
    use super::config::{Config, Validators};
    // use super::context::GraphQLContext;
    // use super::schema::Info;
    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    use super::Engine;
    use super::EngineBuilder;
    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    use crate::engine::database::DatabasePool;
    use crate::engine::objects::resolvers::{ResolverContext, Resolvers};
    use crate::engine::value::Value;
    use crate::error::Error;
    use juniper::ExecutionResult;
    use std::fs::File;
    use std::io::BufReader;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[allow(dead_code)]
    fn test_config() -> Config {
        init();
        let cf = File::open("tests/fixtures/config.yml")
            .expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    #[allow(dead_code)]
    fn load_config(config: &str) -> Config {
        init();
        let cf = File::open(config).expect("Could not open test model config file.");
        let cr = BufReader::new(cf);
        serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
    }

    /// Passes if the engine can be created.
    #[cfg(any(feature = "cosmos", feature = "neo4j"))]
    #[test]
    fn engine_new() {
        init();

        let config = load_config("tests/fixtures/config_minimal.yml");
        let _engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
            .build()
            .unwrap();
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
        init();
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
        init();
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
        init();
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
        init();
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
        context.resolve_scalar(1 as i32)
    }

    fn my_validator(_value: &Value) -> Result<(), Error> {
        Ok(())
    }
}
