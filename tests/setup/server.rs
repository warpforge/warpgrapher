use super::extension::{Metadata, MetadataExtension, MetadataExtensionCtx};
use actix_web::dev;
use futures::executor::block_on;
use serde_json::json;
use std::env::var_os;
use std::fmt::Debug;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use warpgrapher::engine::config::{Config, Validators};
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::extensions::WarpgrapherExtensions;
use warpgrapher::engine::resolvers::{GraphNode, GraphRel, ResolverContext, Resolvers};
use warpgrapher::juniper::ExecutionResult;
use warpgrapher::{Error, ErrorKind};

#[derive(Clone, Debug)]
pub struct AppGlobalCtx {
    version: String,
}

#[derive(Clone, Debug)]
pub struct AppRequestCtx {
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

pub fn name_validator(value: &serde_json::Value) -> Result<(), Error> {
    let name = match value {
        serde_json::Value::Object(o) => match o.get("name") {
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
        serde_json::Value::String(s) => {
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

/// custom endpoint returning scalar:
#[allow(dead_code)]
pub fn project_count(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    let db = context.get_db()?;
    let query = "MATCH (n:Project) RETURN (n);";
    let results = db.exec(query).unwrap();
    context.resolve_scalar(results.data.len() as i32)
}

/// custom endpoint returning scalar_list:
pub fn global_top_tags(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    context.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
}

/// custom endpoint returning node
pub fn global_top_dev(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    context.resolve_node(GraphNode {
        typename: "User",
        props: json!({
            "name": "Joe"
        })
        .as_object()
        .unwrap(),
    })
}

/*
/// custom endpoint returning node_list
pub fn global_top_issues(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) {
    // TODO: add real database query
    context.resolve_node_list()
}
*/

/// custom field returning scalar
pub fn project_points(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    context.resolve_scalar(138)
}

/// custom field returning scalar_list
pub fn project_top_tags(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    context.resolve_scalar_list(vec!["cypher", "sql", "neo4j"])
}

/// custom rel returning rel
pub fn project_top_dev(context: ResolverContext<AppGlobalCtx, AppRequestCtx>) -> ExecutionResult {
    context.resolve_rel(GraphRel {
        id: "1234567890",
        props: None,
        dst: GraphNode {
            typename: "User",
            props: json!({
                "name": "Joe"
            })
            .as_object()
            .unwrap(),
        },
    })
}

/// custom rel returning rel_list
pub fn project_top_issues(
    context: ResolverContext<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    context.resolve_rel_list(vec![
        GraphRel {
            id: "1234567890",
            props: None,
            dst: GraphNode {
                typename: "Feature",
                props: json!({
                    "name": "Add async support"
                })
                .as_object()
                .unwrap(),
            },
        },
        GraphRel {
            id: "0987654321",
            props: None,
            dst: GraphNode {
                typename: "Bug",
                props: json!({
                    "name": "Fix type mismatch"
                })
                .as_object()
                .unwrap(),
            },
        },
    ])
}

pub struct Server {
    config: Config,
    db_url: String,
    global_ctx: AppGlobalCtx,
    resolvers: Resolvers<AppGlobalCtx, AppRequestCtx>,
    validators: Validators,
    extensions: WarpgrapherExtensions<AppGlobalCtx, AppRequestCtx>,
    server: Option<dev::Server>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn new(
        config: Config,
        db_url: String,
        global_ctx: AppGlobalCtx,
        resolvers: Resolvers<AppGlobalCtx, AppRequestCtx>,
        validators: Validators,
        extensions: WarpgrapherExtensions<AppGlobalCtx, AppRequestCtx>,
    ) -> Server {
        Server {
            config,
            db_url,
            global_ctx,
            resolvers,
            validators,
            extensions,
            server: None,
            handle: None,
        }
    }

    pub fn serve(&mut self, block: bool) -> Result<(), Error> {
        if self.handle.is_some() || self.server.is_some() {
            return Err(Error::new(ErrorKind::ServerAlreadyRunning, None));
        }

        let (tx, rx) = mpsc::channel();

        if block {
            super::actix_server::start(
                &self.config,
                &self.db_url,
                &self.global_ctx,
                &self.resolvers,
                &self.validators,
                &self.extensions,
                tx,
            );
        } else {
            let config = self.config.clone();
            let db_url = self.db_url.clone();
            let global_ctx = self.global_ctx.clone();
            let resolvers = self.resolvers.clone();
            let validators = self.validators.clone();
            let extensions = self.extensions.clone();

            self.handle = Some(spawn(move || {
                super::actix_server::start(
                    &config,
                    &db_url,
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
pub fn test_server(config_path: &str) -> Server {
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
    let extensions: WarpgrapherExtensions<AppGlobalCtx, AppRequestCtx> =
        vec![Arc::new(metadata_extension)];

    // configure server
    let db_url = match var_os("DB_URL") {
        None => "http://neo4j:testpass@127.0.0.1:7474/db/data".to_owned(),
        Some(os) => os
            .to_str()
            .unwrap_or("http://neo4j:testpass@127.0.0.1:7474/db/data")
            .to_owned(),
    };

    // create server
    Server::new(
        config, db_url, global_ctx, resolvers, validators, extensions,
    )
}
