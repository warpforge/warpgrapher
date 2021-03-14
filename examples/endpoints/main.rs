use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::context::RequestContext;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::engine::resolvers::{ExecutionResult, ResolverFacade, Resolvers};
use warpgrapher::engine::value::Value;
use warpgrapher::juniper::BoxFuture;
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
  - name: TopIssue
    class: Query
    input: null
    output:
      type: Issue
";

#[derive(Clone, Debug)]
struct AppRequestContext {}

impl RequestContext for AppRequestContext {
    type DBEndpointType = Neo4jEndpoint;
    fn new() -> AppRequestContext {
        AppRequestContext {}
    }
}

// endpoint returning a list of `Issue` nodes
fn resolve_top_issue(facade: ResolverFacade<AppRequestContext>) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        let top_issue = facade.node(
            "Issue",
            hashmap! {
                "name".to_string() => Value::from("Learn more rust".to_string()),
                "points".to_string() => Value::from(Into::<i64>::into(5))
            },
        );

        facade.resolve_node(&top_issue).await
    })
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
    let mut resolvers = Resolvers::<AppRequestContext>::new();
    resolvers.insert("TopIssue".to_string(), Box::new(resolve_top_issue));

    // create warpgrapher engine
    let engine: Engine<AppRequestContext> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // create new project
    let query = "
        query {
            TopIssue {
                name
                points
            }
        }
    ".to_string();
    let metadata = HashMap::new();
    let result = engine.execute(query, None, metadata).await.unwrap();

    // verify result
    println!("result: {:#?}", result);
}
