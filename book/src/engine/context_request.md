# Request Context

Warpgrapher Request Contexts enables the creation of mutable state through the lifecycle of a request including extensions hooks and resolvers.  

## Usage

#### 1. Define RequestContext struct

The request context must implement `Clone`, `Debug`, `Sync`, `Send`, and Warpgrapher `RequestContext`. 

```rust
#[derive(Clone, Debug)]
struct AppRequestContext {
    request_id: String
}

impl warpgrapher::engine::context::RequestContext for AppRequestContext {
    fn new() -> AppRequestContext {

        // initialize context ...

        AppRequestContext {
            request_id
        }
    }
}
```

#### 2. Create Engine with RequestContext type parameter

The RequestContext is specified in the second type paramter of `Engine`. 

```rust
let engine: Engine<(), AppRequestContext> = Engine::new(config, db)
    .build();
```

#### 3. Access Context inside resolver

```rust
fn resolve(facade: ResolverFacade<(), AppRequestContext>) -> ExecutionResult {
    let request_ctx = facade.request_context()?;

    // use request_ctx
}
```

## Full Example

```rust
use std::collections::HashMap;
use warpgrapher::{Engine, Config};
use warpgrapher::engine::databases::neo4j::Neo4jEndpoint;
use warpgrapher::engine::resolvers::{Resolvers, ResolverFacade, ExecutionResult};
use warpgrapher::GraphQLRequest;

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
fn resolve_request_debug(context: ResolverFacade<(), AppRequestContext>) -> ExecutionResult {
    let request_ctx = context.request_context()?;
    context.resolve_scalar(request_ctx.request_id.clone())
}

static CONFIG : &'static str = "
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
    let db = Neo4jEndpoint::from_env().unwrap();

    // define resolvers
    let mut resolvers = Resolvers::<(), AppRequestContext>::new();
    resolvers.insert("RequestDebug".to_string(), Box::new(resolve_request_debug));

    // create warpgrapher engine
    let engine: Engine<(), AppRequestContext> = Engine::new(config, db.pool().unwrap())
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
```