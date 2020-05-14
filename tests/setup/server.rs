#[cfg(feature = "neo4j")]
use super::extension::MetadataExtension;
use super::extension::{Metadata, MetadataExtensionCtx};
use futures::executor::block_on;
use juniper::{Arguments, ExecutionResult, Executor};
#[allow(unused_imports)]
use std::collections::HashMap;
use std::sync::mpsc;
#[cfg(feature = "neo4j")]
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use warpgrapher::engine::config::Config;
use warpgrapher::engine::config::Resolvers;
use warpgrapher::engine::config::Validators;
use warpgrapher::engine::context::GraphQLContext;
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "graphson2")]
use warpgrapher::engine::database::graphson2::Graphson2Endpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use warpgrapher::engine::database::DatabasePool;
use warpgrapher::engine::extensions::Extensions;
use warpgrapher::engine::schema::Info;
use warpgrapher::engine::value::Value;
use warpgrapher::{Error, ErrorKind};

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
#[derive(Clone, Debug)]
pub struct AppGlobalCtx {
    version: String,
}

#[derive(Clone, Debug)]
pub struct AppReqCtx {
    metadata: Metadata,
}

impl RequestContext for AppReqCtx {
    fn new() -> AppReqCtx {
        AppReqCtx {
            metadata: Metadata {
                src_ip: "".to_string(),
                src_useragent: "".to_string(),
            },
        }
    }
}

impl MetadataExtensionCtx for AppReqCtx {
    fn set_metadata(&mut self, metadata: Metadata) {
        self.metadata = metadata
    }
}

#[allow(dead_code)]
pub fn name_validator(value: &Value) -> Result<(), Error> {
    let name = match value {
        Value::Map(m) => match m.get("name") {
            Some(n) => n,
            None => {
                return Err(Error::new(
                    ErrorKind::ValidationError(format!(
                        "Input validator for {field_name} failed.",
                        field_name = "name"
                    )),
                    None,
                ))
            }
        },
        _ => {
            return Err(Error::new(
                ErrorKind::ValidationError(format!(
                    "Input validator for {field_name} failed.",
                    field_name = "name"
                )),
                None,
            ))
        }
    };

    match name {
        Value::String(s) => {
            if s == "KENOBI" {
                Err(Error::new(
                    ErrorKind::ValidationError(format!(
                        "Input validator for {field_name} failed. Cannot be named KENOBI",
                        field_name = "name"
                    )),
                    None,
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(Error::new(
            ErrorKind::ValidationError(format!(
                "Input validator for {field_name} failed.",
                field_name = "name"
            )),
            None,
        )),
    }
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub fn project_count<AppGlobalCtx, AppReqCtx>(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<AppGlobalCtx, AppReqCtx>>,
) -> ExecutionResult
where
    AppReqCtx: RequestContext,
{
    match &executor.context().pool {
        DatabasePool::Neo4j(p) => {
            // get projects from database
            let graph = p.get().unwrap();
            let mut transaction = graph.transaction().begin()?.0;
            let query = "MATCH (n:Project) RETURN (n);";
            let results = transaction.exec(query).unwrap();

            // return number of projects
            let count = results.data.len();
            Ok(juniper::Value::scalar(count as i32))
        }
        _ => Err(Error::new(
            ErrorKind::UnsupportedDatabase("Anything but neo4j".to_owned()),
            None,
        )
        .into()),
    }
}

#[allow(dead_code)]
pub fn project_points<AppGlobalCtx, AppReqCtx>(
    _info: &Info,
    _args: &Arguments,
    _executor: &Executor<GraphQLContext<AppGlobalCtx, AppReqCtx>>,
) -> ExecutionResult
where
    AppReqCtx: RequestContext,
{
    Ok(juniper::Value::scalar(1_000_000 as i32))
}

#[allow(dead_code)]
#[cfg(feature = "graphson2")]
pub fn test_server_graphson2(config_path: &str) -> Server {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config = Config::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // create server
    Server::new(
        config,
        Graphson2Endpoint::from_env().unwrap().get_pool().unwrap(),
        global_ctx,
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
    )
}

#[allow(dead_code)]
pub struct Server {
    config: Config,
    db_pool: DatabasePool,
    global_ctx: AppGlobalCtx,
    resolvers: Resolvers<AppGlobalCtx, AppReqCtx>,
    validators: Validators,
    extensions: Extensions<AppGlobalCtx, AppReqCtx>,
    server: Option<actix_web::dev::Server>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn new(
        config: Config,
        db_pool: DatabasePool,
        global_ctx: AppGlobalCtx,
        resolvers: Resolvers<AppGlobalCtx, AppReqCtx>,
        validators: Validators,
        extensions: Extensions<AppGlobalCtx, AppReqCtx>,
    ) -> Server {
        Server {
            config,
            db_pool,
            global_ctx,
            resolvers,
            validators,
            extensions,
            server: None,
            handle: None,
        }
    }

    #[allow(dead_code)]
    pub fn serve(&mut self, block: bool) -> Result<(), Error> {
        if self.handle.is_some() || self.server.is_some() {
            return Err(Error::new(ErrorKind::ServerAlreadyRunning, None));
        }

        let (tx, rx) = mpsc::channel();

        if block {
            super::actix_server::start(
                &self.config,
                self.db_pool.clone(),
                &self.global_ctx,
                &self.resolvers,
                &self.validators,
                &self.extensions,
                tx,
            );
        } else {
            let config = self.config.clone();
            let db_pool = self.db_pool.clone();
            let global_ctx = self.global_ctx.clone();
            let resolvers = self.resolvers.clone();
            let validators = self.validators.clone();
            let extensions = self.extensions.clone();

            self.handle = Some(spawn(move || {
                super::actix_server::start(
                    &config,
                    db_pool,
                    &global_ctx,
                    &resolvers,
                    &validators,
                    &extensions,
                    tx,
                );
            }));
        }

        rx.recv()
            .map_err(|e| Error::new(ErrorKind::ServerStartupFailed(e), None))
            .and_then(|m| match m {
                Ok(s) => {
                    self.server = Some(s);
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

    #[allow(dead_code)]
    pub fn shutdown(&mut self) -> Result<(), Error> {
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
            .map_err(|_| Error::new(ErrorKind::ServerShutdownFailed, None))
            .and_then(|_| Ok(()))
    }
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub fn test_server_neo4j(config_path: &str) -> Server {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config = Config::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // load resolvers
    let mut resolvers: Resolvers<AppGlobalCtx, AppReqCtx> = Resolvers::new();
    resolvers.insert(
        "ProjectCount".to_owned(),
        Box::new(project_count::<AppGlobalCtx, AppReqCtx>),
    );

    resolvers.insert(
        "ProjectPoints".to_string(),
        Box::new(project_points::<AppGlobalCtx, AppReqCtx>),
    );

    let mut validators: Validators = Validators::new();
    validators.insert("NameValidator".to_string(), Box::new(name_validator));

    // initialize extensions
    let metadata_extension: MetadataExtension<AppGlobalCtx, AppReqCtx> = MetadataExtension::new();
    let extensions: Extensions<AppGlobalCtx, AppReqCtx> = vec![Arc::new(metadata_extension)];

    // create server
    Server::new(
        config,
        Neo4jEndpoint::from_env().unwrap().get_pool().unwrap(),
        global_ctx,
        resolvers,
        validators,
        extensions,
    )
}
