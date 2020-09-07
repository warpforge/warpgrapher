use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade, Resolvers};
use warpgrapher::engine::value::Value;
use warpgrapher::juniper::http::GraphQLRequest;
use warpgrapher::Engine;

static CONFIG: &str = "
version: 1
model: 
 - name: Issue
   props: 
    - name: name
      type: String 
    - name: points
      type: Int 
endpoints:

";

// endpoint returning a list of `Issue` nodes
fn resolve_top_issue(facade: ResolverFacade<(), ()>) -> ExecutionResult {
    let top_issue = facade.create_node(
        "Issue",
        hashmap! {
            "name".to_string() => Value::from("Learn more rust".to_string()),
            "point".to_string() => Value::from(5 as i64)
        },
    );

    facade.resolve_node(&top_issue)
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
    let mut resolvers = Resolvers::<(), ()>::new();
    resolvers.insert(
        "resolve_project_points".to_string(),
        Box::new(resolve_top_issue),
    );

    // create warpgrapher engine
    let engine: Engine<(), ()> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // create new project
    let request = GraphQLRequest::new(
        "mutation {
            ProjectCreate(input: {
                name: \"Project1\"
            }) {
                id
                points
            }
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
}
