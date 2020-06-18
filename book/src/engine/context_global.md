# Global Context

Warpgrapher's GlobalContext feature enable the creation of global state that is accessible across multiple points within Warpgrapher's event hooks including inside function endpoints. The example below will demonstrate how to set a `tenant_id` variable in the global context that denotes under which account the app is running. The app also creates a defined endpoint that returns that value to the caller.

## Usage

#### 1. Define GlobalContext struct

```rust
#[derive(Clone, Debug, Sync, Send)]
struct AppGlobalContext {
    tenant_id: String
}
```

The struct must implement `Clone`, `Debug`, `Sync`, and `Send`.

#### 2. Create Engine with GlobalContext type parameter and option

The GlobalContext type is specified in the first type parameter of `Engine`. 

```rust
let global_ctx = AppGlobalContext {
    tenant_id: "123456".to_string();
}

let engine: Engine<AppGlobalContext, ()> = Engine::new(config, db)
    .with_global_ctx(global_ctx)
    .build();
```


#### 3. Use GlobalContext in a resolver

```rust
fn resolve(facade: ResolverFacade<AppGlobalContext, ()>) -> ExecutionResult {
    let global_ctx = facade.global_context()?;

    // use global_ctx
}
```

## Full Example

`main.rs`

```rust
use std::collections::HashMap;
use warpgrapher::{Engine, Config};
use warpgrapher::engine::databases::neo4j::Neo4jEndpoint;
use warpgrapher::engine::resolvers::{Resolvers, ResolverFacade, ExecutionResult};
use warpgrapher::GraphQLRequest;

static CONFIG : &'static str = "
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
  - name: Team
    props:
      - name: name
        type: String
    rels:
      - name: users
        nodes: [User]
endpoints:
  - name: GetEnvironment
    class: Query
    input: null
    output: 
      type: String
";

#[derive(Clone, Debug)]
struct AppGlobalContext {
    tenant_id: String
}

fn resolve_get_environment(facade: ResolverFacade<AppGlobalContext, ()>) -> ExecutionResult {
    let global_ctx = facade.global_context()?;
    facade.resolve_scalar(global_ctx.tenant_id.clone())
}

fn main() {

    // parse warpgrapher config
    let config = Config::from_string(CONFIG.to_string())
        .expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Neo4jEndpoint::from_env().unwrap();

    // define global context
    let global_ctx = AppGlobalContext {
        tenant_id: "123456".to_string()
    };

    // define resolvers
    let mut resolvers = Resolvers::<AppGlobalContext, ()>::new();
    resolvers.insert("GetEnvironment".to_string(), Box::new(resolve_get_environment));

    // create warpgrapher engine
    let engine: Engine<AppGlobalContext, ()> = Engine::new(config, db.pool().unwrap())
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
```
