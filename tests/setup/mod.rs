mod extension;

#[cfg(feature = "neo4j")]
use bolt_proto::Message;
#[cfg(feature = "neo4j")]
use extension::MetadataExtension;
use extension::{Metadata, MetadataExtensionCtx};
#[cfg(feature = "gremlin")]
use gremlin_client::TlsOptions;
#[cfg(any(feature = "cosmos", feature = "gremlin"))]
use gremlin_client::{ConnectionOptions, GraphSON, GremlinClient};
#[cfg(any(feature = "neo4j"))]
use log::trace;
#[cfg(feature = "neo4j")]
use std::collections::HashMap;
#[cfg(feature = "neo4j")]
use std::convert::TryFrom;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use std::convert::TryInto;
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
#[cfg(feature = "neo4j")]
use std::iter::FromIterator;
#[cfg(feature = "neo4j")]
use std::sync::Arc;
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::env_bool;
#[cfg(feature = "cosmos")]
use warpgrapher::engine::database::gremlin::CosmosEndpoint;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::gremlin::GremlinEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::DatabasePool;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::events::EventHandlerBag;
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
#[cfg(feature = "neo4j")]
use warpgrapher::juniper::BoxFuture;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
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
fn cosmos_user() -> String {
    var_os("WG_COSMOS_USER")
        .expect("Expected WG_COSMOS_USER to be set.")
        .to_str()
        .expect("Expected WG_COSMOS_USER to be a string.")
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

#[cfg(feature = "gremlin")]
fn gremlin_host() -> String {
    var_os("WG_GREMLIN_HOST")
        .expect("Expected WG_GREMLIN_HOST to be set.")
        .to_str()
        .expect("Expected WG_GREMLIN_HOST to be a string.")
        .to_owned()
}

#[cfg(feature = "gremlin")]
fn gremlin_port() -> u16 {
    var_os("WG_GREMLIN_PORT")
        .expect("Expected WG_GREMLIN_PORT to be set.")
        .to_str()
        .expect("Expected WG_GREMLIN_PORT to be a string.")
        .parse::<u16>()
        .expect("Expected WG_GREMLIN_PORT to be a u16.")
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
fn gremlin_user() -> Option<String> {
    var_os("WG_GREMLIN_USER").map(|osstr| osstr.to_string_lossy().into_owned())
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
fn gremlin_pass() -> Option<String> {
    var_os("WG_GREMLIN_PASS").map(|osstr| osstr.to_string_lossy().into_owned())
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
fn gremlin_use_tls() -> bool {
    env_bool("WG_GREMLIN_USE_TLS").unwrap_or(true)
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
fn gremlin_accept_invalid_tls() -> bool {
    env_bool("WG_GREMLIN_CERT").unwrap_or(true)
}

#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_host() -> String {
    var_os("WG_NEO4J_HOST")
        .expect("Expected WG_NEO4J_HOST to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_HOST to be a string.")
        .to_owned()
}

#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_port() -> u16 {
    var_os("WG_NEO4J_PORT")
        .expect("Expected WG_NEO4J_PORT to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_PORT to be a string.")
        .parse::<u16>()
        .expect("Expected WG_NEO4J_PORT to be a u16.")
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_user() -> String {
    var_os("WG_NEO4J_USER")
        .expect("Expected WG_NEO4J_USER to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_USER to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) fn neo4j_pass() -> String {
    var_os("WG_NEO4J_PASS")
        .expect("Expected WG_NEO4J_PASS to be set.")
        .to_str()
        .expect("Expected WG_NEO4J_PASS to be a string.")
        .to_owned()
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn bolt_client() -> bolt_client::Client {
    let mut graph = bolt_client::Client::new(
        neo4j_host().to_string() + ":" + &neo4j_port().to_string(),
        None as Option<String>,
    )
    .await
    .expect("Expected client.");
    let handshake = graph.handshake(&[4, 0, 0, 0]).await;
    handshake.expect("Expected successful handshake.");

    let hello = graph
        .hello(Some(bolt_client::Metadata::from_iter(vec![
            ("user_agent", "warpgrapher/0.2.0"),
            ("scheme", "basic"),
            ("principal", &neo4j_user()),
            ("credentials", &neo4j_pass()),
        ])))
        .await;

    hello.expect("Expected successful handshake.");

    graph
}

#[allow(dead_code)]
fn load_config(config: &str) -> Configuration {
    let cf = File::open(config).expect("Could not open test model config file.");
    let cr = BufReader::new(cf);
    serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn neo4j_test_client(config_path: &str) -> Client<AppRequestCtx> {
    neo4j_test_client_with_events(config_path, EventHandlerBag::new()).await
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn neo4j_test_client_with_events(
    config_path: &str,
    ehb: EventHandlerBag<AppRequestCtx>,
) -> Client<AppRequestCtx> {
    // load config
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = Neo4jEndpoint::from_env().unwrap().pool().await.unwrap();

    // load resolvers
    let mut resolvers: Resolvers<AppRequestCtx> = Resolvers::new();
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
    let metadata_extension: MetadataExtension<AppRequestCtx> = MetadataExtension::new();
    let extensions: Extensions<AppRequestCtx> = vec![Arc::new(metadata_extension)];

    let engine = Engine::<AppRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .with_resolvers(resolvers.clone())
        .with_validators(validators.clone())
        .with_extensions(extensions.clone())
        .with_event_handlers(ehb)
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_engine(engine, None)
}

#[allow(dead_code)]
#[cfg(feature = "cosmos")]
pub(crate) async fn cosmos_test_client(config_path: &str) -> Client<AppRequestCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = CosmosEndpoint::from_env().unwrap().pool().await.unwrap();

    let engine = Engine::<AppRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_engine(engine, None)
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
            .credentials(&cosmos_user(), &cosmos_pass())
            .build(),
    )
    .expect("Expected successful gremlin client creation.");

    let _ = client.execute("g.V().drop()", &[]);
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
pub(crate) async fn gremlin_test_client(config_path: &str) -> Client<AppRequestCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = GremlinEndpoint::from_env().unwrap().pool().await.unwrap();

    let engine = Engine::<AppRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_engine(engine, None)
}

#[cfg(feature = "gremlin")]
#[allow(dead_code)]
fn clear_gremlin_db() {
    let mut options_builder = ConnectionOptions::builder()
        .host(gremlin_host())
        .port(gremlin_port())
        .pool_size(num_cpus::get().try_into().unwrap_or(8))
        .serializer(GraphSON::V3)
        .deserializer(GraphSON::V3);
    if let (Some(user), Some(pass)) = (gremlin_user(), gremlin_pass()) {
        options_builder = options_builder.credentials(&user, &pass);
    }
    if gremlin_use_tls() {
        options_builder = options_builder.ssl(true).tls_options(TlsOptions {
            accept_invalid_certs: gremlin_accept_invalid_tls(),
        });
    }
    let options = options_builder.build();
    let client =
        GremlinClient::connect(options).expect("Expected successful gremlin client creation.");
    let _ = client.execute("g.V().drop()", &[]);
}

#[cfg(feature = "neo4j")]
#[allow(dead_code)]
async fn clear_neo4j_db() {
    let mut graph = bolt_client().await;
    let result = graph
        .run_with_metadata("MATCH (n) DETACH DELETE (n);", None, None)
        .await;
    result.expect("Expected successful query run.");

    let pull_meta = bolt_client::Metadata::from_iter(vec![("n", 1)]);
    let (_response, _records) = graph
        .pull(Some(pull_meta))
        .await
        .expect("Expected pull to succeed.");
}

#[allow(dead_code)]
pub(crate) async fn clear_db() {
    #[cfg(feature = "cosmos")]
    clear_cosmos_db();

    #[cfg(feature = "gremlin")]
    clear_gremlin_db();

    #[cfg(feature = "neo4j")]
    clear_neo4j_db().await;
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
pub(crate) fn project_count(facade: ResolverFacade<AppRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        if let DatabasePool::Neo4j(p) = facade.executor().context().pool() {
            let mut db = p.get().await?;
            let query = "MATCH (n:Project) RETURN (n);";
            db.run_with_metadata(query, None, None)
                .await
                .expect("Expected successful query run.");
            let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
            let (response, records) = db.pull(Some(pull_meta)).await?;
            match response {
                Message::Success(_) => (),
                message => return Err(Error::Neo4jQueryFailed { message }.into()),
            }

            facade.resolve_scalar(records.len() as i32)
        } else {
            panic!("Unsupported database.");
        }
    })
}

/// custom endpoint returning scalar_list:
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_tags(facade: ResolverFacade<AppRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        facade.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
    })
}

/// custom endpoint returning node
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_dev(facade: ResolverFacade<AppRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        trace!("global_top_dev called");
        let mut hm = HashMap::new();
        hm.insert("name".to_string(), Value::String("Joe".to_string()));
        facade.resolve_node(&facade.create_node("User", hm)).await
    })
}

/// custom field returning scalar
#[cfg(feature = "neo4j")]
pub(crate) fn project_points(facade: ResolverFacade<AppRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar(138) })
}

