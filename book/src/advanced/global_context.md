# Global Context

Warpgrapher's GlobalContext feature enable the creation of global state that is accessible across multiple points within Warpgrapher's event hooks including inside function endpoints. 

`config.yml`

```yaml
version: 1
model: 
  - name: User
    props:
      - name: name
        type: String
endpoints: 
  - name: GetRepoName
    class: Query
    input: null
    output:
      list: false
      type: String
```

`main.rs`

```rust
extern crate warpgrapher;

use warpgrapher::{
    Arguments, Error, ExecutionResult, Executor, GraphQLContext, Info, Neo4jEndpoint, Server,
    Value, WarpgrapherConfig, WarpgrapherResolvers,
};

// define global context struct
#[derive(Clone, Debug)]
pub struct GlobalContext {
    repo_name: String,
}

// define endpoint
pub fn get_repo_name(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<GlobalContext, ()>>,
) -> ExecutionResult {
    let global_ctx = &executor.context().global_ctx.as_ref().unwrap();
    let repo_name = global_ctx.repo_name.clone();
    Ok(Value::scalar(repo_name))
}

fn main() -> Result<(), Error> {
    // define database
    let db = Neo4jEndpoint::from_env("DB_URL")?;

    // load config
    let config = WarpgrapherConfig::from_file("./src/config2.yml".to_string())
        .expect("Could not load config file");

    // initialize global context
    let global_ctx = GlobalContext {
        repo_name: "warpgrapher".to_string(),
    };

    // initialize resolvers
    let mut resolvers = WarpgrapherResolvers::<GlobalContext, ()>::new();
    resolvers.insert("GetRepoName".to_string(), Box::new(get_repo_name));

    // build server with global context type
    let mut server: Server<GlobalContext, ()> = Server::new(config, db)
        .with_bind_port("5001".to_string())
        .with_playground_endpoint("/graphiql".to_owned())
        .with_global_ctx(global_ctx)
        .with_resolvers(resolvers)
        .build()
        .expect("Failed to build server");

    // run server
    println!("Warpgrapher Sample App: http://localhost:5001/graphiql");
    match server.serve(true) {
        Ok(_) => {}
        Err(_) => println!("Failed to start server"),
    }

    Ok(())
}
```

If you query the `GetRepoName` endpoint, the response will return "warpgrapher". 

```
query {
    GetRepoName
}
```

```json
{
  "data": {
    "GetRepoName": "warpgrapher"
  }
}
```
