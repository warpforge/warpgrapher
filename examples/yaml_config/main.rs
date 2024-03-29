use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::cypher::CypherEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::Engine;

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
    let config_file = File::open("config.yaml").expect("Could not read file");
    let config = Configuration::try_from(config_file).expect("Failed to parse config file");

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
