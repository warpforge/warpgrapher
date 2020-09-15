use std::collections::HashMap;
use std::convert::TryFrom;
use tokio::runtime::Runtime;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::GlobalContext;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade, Resolvers};
use warpgrapher::juniper::http::GraphQLRequest;
use warpgrapher::Engine;

static CONFIG: &str = "
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
endpoints:
  - name: GetTenant
    class: Query
    input: null
    output: 
      type: String
";

#[derive(Clone, Debug)]
struct AppGlobalContext {
    tenant_id: String,
}

impl GlobalContext for AppGlobalContext {}

fn resolve_get_tenant(facade: ResolverFacade<AppGlobalContext, ()>) -> ExecutionResult {
    let global_context = match facade.global_context() {
        Some(v) => v,
        None => {
            return facade.resolve_null();
        }
    };
    let tenant_id = global_context.tenant_id.clone();
    facade.resolve_scalar(tenant_id)
}

fn main() {
    // parse warpgrapher config
    let config = Configuration::try_from(CONFIG.to_string()).expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Runtime::new()
        .expect("Expected tokio runtime.")
        .block_on(
            Neo4jEndpoint::from_env()
                .expect("Failed to parse neo4j endpoint from environment")
                .pool(),
        )
        .expect("Failed to create neo4j database pool");
    // define global context
    let global_ctx = AppGlobalContext {
        tenant_id: "123456".to_string(),
    };

    // define resolvers
    let mut resolvers = Resolvers::<AppGlobalContext, ()>::new();
    resolvers.insert("GetTenant".to_string(), Box::new(resolve_get_tenant));

    // create warpgrapher engine
    let engine: Engine<AppGlobalContext, ()> = Engine::new(config, db)
        .with_global_ctx(global_ctx)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // execute query on `GetEnvironment` endpoint
    let request = GraphQLRequest::new(
        "query {
            GetTenant
        }
        "
        .to_string(),
        None,
        None,
    );
    let metadata = HashMap::new();
    let result = engine.execute(&request, &metadata).unwrap();

    // verify result
    println!("result: {:#?}", result);
    assert_eq!(
        "123456",
        result
            .get("data")
            .unwrap()
            .get("GetTenant")
            .unwrap()
            .as_str()
            .unwrap(),
    );
}
