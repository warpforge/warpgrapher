#[cfg(feature = "gremlin")]
use gremlin_client::TlsOptions;
#[cfg(feature = "gremlin")]
use gremlin_client::{ConnectionOptions, GraphSON, GremlinClient};
use log::trace;
#[cfg(feature = "cypher")]
use std::collections::HashMap;
#[cfg(feature = "cypher")]
use std::convert::TryFrom;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
use std::convert::TryInto;
#[cfg(feature = "gremlin")]
use std::env::var_os;
use std::fs::File;
use std::io::BufReader;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::cypher::CypherEndpoint;
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::cypher::CypherTransaction;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::env_bool;
#[cfg(feature = "gremlin")]
use warpgrapher::engine::database::gremlin::GremlinEndpoint;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
use warpgrapher::engine::database::DatabaseEndpoint;
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::QueryResult;
#[cfg(feature = "cypher")]
use warpgrapher::engine::database::{DatabasePool, Transaction};
#[cfg(feature = "cypher")]
use warpgrapher::engine::events::EventHandlerBag;
#[cfg(feature = "cypher")]
use warpgrapher::engine::objects::Options;
#[cfg(feature = "cypher")]
use warpgrapher::engine::resolvers::ExecutionResult;
#[cfg(feature = "cypher")]
use warpgrapher::engine::resolvers::ResolverFacade;
#[cfg(feature = "cypher")]
use warpgrapher::engine::resolvers::Resolvers;
#[cfg(feature = "cypher")]
use warpgrapher::engine::validators::Validators;
use warpgrapher::engine::value::Value;
#[cfg(feature = "cypher")]
use warpgrapher::juniper::BoxFuture;
#[cfg(any(feature = "gremlin", feature = "cypher"))]
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
#[cfg(feature = "cypher")]
pub(crate) async fn bolt_transaction() -> Result<CypherTransaction, Error> {
    let endpoint = CypherEndpoint::from_env()?;
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
#[cfg(feature = "cypher")]
pub(crate) async fn cypher_test_client(config_path: &str) -> Client<CypherRequestCtx> {
    cypher_test_client_with_events(config_path, EventHandlerBag::new()).await
}

#[allow(dead_code)]
#[cfg(feature = "cypher")]
pub(crate) async fn cypher_test_client_with_events(
    config_path: &str,
    ehb: EventHandlerBag<CypherRequestCtx>,
) -> Client<CypherRequestCtx> {
    // load config
    let config: Configuration = File::open(config_path)
        .expect("Failed to load config file")
        .try_into()
        .unwrap();

    let database_pool = CypherEndpoint::from_env().unwrap().pool().await.unwrap();

    // load resolvers
    let mut resolvers: Resolvers<CypherRequestCtx> = Resolvers::new();
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
    //let metadata_extension: MetadataExtension<CypherRequestCtx> = MetadataExtension::new();
    //let extensions: Extensions<CypherRequestCtx> = vec![Arc::new(metadata_extension)];

    let engine = Engine::<CypherRequestCtx>::new(config, database_pool)
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
    let version = match var_os("WG_GREMLIN_VERSION")
        .map(|osstr| osstr.to_string_lossy().into_owned())
        .unwrap_or_else(|| "3".to_string())
        .parse::<u16>()
        .unwrap_or(3)
    {
        1 => GraphSON::V1,
        2 => GraphSON::V2,
        _ => GraphSON::V3,
    };

    let mut options_builder = ConnectionOptions::builder()
        .host(gremlin_host())
        .port(gremlin_port())
        .pool_size(num_cpus::get().try_into().unwrap_or(8))
        .serializer(version.clone())
        .deserializer(version);
    if let (Some(user), Some(pass)) = (gremlin_user(), gremlin_pass()) {
        options_builder = options_builder.credentials(&user, &pass);
    }
    if gremlin_use_tls() {
        options_builder = options_builder.ssl(true).tls_options(TlsOptions {
            accept_invalid_certs: gremlin_accept_invalid_tls(),
        });
    }
    let options = options_builder.build();
    trace!("Test connection with options: {:#?}", options);
    let client =
        GremlinClient::connect(options).expect("Expected successful gremlin client creation.");
    let _ = client.execute("g.V().drop()", &[]);
}

#[cfg(feature = "cypher")]
#[allow(dead_code)]
async fn clear_cypher_db() {
    bolt_transaction()
        .await
        .expect("Failed to get database client")
        .execute_query::<CypherRequestCtx>(
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

    #[cfg(feature = "cypher")]
    clear_cypher_db().await;
}

#[derive(Clone, Debug)]
pub struct Metadata {
    #[allow(dead_code)]
    pub(crate) src_ip: String,
    #[allow(dead_code)]
    pub(crate) src_useragent: String,
}

#[cfg(feature = "cypher")]
#[derive(Clone, Debug)]
pub struct CypherRequestCtx {
    #[allow(dead_code)]
    metadata: Metadata,
}

#[cfg(feature = "cypher")]
impl RequestContext for CypherRequestCtx {
    type DBEndpointType = CypherEndpoint;
    fn new() -> CypherRequestCtx {
        CypherRequestCtx {
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

#[cfg(feature = "cypher")]
pub(crate) fn project_count(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let query = "MATCH (n:Project) RETURN (n);".to_string();
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        if let QueryResult::Cypher(records) = transaction
            .execute_query::<CypherRequestCtx>(query, HashMap::new())
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
#[cfg(feature = "cypher")]
pub(crate) fn global_top_tags(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        facade.resolve_scalar_list(vec!["web", "database", "rust", "python", "graphql"])
    })
}

/// custom endpoint returning node
#[cfg(feature = "cypher")]
pub(crate) fn global_top_dev(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        trace!("global_top_dev called");
        let mut hm = HashMap::new();
        hm.insert("name".to_string(), Value::String("Joe".to_string()));
        facade.resolve_node(&facade.node("User", hm)).await
    })
}

/// custom field returning scalar
#[cfg(feature = "cypher")]
pub(crate) fn project_points(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar(138) })
}

