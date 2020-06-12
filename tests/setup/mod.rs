mod extension;

#[cfg(feature = "neo4j")]
use extension::MetadataExtension;
use extension::{Metadata, MetadataExtensionCtx};
#[cfg(feature = "cosmos")]
use gremlin_client::{ConnectionOptions, GraphSON, GremlinClient};
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use log::trace;
#[cfg(feature = "neo4j")]
use rusted_cypher::GraphClient;
#[cfg(feature = "neo4j")]
use std::collections::HashMap;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use std::convert::TryInto;
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
#[cfg(feature = "neo4j")]
use std::sync::Arc;
use warpgrapher::engine::context::{GlobalContext, RequestContext};
#[cfg(feature = "cosmos")]
use warpgrapher::engine::database::cosmos::CosmosEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::DatabasePool;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::extensions::Extensions;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::resolvers::ExecutionResult;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::resolvers::ResolverFacade;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::resolvers::Resolvers;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::validators::Validators;
use warpgrapher::engine::value::Value;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use warpgrapher::{Client, Engine};
use warpgrapher::{Configuration, Error};

#[allow(dead_code)]
pub(crate) fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[cfg(feature = "cosmos")]
fn cosmos_host() -> String {
    var_os("WG_COSMOS_HOST")
        .expect("Expected WG_COSMOS_HOST to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_HOST to be a string.")
        .to_owned()
}

