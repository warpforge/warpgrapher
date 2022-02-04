use std::collections::HashMap;
use std::convert::TryFrom;
use uuid::Uuid;
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
 - name: User
   props:
    - name: name
      type: String
 - name: Project
   props: 
    - name: name
      type: String 
   rels:
     - name: top_contributor
       nodes: [User]
       resolver: resolve_project_top_contributor
";

#[derive(Clone, Debug)]
struct AppRequestContext {}

impl RequestContext for AppRequestContext {
    type DBEndpointType = Neo4jEndpoint;
    fn new() -> AppRequestContext {
        AppRequestContext {}
    }
}

fn resolve_project_top_contributor(
    facade: ResolverFacade<AppRequestContext>,
) -> BoxFuture<ExecutionResult> {
    Box::pin(async move {
        // create dynamic dst node
        let mut top_contributor_props = HashMap::<String, Value>::new();
        top_contributor_props.insert(
            "id".to_string(),
            Value::from(Uuid::new_v4().to_hyphenated().to_string()),
        );
        top_contributor_props.insert("name".to_string(), Value::from("user0".to_string()));
        let top_contributor = facade.node("User", top_contributor_props);

        // create dynamic rel
        let rel_id = "1234567890".to_string();
        let top_contributor_rel = facade.create_rel_with_dst_node(
            Value::from(rel_id),
            "topdev",
            HashMap::new(),
            top_contributor,
        )?;

        facade.resolve_rel(&top_contributor_rel).await
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
    resolvers.insert(
        "resolve_project_top_contributor".to_string(),
        Box::new(resolve_project_top_contributor),
    );

    // create warpgrapher engine
    let engine: Engine<AppRequestContext> = Engine::new(config, db)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build engine");

    // create new project
    let query = "
        mutation {
            ProjectCreate(input: {
                name: \"Project1\"
            }) {
                id
                top_contributor {
                    dst {
                        ... on User {
                            id
                            name
                        }
                    }
                }
            }
        }
    "
    .to_string();
    let metadata = HashMap::new();
    let result = engine.execute(query, None, metadata).await.unwrap();

    // verify result
    println!("result: {:#?}", result);
}
