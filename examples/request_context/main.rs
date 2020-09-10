use std::collections::HashMap;
use std::convert::TryFrom;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::RequestContext;
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
  - name: EchoRequest
    class: Query
    input: null
    output: 
      type: String
";

#[derive(Clone, Debug)]
struct AppRequestContext {
    request_id: String,
}

impl RequestContext for AppRequestContext {
    fn new() -> AppRequestContext {
        // generate random request id
        let request_id = "1234".to_string();
        AppRequestContext { request_id }
    }
}

fn resolve_echo_request(facade: ResolverFacade<(), AppRequestContext>) -> ExecutionResult {
    let request_context = facade.request_context().unwrap();
    let request_id = request_context.request_id.clone();
    facade.resolve_scalar(format!("echo! (request_id: {})", request_id))
}

#[tokio::main]
async fn main() {
    // parse warpgrapher config
    let config = Configuration::try_from(CONFIG.to_string()).expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Neo4jEndpoint::from_env()
        .expect("Failed to parse neo4j endpoint from environment")
        .pool()
        .await
        .expect("Failed to create neo4j database pool");

    // define resolvers
    let mut resolvers = Resolvers::<(), AppRequestContext>::new();
    resolvers.insert("EchoRequest".to_string(), Box::new(resolve_echo_request));

    // create warpgrapher engine
    let engine: Engine<(), AppRequestContext> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // execute query on `GetEnvironment` endpoint
    let request = GraphQLRequest::new(
        "query {
            EchoRequest
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
        "echo! (request_id: 1234)",
        result
            .get("data")
            .unwrap()
            .get("EchoRequest")
            .unwrap()
            .as_str()
            .unwrap(),
    );
}
