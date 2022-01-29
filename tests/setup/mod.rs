#[cfg(feature = "gremlin")]
use gremlin_client::TlsOptions;
#[cfg(feature = "gremlin")]
use gremlin_client::{ConnectionOptions, GraphSON, GremlinClient};
#[cfg(any(feature = "neo4j"))]
use log::trace;
#[cfg(feature = "neo4j")]
use std::collections::HashMap;
#[cfg(feature = "neo4j")]
use std::convert::TryFrom;
#[cfg(any(feature = "gremlin", feature = "neo4j"))]
use std::convert::TryInto;
#[cfg(feature = "gremlin")]
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
#[cfg(any(feature = "gremlin", feature = "neo4j"))]
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::env_bool;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::gremlin::GremlinEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::neo4j::Neo4jTransaction;
#[cfg(any(feature = "gremlin", feature = "neo4j"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::QueryResult;
#[cfg(feature = "neo4j")]
use warpgrapher::engine::database::{DatabasePool, Transaction};
#[cfg(feature = "neo4j")]
use warpgrapher::engine::events::EventHandlerBag;
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
#[cfg(any(feature = "gremlin", feature = "neo4j"))]
use warpgrapher::{Client, Engine};
use warpgrapher::{Configuration, Error};

#[allow(dead_code)]
pub(crate) fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
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
    env_bool("WG_GREMLIN_VALIDATE_CERTS").unwrap_or(true)
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn bolt_transaction() -> Result<Neo4jTransaction, Error> {
    let endpoint = Neo4jEndpoint::from_env()?;
    let pool = endpoint.pool().await?;

    pool.transaction().await
}

#[allow(dead_code)]
fn load_config(config: &str) -> Configuration {
    let cf = File::open(config).expect("Could not open test model config file.");
    let cr = BufReader::new(cf);
    serde_yaml::from_reader(cr).expect("Could not deserialize configuration file.")
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn neo4j_test_client(config_path: &str) -> Client<Neo4jRequestCtx> {
    neo4j_test_client_with_events(config_path, EventHandlerBag::new()).await
}

#[allow(dead_code)]
#[cfg(feature = "neo4j")]
pub(crate) async fn neo4j_test_client_with_events(
    config_path: &str,
    ehb: EventHandlerBag<Neo4jRequestCtx>,
) -> Client<Neo4jRequestCtx> {
    // load config
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = Neo4jEndpoint::from_env().unwrap().pool().await.unwrap();

    // load resolvers
    let mut resolvers: Resolvers<Neo4jRequestCtx> = Resolvers::new();
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
    //let metadata_extension: MetadataExtension<Neo4jRequestCtx> = MetadataExtension::new();
    //let extensions: Extensions<Neo4jRequestCtx> = vec![Arc::new(metadata_extension)];

    let engine = Engine::<Neo4jRequestCtx>::new(config, database_pool)
        .with_version("1.0".to_string())
        .with_resolvers(resolvers.clone())
        .with_validators(validators.clone())
        //.with_extensions(extensions.clone())
        .with_event_handlers(ehb)
        .build()
        .expect("Could not create warpgrapher engine");

    Client::new_with_engine(engine, None)
}

#[allow(dead_code)]
#[cfg(feature = "gremlin")]
pub(crate) async fn gremlin_test_client(config_path: &str) -> Client<GremlinRequestCtx> {
    // load config
    //let config_path = "./tests/fixtures/config.yml".to_string();
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = GremlinEndpoint::from_env().unwrap().pool().await.unwrap();

    let engine = Engine::<GremlinRequestCtx>::new(config, database_pool)
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
    bolt_transaction()
        .await
        .expect("Failed to get database client")
        .execute_query::<Neo4jRequestCtx>(
            "MATCH (n) DETACH DELETE (n);".to_string(),
            HashMap::new(),
        )
        .await
        .expect("Expected successful query run.");
}

#[allow(dead_code)]
pub(crate) async fn clear_db() {
    #[cfg(feature = "gremlin")]
    clear_gremlin_db();

    #[cfg(feature = "neo4j")]
    clear_neo4j_db().await;
}

#[derive(Clone, Debug)]
pub struct Metadata {
    #[allow(dead_code)]
    pub(crate) src_ip: String,
    #[allow(dead_code)]
    pub(crate) src_useragent: String,
}

#[cfg(feature = "neo4j")]
#[derive(Clone, Debug)]
pub struct Neo4jRequestCtx {
    #[allow(dead_code)]
    metadata: Metadata,
}

#[cfg(feature = "neo4j")]
impl RequestContext for Neo4jRequestCtx {
    type DBEndpointType = Neo4jEndpoint;
    fn new() -> Neo4jRequestCtx {
        Neo4jRequestCtx {
            metadata: Metadata {
                src_ip: "".to_string(),
                src_useragent: "".to_string(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct GremlinRequestCtx {
    #[allow(dead_code)]
    metadata: Metadata,
}

#[cfg(feature = "gremlin")]
impl RequestContext for GremlinRequestCtx {
    type DBEndpointType = GremlinEndpoint;
    fn new() -> GremlinRequestCtx {
        GremlinRequestCtx {
            metadata: Metadata {
                src_ip: "".to_string(),
                src_useragent: "".to_string(),
            },
        }
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
pub(crate) fn project_count(facade: ResolverFacade<Neo4jRequestCtx>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let query = "MATCH (n:Project) RETURN (n);".to_string();
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        if let QueryResult::Neo4j(records) = transaction
            .execute_query::<Neo4jRequestCtx>(query, HashMap::new())
            .await?
        {
            transaction.commit().await?;
            std::mem::drop(transaction);
            facade.resolve_scalar(records.len() as i32)
        } else {
            Err(warpgrapher::Error::TypeNotExpected { details: None }.into())
        }
    })
}

/// custom endpoint returning scalar_list:
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_tags(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        facade.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
    })
}

/// custom endpoint returning node
#[cfg(feature = "neo4j")]
pub(crate) fn global_top_dev(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        trace!("global_top_dev called");
        let mut hm = HashMap::new();
        hm.insert("name".to_string(), Value::String("Joe".to_string()));
        facade.resolve_node(&facade.node("User", hm)).await
    })
}

/// custom field returning scalar
#[cfg(feature = "neo4j")]
pub(crate) fn project_points(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar(138) })
}

