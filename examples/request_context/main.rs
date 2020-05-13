use std::collections::HashMap;
use warpgrapher::{Engine, Config};
use warpgrapher::engine::neo4j::Neo4jEndpoint;
use warpgrapher::engine::resolvers::{Resolvers, ResolverContext, ExecutionResult};
use warpgrapher::juniper::http::GraphQLRequest;

#[derive(Clone, Debug)]
struct AppRequestContext {
    request_id: String
}

impl warpgrapher::engine::context::RequestContext for AppRequestContext {
    fn new() -> AppRequestContext {

        // generate a random id
        let request_id = "12345678901234567890".to_string();

        AppRequestContext {
            request_id
        }
    }
}

/// This function will return the randomly generated request id
fn resolve_request_debug(context: ResolverContext<(), AppRequestContext>) -> ExecutionResult {
    let request_ctx = context.get_request_context()?;
    context.resolve_scalar(request_ctx.request_id.clone())
}

static CONFIG : &str = "
version: 1
model: []
endpoints:
  - name: RequestDebug
    class: Query
    input: null
    output: 
      type: String
";

fn main() {

    // parse warpgrapher config
    let config = Config::from_string(CONFIG.to_string())
        .expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Neo4jEndpoint::from_env("DB_URL").unwrap();

    // define resolvers
    let mut resolvers = Resolvers::<(), AppRequestContext>::new();
    resolvers.insert("RequestDebug".to_string(), Box::new(resolve_request_debug));

    // create warpgrapher engine
    let engine: Engine<(), AppRequestContext> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // execute query on `GetEnvironment` endpoint
    let request = GraphQLRequest::new(
        "query {
            RequestDebug
        }
        ".to_string(),
        None,
        None
    );
    let metadata = HashMap::new();
    let result = engine.execute(request, metadata).unwrap();

    // verify result
    println!("result: {:#?}", result);
    assert_eq!(
        "12345678901234567890",
        result
        .get("data").unwrap()
        .get("RequestDebug").unwrap()
        .as_str().unwrap(),
    );
}
