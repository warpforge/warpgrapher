//! This module provides the Warpgrapher engine, with supporting modules for configuration,
//! GraphQL schema generation, resolvers, and interface to the database.
use super::error::Error;
use config::Configuration;
use context::{GraphQLContext, RequestContext};
use database::{CrudOperation, DatabaseEndpoint, DatabasePool, Transaction};
use events::{EventFacade, EventHandlerBag};
use juniper::http::GraphQLRequest;
use log::debug;
use resolvers::Resolvers;
use schema::{create_root_node, Info, NodeType, RootRef};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::option::Option;
use std::sync::Arc;
use validators::Validators;

pub mod config;
pub mod context;
pub mod database;
pub mod events;
pub mod loader;
pub mod objects;
pub mod resolvers;
pub mod schema;
pub mod validators;
pub mod value;

/// Implements the builder pattern for Warpgrapher engines
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::{Configuration, Engine};
/// # use warpgrapher::engine::database::no_database::NoDatabasePool;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
///
/// let config = Configuration::default();
/// let engine = Engine::<()>::new(config, NoDatabasePool {}).build()?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EngineBuilder<RequestCtx = ()>
where
    RequestCtx: RequestContext,
{
    config: Configuration,
    db_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    event_handlers: EventHandlerBag<RequestCtx>,
    resolvers: Resolvers<RequestCtx>,
    validators: Validators,
    version: Option<String>,
}