/// custom field returning scalar_list
#[cfg(feature = "cypher")]
pub(crate) fn project_top_tags(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move { facade.resolve_scalar_list(vec!["cypher", "sql", "neo4j"]) })
}

/// custom rel returning rel
#[cfg(feature = "cypher")]
pub(crate) fn project_top_dev(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        let query = "MATCH (n:User) RETURN (n);".to_string();
        let qr = transaction
            .execute_query::<CypherRequestCtx>(query, HashMap::new())
            .await
            .expect("Expected successful query run.");

        if let QueryResult::Cypher(result) = qr {
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
                            "topdev",
                            HashMap::new(),
                            dev_id,
                            Options::default(),
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
#[cfg(feature = "cypher")]
pub(crate) fn project_top_issues(
    facade: ResolverFacade<CypherRequestCtx>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let mut transaction = facade.executor().context().pool().transaction().await?;
        transaction.begin().await?;
        let query = "MATCH (n:Bug) RETURN (n);".to_string();

        let qr = transaction
            .execute_query::<CypherRequestCtx>(query, HashMap::new())
            .await?;

        let bug_id = if let QueryResult::Cypher(records) = qr {
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
            panic!("Expected Cypher records");
        };

        let query = "MATCH (n:Feature) RETURN (n);".to_string();
        let qr = transaction
            .execute_query::<CypherRequestCtx>(query, HashMap::new())
            .await?;

        let feature_id = if let QueryResult::Cypher(records) = qr {
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
            panic!("Expected Cypher records");
        };

        transaction.commit().await?;
        std::mem::drop(transaction);

        facade
            .resolve_rel_list(vec![
                &facade
                    .create_rel(
                        Value::String("1234567890".to_string()),
                        "topissues",
                        HashMap::new(),
                        bug_id,
                        Options::default(),
                    )
                    .expect("Expected rel"),
                &facade
                    .create_rel(
                        Value::String("0987654321".to_string()),
                        "topissues",
                        HashMap::new(),
                        feature_id,
                        Options::default(),
                    )
                    .expect("Expected rel"),
            ])
            .await
    })
}
