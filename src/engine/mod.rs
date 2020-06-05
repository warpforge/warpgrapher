//! This module provides the Warpgrapher engine, with supporting modules for configuration,
//! GraphQL schema generation, resolvers, and interface to the database.

use super::error::Error;
use config::Configuration;
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
use validators::Validators;

pub mod config;
pub mod context;
pub mod database;
pub mod extensions;
pub mod objects;
pub mod schema;
pub mod validators;
pub mod value;

/// Implements the builder pattern for Warpgrapher engines
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::{Configuration, DatabasePool, Engine};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let config = Configuration::default();
/// let engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase).build()?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct EngineBuilder<GlobalCtx = (), RequestCtx = ()>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    config: Configuration,
    db_pool: DatabasePool,
    extensions: Extensions<GlobalCtx, RequestCtx>,
    global_ctx: Option<GlobalCtx>,
    resolvers: Resolvers<GlobalCtx, RequestCtx>,
    validators: Validators,
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
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::context::GlobalContext;
    ///
    /// #[derive(Clone, Debug)]
    /// pub struct AppGlobalCtx {
    ///     global_var: String    
    /// }
    ///
    /// impl GlobalContext for AppGlobalCtx {}
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let global_ctx = AppGlobalCtx { global_var: "Hello World".to_string() };
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<AppGlobalCtx, ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_global_ctx(global_ctx)
    ///     .build()?;
    /// # Ok(())
    /// # }
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
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::objects::resolvers::Resolvers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let resolvers = Resolvers::<(), ()>::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_resolvers(resolvers)
    ///     .build()?;
    /// # Ok(())
    /// # }
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
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::validators::Validators;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let validators = Validators::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_validators(validators)
    ///     .build()?;
    /// # Ok(())
    /// # }
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
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::extensions::Extensions;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let extensions = Extensions::<(), ()>::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_extensions(extensions)
    ///     .build()?;
    /// # Ok(())
    /// # }
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
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase)
    ///     .with_version("1.0.0".to_string())
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_version(mut self, version: String) -> EngineBuilder<GlobalCtx, RequestCtx> {
        self.version = Some(version);
        self
    }

    /// Builds a configured [`Engine`] including generating the data model, CRUD operations, and
    /// custom endpoints from the [`Configuration`] `c`. Returns the [`Engine`].
    ///
    /// [`Engine`]: ./struct.Engine.html
    /// [`Configuration`]: ./config/struct.Configuration.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ConfigItemDuplicated`] if there is more than one type or
    /// more than one endpoint that use the same name.
    ///
    /// Returns an [`Error`] variant [`ConfigItemReserved`] if a named configuration item, such as
    /// an endpoint or type, has a name that is a reserved word, such as "ID" or the name of a
    /// GraphQL scalar type.
    ///
    /// Returns an [`Error`] variant [`SchemaItemNotFound`] if there is an error in the
    /// configuration, specifically if the configuration of type A references type B, but type B
    /// cannot be found.
    ///
    /// Returns an [`Error`] variant [`ResolverNotFound`] if there is a resolver defined in the
    /// configuration for which no [`ResolverFunc`] has been added to the [`Resolvers`] collection
    /// applied to the EngineBuilder with [`with_resolvers`].
    ///
    /// Returns an [`Error`] variant [`ValidatorNotFound`] if there is a validator defined in the
    /// configuration for which no [`ValidatorFunc`] has been added to the [`Validators`] collection
    /// applied to the EngineBuilder with [`with_validators`].
    ///
    /// Returns an
    ///
    /// [`ConfigItemDuplicated`]: ../error/enum.Error.html#variant.ConfigItemDuplicated
    /// [`ConfigItemReserved`]: ../error/enum.Error.html#variant.ConfigItemReserved
    /// [`Error`]: ../error/enum.Error.html
    /// [`ResolverNotFound`]: ../error/enum.Error.html#variant.ResolverNotFound
    /// [`ResolverFunc`]: ./objects/resolvers/type.ResolverFunc.html
    /// [`Resolvers`]: ./objects/resolvers/type.Resolvers.html
    /// [`SchemaItemNotFound`]: ../error/enum.Error.html#variant.SchemaItemNotFound
    /// [`ValidatorNotFound`]: ../error/enum.Error.html#variant.ValidatorNotFound
    /// [`ValidatorFunc`]: ./config/type.ValidatorFunc.html
    /// [`Validators`]: ./config/type.Validators.html
    /// [`with_resolvers`]: ./struct.EngineBuilder.html#method.with_resolvers
    /// [`with_validators`]: ./struct.EngineBuilder.html#method.with_validators
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::new(1, Vec::new(), Vec::new());
    ///
    /// let mut engine = Engine::<()>::new(config, DatabasePool::NoDatabase).build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Engine<GlobalCtx, RequestCtx>, Error> {
        self.validate()?;

        let root_node = create_root_node(&self.config)?;

        let engine = Engine::<GlobalCtx, RequestCtx> {
            config: self.config,
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

    fn validate(&self) -> Result<(), Error> {
        self.config.validate()?;

        // Validate Custom Endpoint defined in Configuration exists as a Resolver
        for e in self.config.endpoints() {
            if !self.resolvers.contains_key(e.name()) {
                return Err(Error::ResolverNotFound {
                    name: e.name().to_string(),
                });
            }
        }

        for t in self.config.types() {
            // Validate Custom Prop defined in Configuration exists as a Resolver
            for r in t.properties().filter_map(|p| p.resolver().as_ref()) {
                if !self.resolvers.contains_key(r) {
                    return Err(Error::ResolverNotFound {
                        name: r.to_string(),
                    });
                }
            }

            // Validate that custom validator defined in Configuration exists as a Validator
            for v in t.properties().filter_map(|p| p.validator().as_ref()) {
                if !self.validators.contains_key(v) {
                    return Err(Error::ValidatorNotFound {
                        name: v.to_string(),
                    });
                }
            }
        }

        // validation passed
        Ok(())
    }
}