impl<RequestCtx> EngineBuilder<RequestCtx>
where
    RequestCtx: RequestContext,
{
    /// Adds resolvers to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    /// # use warpgrapher::engine::resolvers::Resolvers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let resolvers = Resolvers::<()>::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {})
    ///     .with_resolvers(resolvers)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_resolvers(mut self, resolvers: Resolvers<RequestCtx>) -> EngineBuilder<RequestCtx> {
        self.resolvers = resolvers;
        self
    }

    /// Adds validators to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    /// # use warpgrapher::engine::validators::Validators;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let validators = Validators::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {})
    ///     .with_validators(validators)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_validators(mut self, validators: Validators) -> EngineBuilder<RequestCtx> {
        self.validators = validators;
        self
    }

    /// Adds event handlers to the engine
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let event_handlers = EventHandlerBag::<()>::new();
    ///
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {})
    ///     .with_event_handlers(event_handlers)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_event_handlers(
        mut self,
        event_handlers: EventHandlerBag<RequestCtx>,
    ) -> EngineBuilder<RequestCtx> {
        self.event_handlers = event_handlers;
        self
    }

    /// Sets the version of the app
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {})
    ///     .with_version("1.0.0".to_string())
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_version(mut self, version: String) -> EngineBuilder<RequestCtx> {
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
    /// [`ResolverFunc`]: ./resolvers/type.ResolverFunc.html
    /// [`Resolvers`]: ./resolvers/type.Resolvers.html
    /// [`SchemaItemNotFound`]: ../error/enum.Error.html#variant.SchemaItemNotFound
    /// [`ValidatorNotFound`]: ../error/enum.Error.html#variant.ValidatorNotFound
    /// [`ValidatorFunc`]: ./validators/type.ValidatorFunc.html
    /// [`Validators`]: ./validators/type.Validators.html
    /// [`with_resolvers`]: ./struct.EngineBuilder.html#method.with_resolvers
    /// [`with_validators`]: ./struct.EngineBuilder.html#method.with_validators
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::new(1, Vec::new(), Vec::new());
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {}).build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(mut self) -> Result<Engine<RequestCtx>, Error> {
        self.validate()?;

        for event_handler in self.event_handlers.before_engine_build() {
            event_handler(&mut self.config)?;
        }

        let root_node = create_root_node(&self.config)?;

        let engine = Engine::<RequestCtx> {
            config: self.config,
            db_pool: self.db_pool,
            resolvers: self.resolvers,
            validators: self.validators,
            event_handlers: self.event_handlers,
            version: self.version,
            root_node,
        };

        Ok(engine)
    }

    fn validate(&self) -> Result<(), Error> {
        self.config.validate()?;

        // Validate Custom Endpoint defined in Configuration exists as a Resolver
        self.config
            .endpoints()
            .map(|e| {
                if !self.resolvers.contains_key(e.name()) {
                    Err(Error::ResolverNotFound {
                        name: e.name().to_string(),
                    })
                } else {
                    Ok(())
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;

        self.config
            .types()
            .map(|t| {
                // Validate that custom resolver defined in Configuration exists as a Resolver
                t.props()
                    .filter_map(|p| p.resolver())
                    .map(|r| {
                        if !self.resolvers.contains_key(r) {
                            Err(Error::ResolverNotFound {
                                name: r.to_string(),
                            })
                        } else {
                            Ok(())
                        }
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                // Validate that custom validator defined in Configuration exists as a Validator
                t.props()
                    .filter_map(|p| p.validator())
                    .map(|v| {
                        if !self.validators.contains_key(v) {
                            Err(Error::ValidatorNotFound {
                                name: v.to_string(),
                            })
                        } else {
                            Ok(())
                        }
                    })
                    .collect::<Result<Vec<_>, Error>>()?;

                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // validation passed
        Ok(())
    }
}

impl Debug for EngineBuilder {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EngineBuilder")
            .field("config", &self.config)
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
/// # use warpgrapher::engine::database::no_database::NoDatabasePool;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Configuration::default();
///
/// let mut engine = Engine::<()>::new(config, NoDatabasePool {}).build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Engine<RequestCtx = ()>
where
    RequestCtx: RequestContext,
{
    config: Configuration,
    db_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    resolvers: Resolvers<RequestCtx>,
    validators: Validators,
    event_handlers: EventHandlerBag<RequestCtx>,
    version: Option<String>,
    root_node: RootRef<RequestCtx>,
}

impl<RequestCtx> Engine<RequestCtx>
where
    RequestCtx: RequestContext,
{
    /// Creates a new [`EngineBuilder`]. Requiered arguments are a [`Configuration`], the
    /// deserialized configuration for a Warpgrapher engine, which contains definitions of types
    /// and endpoints, as well as a [`DatabasePool`], which tells the ending how to connect with a
    /// back-end graph storage engine.
    ///
    /// [`Configuration`]: ./config/struct.Configuration.html
    /// [`DatabasePool`]: ./database/trait.DatabasePool.html
    /// [`EngineBuilder`]: ./struct.EngineBuilder.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    ///
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {}).build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        config: Configuration,
        database_pool: <<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType,
    ) -> EngineBuilder<RequestCtx> {
        EngineBuilder::<RequestCtx> {
            config,
            db_pool: database_pool,
            resolvers: HashMap::new(),
            validators: HashMap::new(),
            event_handlers: EventHandlerBag::new(),
            version: None,
        }
    }

    /// Executes a [`GraphQLRequest`], returning a serialized JSON response.
    ///
    /// [`GraphQLRequest`]: ../../juniper/http/struct.GraphQLRequest.html
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] variant [`ExtensionFailed`] if a pre request hook or post request
    /// hook extension returns an error.
    ///
    /// Returns an [`Error`] variant [`SerializationFailed`] if the engine response cannot be
    /// serialized successfully.
    ///
    /// [`Error`]: ../error/enum.Error.html
    /// [`ExtensionFailed`]: ../error/enum.Error.html#variant.ExtensionFailed
    /// [`SerializationFailed`]: ../error/enum.Error.html#variant.SerializationFailed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use warpgrapher::{Configuration, DatabasePool, Engine};
    /// # use warpgrapher::engine::database::no_database::NoDatabasePool;
    /// # use warpgrapher::juniper::http::GraphQLRequest;
    /// # use serde_json::{from_value, json};
    /// # use std::collections::HashMap;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Configuration::default();
    /// let mut engine = Engine::<()>::new(config, NoDatabasePool {}).build()?;
    ///
    /// let query = "query { name }".to_string();
    /// let metadata: HashMap<String, String> = HashMap::new();
    ///
    /// let result = engine.execute(query, None, metadata).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[tracing::instrument(name = "wg-execute", skip(self, query, input, metadata))]
    pub async fn execute(
        &self,
        query: String,
        input: Option<serde_json::Value>,
        metadata: HashMap<String, String>,
    ) -> Result<serde_json::Value, Error> {
        debug!("Engine::execute called");

        // create new request context
        let mut rctx = RequestCtx::new();

        let gql_schema: HashMap<String, NodeType> =
            crate::engine::schema::generate_schema(&self.config)?;
        let info = Info::new("".to_string(), Arc::new(gql_schema));

        // execute before_request handlers
        let before_request_handlers = self.event_handlers.before_request();
        if !before_request_handlers.is_empty() {
            let mut dbtx = self.db_pool.transaction().await?;
            let gqlctx_tmp = GraphQLContext::<RequestCtx>::new(
                self.db_pool.clone(),
                self.resolvers.clone(),
                self.validators.clone(),
                self.event_handlers.clone(),
                Some(rctx.clone()),
                self.version.clone(),
                metadata.clone(),
                info.clone(),
            );
            for handler in before_request_handlers {
                rctx = handler(
                    rctx,
                    EventFacade::new(CrudOperation::None, &gqlctx_tmp, &mut dbtx, &info),
                    metadata.clone(),
                )
                .await?;
            }
            std::mem::drop(dbtx);
        }

        // convert serde_json input to juniper input
        let input_value: Option<juniper::InputValue> = match input {
            Some(input) => Some(serde_json::from_value::<juniper::InputValue>(input)?),
            None => None,
        };

        // execute graphql query
        let gqlctx = GraphQLContext::<RequestCtx>::new(
            self.db_pool.clone(),
            self.resolvers.clone(),
            self.validators.clone(),
            self.event_handlers.clone(),
            Some(rctx.clone()),
            self.version.clone(),
            metadata.clone(),
            info.clone(),
        );
        let req = GraphQLRequest::new(query, None, input_value);
        let res = req.execute(&self.root_node, &gqlctx).await;

        // convert graphql response (json) to mutable serde_json::Value
        let mut ret_value = serde_json::to_value(&res)?;

        // execute after_request handlers
        let after_request_handlers = self.event_handlers.after_request();
        if !after_request_handlers.is_empty() {
            let mut dbtx = self.db_pool.transaction().await?;
            let gqlctx_tmp = GraphQLContext::<RequestCtx>::new(
                self.db_pool.clone(),
                self.resolvers.clone(),
                self.validators.clone(),
                self.event_handlers.clone(),
                Some(rctx.clone()),
                self.version.clone(),
                metadata.clone(),
                info.clone(),
            );
            for handler in self.event_handlers.after_request() {
                ret_value = handler(
                    EventFacade::new(CrudOperation::None, &gqlctx_tmp, &mut dbtx, &info),
                    ret_value,
                )
                .await?;
            }
            std::mem::drop(dbtx);
        }

        debug!("Engine::execute -- ret_value: {:#?}", ret_value);
        Ok(ret_value)
    }
}

impl<RequestCtx> Display for Engine<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:#?}", self)
    }
}

impl<RequestCtx> Debug for Engine<RequestCtx>
where
    RequestCtx: RequestContext,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("Engine")
            .field("config", &self.config)
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
    use crate::engine::database::no_database::NoDatabasePool;
    use crate::engine::resolvers::{ResolverFacade, Resolvers};
    use crate::engine::validators::Validators;
    use crate::engine::value::Value;
    use crate::{Configuration, Engine, Error};
    use juniper::{BoxFuture, ExecutionResult};
    use std::convert::TryInto;
    use std::fs::File;

    /// Passes if the engine can be created.
    #[test]
    fn engine_new() {
        let _engine = Engine::<()>::new(
            File::open("tests/fixtures/minimal.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            NoDatabasePool {},
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
        assert!(Engine::<()>::new(
            File::open("tests/fixtures/config-validation/test_config_ok.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .build()
        .is_ok());
    }

    #[test]
    fn test_engine_validate_custom_validators() {
        //Validator defined
        //No validator in config
        //is_ok
        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));
        assert!(Engine::<()>::new(
            File::open("tests/fixtures/minimal.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_validators(validators)
        .build()
        .is_ok());

        //Validator defined
        //Validator in config
        //is_ok
        let mut validators = Validators::new();
        validators.insert("MyValidator".to_string(), Box::new(my_validator));
        assert!(Engine::<()>::new(
            File::open("tests/fixtures/config-validation/test_config_with_custom_validator.yml")
                .expect("Couldn't read config")
                .try_into()
                .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_validators(validators)
        .build()
        .is_ok());

        //Validator not defined
        //validator in config
        //is_err
        let validators = Validators::new();
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open(
                    "tests/fixtures/config-validation/test_config_with_custom_validator.yml"
                )
                .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_validators(validators)
        .build()
        .is_err());
    }

    #[test]
    fn test_engine_validate_custom_endpoint() {
        //No endpoint resolvers in config
        //No resolver defined
        //is_ok
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/config-validation/test_config_ok.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .build()
        .is_ok());

        //Endpoint resolver in config
        //No resolver defined
        //is_err
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/config-validation/test_config_with_custom_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert config"),
            NoDatabasePool {}
        )
        .build()
        .is_err());

        //Endpoint resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/config-validation/test_config_with_custom_resolver.yml")
                    .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());
    }

    #[test]
    fn test_engine_validate_custom_prop() {
        //Prop resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open(
                    "tests/fixtures/config-validation/test_config_with_custom_prop_resolver.yml"
                )
                .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());

        //No prop resolver in config
        //Resolver defined
        //is_ok
        let mut resolvers = Resolvers::<()>::new();
        resolvers.insert("MyResolver".to_string(), Box::new(my_resolver));
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open("tests/fixtures/minimal.yml").expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .with_resolvers(resolvers)
        .build()
        .is_ok());

        //Prop resolver in config
        //No resolver defined
        //is_err
        assert!(Engine::<()>::new(
            TryInto::<Configuration>::try_into(
                File::open(
                    "tests/fixtures/config-validation/test_config_with_custom_prop_resolver.yml"
                )
                .expect("Couldn't read config")
            )
            .expect("Couldn't convert to config"),
            NoDatabasePool {}
        )
        .build()
        .is_err());
    }

    pub fn my_resolver(executor: ResolverFacade<()>) -> BoxFuture<ExecutionResult> {
        Box::pin(async move { executor.resolve_scalar(1) })
    }

    #[allow(clippy::unnecessary_wraps)]
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
