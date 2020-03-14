#[cfg(feature = "neo4j")]
use super::extension::MetadataExtension;
use super::extension::{Metadata, MetadataExtensionCtx};
#[cfg(feature = "neo4j")]
use std::sync::Arc;
#[cfg(feature = "graphson2")]
use warpgrapher::server::database::graphson2::Graphson2Endpoint;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use warpgrapher::server::database::DatabaseEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::server::database::DatabasePool;
use warpgrapher::{
    Arguments, Error, ErrorKind, ExecutionResult, Executor, GraphQLContext, Info, Value,
    WarpgrapherRequestContext,
};
#[cfg(feature = "neo4j")]
use warpgrapher::{
    Neo4jEndpoint, WarpgrapherExtensions, WarpgrapherResolvers, WarpgrapherValidators,
};
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use warpgrapher::{Server, WarpgrapherConfig};

#[cfg(any(feature = "graphson2", feature = "neo4j"))]
#[derive(Clone, Debug)]
pub struct AppGlobalCtx {
    version: String,
}

#[derive(Clone, Debug)]
pub struct AppReqCtx {
    metadata: Metadata,
}

impl WarpgrapherRequestContext for AppReqCtx {
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

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub fn project_count<AppGlobalCtx, AppReqCtx>(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<AppGlobalCtx, AppReqCtx>>,
) -> ExecutionResult
where
    AppReqCtx: WarpgrapherRequestContext,
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
            Ok(Value::scalar(count as i32))
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
    AppReqCtx: WarpgrapherRequestContext,
{
    Ok(Value::scalar(1_000_000 as i32))
}

#[allow(dead_code)]
#[cfg(feature = "graphson2")]
pub fn test_server_graphson2(config_path: &str) -> Server<AppGlobalCtx, AppReqCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config =
        WarpgrapherConfig::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // create server
    Server::new(
        config,
        Graphson2Endpoint::from_env().unwrap().get_pool().unwrap(),
    )
    .with_global_ctx(global_ctx)
    .build()
    .expect("Failed to build server")
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub fn test_server_neo4j(config_path: &str) -> Server<AppGlobalCtx, AppReqCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config =
        WarpgrapherConfig::from_file(config_path.to_string()).expect("Failed to load config file");

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    // load resolvers
    let mut resolvers: WarpgrapherResolvers<AppGlobalCtx, AppReqCtx> = WarpgrapherResolvers::new();
    resolvers.insert(
        "ProjectCount".to_owned(),
        Box::new(project_count::<AppGlobalCtx, AppReqCtx>),
    );

    resolvers.insert(
        "ProjectPoints".to_string(),
        Box::new(project_points::<AppGlobalCtx, AppReqCtx>),
    );

    let mut validators: WarpgrapherValidators = WarpgrapherValidators::new();
    validators.insert("NameValidator".to_string(), Box::new(name_validator));

    // initialize extensions
    let metadata_extension: MetadataExtension<AppGlobalCtx, AppReqCtx> = MetadataExtension::new();
    let extensions: WarpgrapherExtensions<AppGlobalCtx, AppReqCtx> =
        vec![Arc::new(metadata_extension)];

    // create server
    Server::new(
        config,
        Neo4jEndpoint::from_env().unwrap().get_pool().unwrap(),
    )
    .with_global_ctx(global_ctx)
    .with_resolvers(resolvers)
    .with_validators(validators)
    .with_extensions(extensions)
    .build()
    .expect("Failed to build server")
}