impl Debug for EngineBuilder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EngineBuilder")
            .field("config", &self.config)
            .field("db_pool", &self.db_pool)
            .field("version", &self.version)
            .finish()
    }
}

/// A Warpgrapher GraphQL engine.
///
/// The [`Engine`] struct Juniper GraphQL service on top of it, with an auto-generated set of
/// resolvers that cover basic CRUD operations, and potentially custom resolvers, on a set of
/// data types and the relationships between them.  The engine includes handling of back-end
/// communications with the chosen databse.
///
/// [`Engine`]: ./struct.Engine.html
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::{Configuration, DatabasePool, Engine};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Configuration::default();
///
/// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase).build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Engine<GlobalCtx = (), RequestCtx = ()>
where
    GlobalCtx: GlobalContext,
    RequestCtx: RequestContext,
{
    config: Configuration,
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
    /// Creates a new [`EngineBuilder`]. Requiered arguments are a [`Configuration`], the
    /// deserialized configuration for a Warpgrapher engine, which contains definitions of types
    /// and endpoints, as well as a [`DatabasePool`], which tells the ending how to connect with a
    /// back-end graph storage engine.
    ///
    /// [`Configuration`]: ./config/struct.Configuration.html
    /// [`DatabasePool`]: ./database/enum.DatabasePool.html
    /// [`EngineBuilder`]: ./struct.EngineBuilder.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase).build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: Configuration,
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

    /// Executes a [`GraphQLRequest`], returning a serialized JSON response.
    ///
    /// [`GraphQLRequest`]: ../struct.GraphQLRequest.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ExtensionFailed`] if a pre request hook or post request
    /// hook extension returns an error.
    ///
    /// Returns an [`Error`] variant [`SerializationFailed`] if the engine response cannot be
    /// serialized successfully.
    ///
    /// [`ExtensionFailed`]: ../error/enum.Error.html#variant.ExtensionFailed
    /// [`Error`]: ../error/enum.Error.html
    ///
    /// # Examples
    ///
    /// ```rust,norun
    /// # use warpgrapher::{Configuration, DatabasePool, Engine, GraphQLRequest};
    /// # use serde_json::{from_value, json};
    /// # use std::collections::HashMap;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    /// let mut engine = Engine::<(), ()>::new(config, DatabasePool::NoDatabase).build()?;
    ///
    /// let metadata: HashMap<String, String> = HashMap::new();
    /// let req_body = json!({"query": "query { name }"});
    ///
    /// let result = engine.execute(&from_value::<GraphQLRequest>(req_body)?, &metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(
        &self,
        req: &GraphQLRequest,
        metadata: &HashMap<String, String>,
    ) -> Result<serde_json::Value, Error> {
        debug!(
            "Engine::execute called -- request: {:#?}, metadata: {:#?}",
            req, metadata
        );

        // initialize empty request context
        let mut req_ctx = RequestCtx::new();

        // run pre request plugin hooks
        for extension in &self.extensions {
            extension.pre_request_hook(self.global_ctx.as_ref(), &mut req_ctx, &metadata)?;
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
        let mut res_value = serde_json::to_value(&res)?;

        // run post request plugin hooks
        for extension in &self.extensions {
            extension.post_request_hook(self.global_ctx.as_ref(), &req_ctx, &mut res_value)?;
        }

        debug!("Engine::execute -- res_value: {:#?}", res_value);
        Ok(res_value)
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
    use super::EngineBuilder;
    use crate::engine::database::DatabasePool;
    use crate::engine::objects::resolvers::{ResolverContext, Resolvers};
    use crate::engine::validators::Validators;
    use crate::engine::value::Value;
    use crate::{Configuration, Engine, Error};
    use juniper::ExecutionResult;
    use std::convert::TryInto;
    use std::fs::File;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    /// Passes if the engine can be created.
    #[test]
    fn engine_new() {
        init();

        let _engine = Engine::<(), ()>::new(
            File::open("tests/fixtures/config_minimal.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase,
        )
        .build()
        .unwrap();
    }

    #[test]
    fn test_engine_validate_minimal() {
        //No prop resolver in config
        //No endpoint resolver in config
        //No validator in config
        //No resolver defined
        //No validator defined
        //is_ok
        init();
        assert!(Engine::<(), ()>::new(
            File::open("tests/fixtures/test_config_ok.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .build()
        .is_ok());
    }

    #[test]
    fn test_engine_validate_custom_validators() {
        //Validator defined
        //No validator in config
        //is_ok
        init();

        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));
        assert!(Engine::<(), ()>::new(
            File::open("tests/fixtures/config_minimal.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_validators(validators)
        .build()
        .is_ok());

        //Validator defined
        //Validator in config
        //is_ok
        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));
        assert!(Engine::<(), ()>::new(
            File::open("tests/fixtures/test_config_with_custom_validator.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_validators(validators)
        .build()
        .is_ok());

        //Validator not defined
        //validator in config
        //is_err
        let validators = Validators::new();
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_with_custom_validator.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_validators(validators)
        .build()
        .is_err());
    }

    #[test]
    fn test_engine_validate_custom_endpoint() {
        init();

        //No endpoint resolvers in config
        //No resolver defined
        //is_ok
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_ok.yml").expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .build()
        .is_ok());

        //Endpoint resolver in config
        //No resolver defined
        //is_err
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_with_custom_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert config"),
            DatabasePool::NoDatabase
        )
        .build()
        .is_err());

        //Endpoint resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_with_custom_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());
    }

    #[test]
    fn test_engine_validate_custom_prop() {
        init();

        //Prop resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_with_custom_prop_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());

        //No prop resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<(), ()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/config_minimal.yml").expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());

        //Prop resolver in config
        //No resolver defined
        //is_err
        assert!(Engine::<(), ()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/test_config_with_custom_prop_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            DatabasePool::NoDatabase
        )
        .build()
        .is_err());
    }

    pub fn my_resolver(context: ResolverContext<(), ()>) -> ExecutionResult {
        context.resolve_scalar(1 as i32)
    }

    fn my_validator(_value: &Value) -> Result<(), Error> {
        Ok(())
    }

    /// Passes if EngineBuilder implements the Send trait
    #[test]
    fn test_engine_builder_send() {
        fn assert_send<T: Send>() {}
        assert_send::<EngineBuilder>();
    }

    /// Passes if EngineBuilder implements the Sync trait
    #[test]
    fn test_engine_builder_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<EngineBuilder>();
    }

    /// Passes if the Engine implements the Send trait
    #[test]
    fn test_engine_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Engine>();
    }

    /// Passes if Engine implements the Sync trait
    #[test]
    fn test_engine_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Engine>();
    }
}
