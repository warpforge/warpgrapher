use std::collections::HashMap;
use warpgrapher::{Engine, Config};
use warpgrapher::engine::neo4j::Neo4jEndpoint;
use warpgrapher::engine::resolvers::{Resolvers, ResolverContext, ExecutionResult};
use warpgrapher::juniper::http::GraphQLRequest;

#[derive(Clone, Debug)]
struct AppGlobalContext {
    tenant_id: String
}

fn resolve_get_tenant(context: ResolverContext<AppGlobalContext, ()>) -> ExecutionResult {
    let global_ctx = context.get_global_context()?;
    context.resolve_scalar(global_ctx.tenant_id.clone())
}

static CONFIG : &str = "
version: 1
model: []
endpoints:
  - name: GetEnvironment
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

    // define global context
    let global_ctx = AppGlobalContext {
        tenant_id: "123456".to_string()
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
            GetEnvironment
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
        "123456",
        result
        .get("data").unwrap()
        .get("GetEnvironment").unwrap()
        .as_str().unwrap(),
    );
}