#[cfg(feature = "cosmos")]
fn cosmos_port() -> u16 {
    var_os("WG_COSMOS_PORT")
        .expect("Expected WG_COSMOS_PORT to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_PORT to be a string.")
        .parse::<u16>()
        .expect("Expected WG_COSMOS_PORT to be a u16.")
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_login() -> String {
    var_os("WG_COSMOS_LOGIN")
        .expect("Expected WG_COSMOS_LOGIN to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_LOGIN to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_pass() -> String {
    var_os("WG_COSMOS_PASS")
        .expect("Expected WG_COSMOS_PASS to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_PASS to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_url() -> String {
    var_os("WG_NEO4J_URL")
        .expect("Expected WG_NEO4J_URL to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_URL to be a string.")
        .to_owned()
}

#[allow(dead_code)]
fn server_addr() -> String {
    let port = match var_os("WG_SAMPLE_PORT") {
        None => 5000,
        Some(os) => os.to_str().unwrap_or("5000").parse::<u16>().unwrap_or(5000),
    };

    format!("127.0.0.1:{}", port)
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
fn neo4j_server_addr() -> String {
    "127.0.0.1:5000".to_string()
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_server_addr() -> String {
    "127.0.0.1:5001".to_string()
}

// Rust's dead code detection seems not to process all integration test crates,
// leading to a false positive on this function.
#[allow(dead_code)]
fn gql_endpoint() -> String {
    format!("http://{}/graphql", server_addr())
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
fn neo4j_gql_endpoint() -> String {
    format!("http://{}/graphql", neo4j_server_addr())
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
fn cosmos_gql_endpoint() -> String {
    format!("http://{}/graphql", cosmos_server_addr())
}

#[allow(dead_code)]
fn load_config(config: &str) -> Configuration {
    let cf = File::open(config).expect("Could not open test model config file.");
    let cr = BufReader::new(cf);
    serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_test_client(config_path: &str) -> Client<AppGlobalCtx, AppRequestCtx> {
    // load config
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = Neo4jEndpoint::from_env().unwrap().pool().unwrap();

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

    let engine = Engine::<AppGlobalCtx, AppRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .with_global_ctx(global_ctx)
        .with_resolvers(resolvers.clone())
        .with_validators(validators.clone())
        .with_extensions(extensions.clone())
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_local(engine)
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
pub(crate) fn cosmos_test_client(config_path: &str) -> Client<AppGlobalCtx, AppRequestCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = CosmosEndpoint::from_env().unwrap().pool().unwrap();

    // create app context
    let global_ctx = AppGlobalCtx {
        version: "0.0.0".to_owned(),
    };

    let engine = Engine::<AppGlobalCtx, AppRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .with_global_ctx(global_ctx)
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_local(engine)
}

#[cfg(feature = "cosmos")]
#[allow(dead_code)]
fn clear_cosmos_db() {
    // g.V().drop() -- delete the entire graph
    let client = GremlinClient::connect(
        ConnectionOptions::builder()
            .host(cosmos_host())
            .port(cosmos_port())
            .pool_size(1)
            .ssl(true)
            .serializer(GraphSON::V2)
            .credentials(&cosmos_login(), &cosmos_pass())
            .build(),
    )
    .expect("Expected successful gremlin client creation.");

    let results = client.execute("g.V().drop()", &[]);

    trace!("{:#?}", results);
}

#[cfg(feature = "neo4j")]
#[allow(dead_code)]
fn clear_neo4j_db() {
    let graph = GraphClient::connect(neo4j_url()).unwrap();
    graph.exec("MATCH (n) DETACH DELETE (n)").unwrap();
}

#[allow(dead_code)]
pub(crate) fn clear_db() {
    #[cfg(feature = "cosmos")]
    clear_cosmos_db();

    #[cfg(feature = "neo4j")]
    clear_neo4j_db();
}

#[derive(Clone, Debug)]
pub struct AppGlobalCtx {
    version: String,
}

impl GlobalContext for AppGlobalCtx {}

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

#[allow(dead_code)]
fn name_validator(value: &Value) -> Result<(), Error> {
    let name = match value {
        Value::Map(m) => match m.get("name") {
            Some(n) => n,
            None => {
                return Err(Error::ValidationFailed {
                    message: format!(
                        "Input validator for {field_name} failed.",
                        field_name = "name"
                    ),
                })
            }
        },
        _ => {
            return Err(Error::ValidationFailed {
                message: format!(
                    "Input validator for {field_name} failed.",
                    field_name = "name"
                ),
            })
        }
    };

    match name {
        Value::String(s) => {
            if s == "KENOBI" {
                Err(Error::ValidationFailed {
                    message: format!(
                        "Input validator for {field_name} failed. Cannot be named KENOBI",
                        field_name = "name"
                    ),
                })
            } else {
                Ok(())
            }
        }
        _ => Err(Error::ValidationFailed {
            message: format!(
                "Input validator for {field_name} failed.",
                field_name = "name"
            ),
        }),
    }
}

#[cfg(feature = "neo4j")]
pub(crate) fn project_count(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    if let DatabasePool::Neo4j(p) = facade.executor().context().pool() {
        let db = p.get()?;
        let query = "MATCH (n:Project) RETURN (n);";
        let results = db.exec(query)?;
        facade.resolve_scalar(results.data.len() as i32)
    } else {
        panic!("Unsupported database.");
    }
}

/// custom endpoint returning scalar_list:
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_tags(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    facade.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
}

/// custom endpoint returning node
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_dev(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    trace!("global_top_dev called");
    let mut hm = HashMap::new();
    hm.insert("name".to_string(), Value::String("Joe".to_string()));
    facade.resolve_node(&facade.create_node("User", hm))
}

/*
/// custom endpoint returning node_list
pub fn global_top_issues(facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>) {
    // TODO: add real database query
    facade.resolve_node_list()
}
*/

/// custom field returning scalar
#[cfg(feature = "neo4j")]
pub(crate) fn project_points(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    facade.resolve_scalar(138)
}

/// custom field returning scalar_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_tags(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    facade.resolve_scalar_list(vec!["cypher", "sql", "neo4j"])
}

/// custom rel returning rel
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_dev(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
) -> ExecutionResult {
    let mut hm = HashMap::new();
    hm.insert("name".to_string(), Value::String("Joe".to_string()));
    facade.resolve_rel(
        &facade
            .create_rel(
                Value::String("1234567890".to_string()),
                None,
                facade.create_node("User", hm),
            )
            .expect("Expected new rel"),
    )
}

/// custom rel returning rel_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_issues(
    facade: ResolverFacade<AppGlobalCtx, AppRequestCtx>,
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
    facade.resolve_rel_list(vec![
        &facade
            .create_rel(
                Value::String("1234567890".to_string()),
                None,
                facade.create_node("Feature", hm1),
            )
            .expect("Expected rel"),
        &facade
            .create_rel(
                Value::String("0987654321".to_string()),
                None,
                facade.create_node("Bug", hm2),
            )
            .expect("Expected rel"),
    ])
}
