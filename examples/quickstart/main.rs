use std::collections::HashMap;
use std::convert::TryFrom;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::cypher::CypherEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::Engine;

static CONFIG: &str = "
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
";

#[derive(Clone, Debug)]
struct AppRequestContext {}

impl RequestContext for AppRequestContext {
    type DBEndpointType = CypherEndpoint;
    fn new() -> AppRequestContext {
        AppRequestContext {}
    }
}

#[tokio::main]
async fn main() {
    // parse warpgrapher config
    let config = Configuration::try_from(CONFIG.to_string()).expect("Failed to parse CONFIG");

    // define database endpoint
    let db = CypherEndpoint::from_env()
        .expect("Failed to parse cypher endpoint from environment")
        .pool()
        .await
        .expect("Failed to create cypher database pool");

    // create warpgrapher engine
    let engine: Engine<AppRequestContext> = Engine::new(config, db)
        .build()
        .expect("Failed to build engine");

    // execute graphql mutation to create new user
    let query = "
        mutation {
            UserCreate(input: {
                email: \"a@b.com\"
            }) {
                id
                email
            }
        }
    "
    .to_string();
    let metadata = HashMap::new();
    let result = engine.execute(query, None, metadata).await.unwrap();

    // display result
    println!("result: {:#?}", result);
}
