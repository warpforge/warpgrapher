use super::extension::{Metadata, MetadataExtension, MetadataExtensionCtx};
//use super::server_addr;
use std::env::var_os;
use std::sync::Arc;
use warpgrapher::{
    Arguments, Error, ErrorKind, ExecutionResult, Executor, GraphQLContext, Info, Value,
};
use warpgrapher::{
    Server, WarpgrapherConfig, WarpgrapherExtensions, WarpgrapherRequestContext,
    WarpgrapherResolvers, WarpgrapherValidators,
};

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
pub fn project_count<AppGlobalCtx, AppReqCtx>(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<AppGlobalCtx, AppReqCtx>>,
) -> ExecutionResult
where
    AppReqCtx: WarpgrapherRequestContext,
{
    // get projects from database
    let graph = executor.context().pool.get().unwrap();
    let query = "MATCH (n:Project) RETURN (n);";
    let results = graph.exec(query).unwrap();

    // return number of projects
    let count = results.data.len();
    Ok(Value::scalar(count as i32))
}

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
pub fn test_server(config_path: &str) -> Server<AppGlobalCtx, AppReqCtx> {
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

    // configure server
    let db_url = match var_os("DB_URL") {
        None => "http://neo4j:testpass@127.0.0.1:7474/db/data".to_owned(),
        Some(os) => os
            .to_str()
            .unwrap_or("http://neo4j:testpass@127.0.0.1:7474/db/data")
            .to_owned(),
    };

    // create server
    Server::<AppGlobalCtx, AppReqCtx>::new(config, db_url)
        .with_global_ctx(global_ctx)
        .with_resolvers(resolvers)
        .with_validators(validators)
        .with_extensions(extensions)
        .build()
        .expect("Failed to build server")
}
