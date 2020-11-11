use std::collections::HashMap;
use std::convert::TryFrom;
use tokio::runtime::Runtime;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::juniper::http::GraphQLRequest;
use warpgrapher::Engine;

static CONFIG: &str = "
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
";

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

    // create warpgrapher engine
    let engine: Engine<()> = Engine::new(config, db)
        .build()
        .expect("Failed to build engine");

    // execute graphql mutation to create new user
    let request = GraphQLRequest::new(
        "mutation {
            UserCreate(input: {
                email: \"a@b.com\"
            }) {
                id
                email
            }
        }
        "
        .to_string(),
        None,
        None,
    );
    let metadata = HashMap::new();
    let result = engine.execute(&request, &metadata).unwrap();

    // display result
    println!("result: {:#?}", result);
}
