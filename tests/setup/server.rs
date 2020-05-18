#[cfg(feature = "neo4j")]
use super::extension::MetadataExtension;
use super::extension::{Metadata, MetadataExtensionCtx};
use futures::executor::block_on;
#[cfg(feature = "neo4j")]
use juniper::ExecutionResult;
#[allow(unused_imports)]
use std::collections::HashMap;
use std::sync::mpsc;
#[cfg(feature = "neo4j")]
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use warpgrapher::engine::config::Config;
use warpgrapher::engine::config::Validators;
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "cosmos")]
use warpgrapher::engine::database::cosmos::CosmosEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use warpgrapher::engine::database::DatabasePool;
use warpgrapher::engine::extensions::Extensions;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::objects::resolvers::{GraphNode, GraphRel, ResolverContext};
use warpgrapher::engine::objects::resolvers::{Resolvers};
use warpgrapher::engine::value::Value;
use warpgrapher::{Error, ErrorKind};

#[cfg(any(feature = "cosmos", feature = "neo4j"))]
#[derive(Clone, Debug)]
pub(crate) struct AppGlobalCtx {
    version: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AppRequestCtx {
    metadata: Metadata,
}

impl RequestContext for AppRequestCtx {
    fn new() -> AppRequestCtx {
        AppRequestCtx {
            metadata: Metadata {
                src_ip: "".to_string(),
                src_useragent: "".to_string(),
            },
        }
    }
}

impl MetadataExtensionCtx for AppRequestCtx {
    fn set_metadata(&mut self, metadata: Metadata) {
        self.metadata = metadata
    }
}

#[allow(dead_code)]
fn name_validator(value: &Value) -> Result<(), Error> {
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
#[cfg(feature = "cosmos")]
pub(crate) fn test_server_cosmos(config_path: &str) -> Server {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config = Config::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // create server
    Server::new(
        "5001",
        config,
        CosmosEndpoint::from_env().unwrap().pool().unwrap(),
        global_ctx,
        HashMap::new(),
        HashMap::new(),
        Vec::new(),
    )
}

#[cfg(feature = "neo4j")]
pub(crate) fn project_count(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    if let DatabasePool::Neo4j(p) = context.executor().context().pool() {
        let db = p.get()?;
        let query = "MATCH (n:Project) RETURN (n);";
        let results = db.exec(query)?;
        context.resolve_scalar(results.data.len() as i32)
    } else {
        Err(Error::new(
            ErrorKind::UnsupportedDatabase("Non-Neo4j".to_string()),
            None,
        )
        .into())
    }
}

/// custom endpoint returning scalar_list:
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_tags(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    context.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
}

/// custom endpoint returning node
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_dev(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    let mut hm = HashMap::new();
    hm.insert("name".to_string(), Value::String("Joe".to_string()));
    context.resolve_node(GraphNode::new("User", &hm))
}

/*
/// custom endpoint returning node_list
pub fn global_top_issues(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) {
    // TODO: add real database query
    context.resolve_node_list()
}
*/

/// custom field returning scalar
#[cfg(feature = "neo4j")]
pub(crate) fn project_points(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    context.resolve_scalar(138)
}

/// custom field returning scalar_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_tags(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    context.resolve_scalar_list(vec!["cypher", "sql", "neo4j"])
}

/// custom rel returning rel
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_dev(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    let mut hm = HashMap::new();
    hm.insert("name".to_string(), Value::String("Joe".to_string()));
    context.resolve_rel(GraphRel::new(
        "1234567890",
        None,
        GraphNode::new("User", &hm),
    ))
}

/// custom rel returning rel_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_issues(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    let mut hm1 = HashMap::new();
    hm1.insert(
        "name".to_string(),
        Value::String("Add async support".to_string()),
    );
    let mut hm2 = HashMap::new();
    hm2.insert(
        "name".to_string(),
        Value::String("Fix type mismatch".to_string()),
    );
    context.resolve_rel_list(vec![
        GraphRel::new("1234567890", None, GraphNode::new("Feature", &hm1)),
        GraphRel::new("0987654321", None, GraphNode::new("Bug", &hm2)),
    ])
}

#[allow(dead_code)]
pub(crate) struct Server {
    bind_port: String,
    config: Config,
    db_pool: DatabasePool,
    global_ctx: AppGlobalCtx,
    resolvers: Resolvers<AppGlobalCtx, AppRequestCtx>,
    validators: Validators,
    extensions: Extensions<AppGlobalCtx, AppRequestCtx>,
    server: Option<actix_web::dev::Server>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn new(
        bind_port: &str,
        config: Config,
        db_pool: DatabasePool,
        global_ctx: AppGlobalCtx,
        resolvers: Resolvers<AppGlobalCtx, AppRequestCtx>,
        validators: Validators,
        extensions: Extensions<AppGlobalCtx, AppRequestCtx>,
    ) -> Server {
        Server {
            bind_port: bind_port.to_owned(),
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
    pub(crate) fn serve(&mut self, block: bool) -> Result<(), Error> {
        if self.handle.is_some() || self.server.is_some() {
            return Err(Error::new(ErrorKind::ServerAlreadyRunning, None));
        }

        let (tx, rx) = mpsc::channel();

        if block {
            super::actix_server::start(
                &self.bind_port,
                &self.config,
                self.db_pool.clone(),
                &self.global_ctx,
                &self.resolvers,
                &self.validators,
                &self.extensions,
                tx,
            );
        } else {
            let bind_port = self.bind_port.clone();
            let config = self.config.clone();
            let db_pool = self.db_pool.clone();
            let global_ctx = self.global_ctx.clone();
            let resolvers = self.resolvers.clone();
            let validators = self.validators.clone();
            let extensions = self.extensions.clone();

            self.handle = Some(spawn(move || {
                super::actix_server::start(
                    &bind_port,
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
    pub(crate) fn shutdown(&mut self) -> Result<(), Error> {
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
pub(crate) fn test_server_neo4j(config_path: &str) -> Server {
    // load config
    let config = Config::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // load resolvers
    let mut resolvers: Resolvers<AppGlobalCtx, AppRequestCtx> = Resolvers::new();
    resolvers.insert("GlobalTopDev".to_owned(), Box::new(global_top_dev));
    resolvers.insert("GlobalTopTags".to_owned(), Box::new(global_top_tags));
    resolvers.insert("ProjectCount".to_owned(), Box::new(project_count));
    resolvers.insert("ProjectPoints".to_string(), Box::new(project_points));
    resolvers.insert("ProjectTopDev".to_string(), Box::new(project_top_dev));
    resolvers.insert("ProjectTopIssues".to_string(), Box::new(project_top_issues));
    resolvers.insert("ProjectTopTags".to_string(), Box::new(project_top_tags));

    // load validators
    let mut validators: Validators = Validators::new();
    validators.insert("NameValidator".to_string(), Box::new(name_validator));

    // initialize extensions
    let metadata_extension: MetadataExtension<AppGlobalCtx, AppRequestCtx> =
        MetadataExtension::new();
    let extensions: Extensions<AppGlobalCtx, AppRequestCtx> = vec![Arc::new(metadata_extension)];

    // create server
    Server::new(
        "5000",
        config,
        Neo4jEndpoint::from_env().unwrap().pool().unwrap(),
        global_ctx,
        resolvers,
        validators,
        extensions,
    )
}