/// custom field returning scalar_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_tags(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar_list(vec!["cypher", "sql", "neo4j"]) })
}

/// custom rel returning rel
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_dev(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        let query = "MATCH (n:User) RETURN (n);".to_string();
        let qr = transaction
            .execute_query::<Neo4jRequestCtx>(query, HashMap::new())
            .await
            .expect("Expected successful query run.");

        if let QueryResult::Neo4j(result) = qr {
            let dev_id = if let bolt_proto::value::Value::Node(n) =
                &result.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                panic!("Expected node.")
            };

            transaction.commit().await?;
            std::mem::drop(transaction);

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
            Err(warpgrapher::Error::TypeNotExpected { details: None }.into())
        }
    })
}

/// custom rel returning rel_list
#[cfg(feature = "neo4j")]
pub(crate) fn project_top_issues(
    facade: ResolverFacade<Neo4jRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        let query = "MATCH (n:Bug) RETURN (n);".to_string();

        let qr = transaction
            .execute_query::<Neo4jRequestCtx>(query, HashMap::new())
            .await?;

        let bug_id = if let QueryResult::Neo4j(records) = qr {
            if let bolt_proto::value::Value::Node(n) =
                &records.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                transaction.rollback().await?;
                panic!("Expected node.")
            }
        } else {
            transaction.rollback().await?;
            panic!("Expected Neo4j records");
        };

        let query = "MATCH (n:Feature) RETURN (n);".to_string();
        let qr = transaction
            .execute_query::<Neo4jRequestCtx>(query, HashMap::new())
            .await?;

        let feature_id = if let QueryResult::Neo4j(records) = qr {
            if let bolt_proto::value::Value::Node(n) =
                &records.get(0).expect("Expected result").fields()[0]
            {
                Value::try_from(n.properties().get("id").expect("Expected id").clone())
                    .expect("Expected string")
            } else {
                transaction.rollback().await?;
                panic!("Expected node.")
            }
        } else {
            transaction.rollback().await?;
            panic!("Expected Neo4j records");
        };

        transaction.commit().await?;
        std::mem::drop(transaction);

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
    })
}