/// custom field returning scalar_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_tags(
    facade: ResolverFacade<AppRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar_list(vec!["cypher", "sql", "neo4j"]) })
}

/// custom rel returning rel
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_dev(facade: ResolverFacade<AppRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        if let DatabasePool::Neo4j(p) = facade.executor().context().pool() {
            let mut db = p.get().await?;
            let query = "MATCH (n:User) RETURN (n);";
            db.run_with_metadata(query, None, None)
                .await
                .expect("Expected successful query run.");

            let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
            let (response, records) = db.pull(Some(pull_meta)).await?;
            match response {
                Message::Success(_) => (),
                message => return Err(Error::Neo4jQueryFailed { message }.into()),
            }

            let dev_id = if let bolt_proto::value::Value::Node(n) =
                &records.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                panic!("Expected node.")
            };

            std::mem::drop(db);

            facade
                .resolve_rel(
                    &facade
                        .create_rel(
                            Value::String("1234567890".to_string()),
                            None,
                            dev_id,
                            "User",
                        )
                        .expect("Expected new rel"),
                )
                .await
        } else {
            panic!("Unsupported database.");
        }
    })
}

/// custom rel returning rel_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_issues(
    facade: ResolverFacade<AppRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        if let DatabasePool::Neo4j(p) = facade.executor().context().pool() {
            let mut db = p.get().await?;
            let query = "MATCH (n:Bug) RETURN (n);";
            db.run_with_metadata(query, None, None)
                .await
                .expect("Expected successful query run.");

            let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
            let (response, records) = db.pull(Some(pull_meta)).await?;
            match response {
                Message::Success(_) => (),
                message => return Err(Error::Neo4jQueryFailed { message }.into()),
            }

            let bug_id = if let bolt_proto::value::Value::Node(n) =
                &records.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                panic!("Expected node.")
            };

            let query = "MATCH (n:Feature) RETURN (n);";
            db.run_with_metadata(query, None, None)
                .await
                .expect("Expected successful query run.");

            let pull_meta = bolt_client::Metadata::from_iter(vec![("n", -1)]);
            let (response, records) = db.pull(Some(pull_meta)).await?;
            match response {
                Message::Success(_) => (),
                message => return Err(Error::Neo4jQueryFailed { message }.into()),
            }

            let feature_id = if let bolt_proto::value::Value::Node(n) =
                &records.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                panic!("Expected node.")
            };

            std::mem::drop(db);

            facade
                .resolve_rel_list(vec![
                    &facade
                        .create_rel(Value::String("1234567890".to_string()), None, bug_id, "Bug")
                        .expect("Expected rel"),
                    &facade
                        .create_rel(
                            Value::String("0987654321".to_string()),
                            None,
                            feature_id,
                            "Feature",
                        )
                        .expect("Expected rel"),
                ])
                .await
        } else {
            panic!("Unsupported database.");
        }
    })
}
