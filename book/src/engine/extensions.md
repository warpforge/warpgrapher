# Extensions

TODO


Warpgrapher's RequestContext feature allows request-specific data to be made available to resolvers. 


`config.yml`

```yaml
version: 1
model: 
  - name: User
    properties:
      - name: name
        type: String
endpoints: 
  - name: WhoAmI
    class: Query
    input: null
    output:
      list: false
      type: String
```

`extension.rs`

This file contains a sample extension for the purpose of demonstrating the use of Request Contexts. Extensions are explored in depth in another chapter. 

```rust
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use warpgrapher::{Extension, Object, Value, WarpgrapherRequestContext};

#[derive(Clone, Debug, Default)]
pub struct UserProfile {
    id: String,
    name: String,
    role: String,
}

impl Into<Value> for UserProfile {
    fn into(self) -> Value {
        let mut o = Object::with_capacity(0);
        o.add_field("id".to_string(), Value::scalar(self.id.clone()));
        o.add_field("name".to_string(), Value::scalar(self.name.clone()));
        o.add_field("role".to_string(), Value::scalar(self.role.clone()));
        Value::Object(o)
    }
}

// sample extension that utilizes the request context
#[derive(Clone)]
pub struct SampleExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static + Clone + Sync + Send + Debug + WarpgrapherRequestContext,
{
    _gctx: PhantomData<GlobalCtx>,
    _rctx: PhantomData<ReqCtx>,
}

// trait that must be implemented by the warpgrapher app's request context.
pub trait SampleExtensionReqContext {
    fn set_user(&mut self, user: UserProfile) -> ();
}

impl<GlobalCtx, ReqCtx> SampleExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static
        + Clone
        + Sync
        + Send
        + Debug
        + WarpgrapherRequestContext
        + SampleExtensionReqContext,
{
    pub fn new() -> SampleExtension<GlobalCtx, ReqCtx> {
        SampleExtension {
            _gctx: PhantomData,
            _rctx: PhantomData,
        }
    }
}

impl<GlobalCtx, ReqCtx> Extension<GlobalCtx, ReqCtx> for SampleExtension<GlobalCtx, ReqCtx>
where
    GlobalCtx: 'static + Clone + Sync + Send + Debug,
    ReqCtx: 'static
        + Clone
        + Sync
        + Send
        + Debug
        + WarpgrapherRequestContext
        + SampleExtensionReqContext,
{
    fn pre_request_hook(
        &self,
        _global_ctx: Option<GlobalCtx>,
        req_ctx: Option<&mut ReqCtx>,
        _headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        let user = UserProfile {
            id: "MockId".to_string(),
            name: "MockUser".to_string(),
            role: "MockRole".to_string(),
        };
        match req_ctx {
            Some(rctx) => {
                rctx.set_user(user);
            }
            None => { /* throw error */ }
        }
        Ok(())
    }
}
```

`main.rs`

```rust
extern crate serde_json;
extern crate warpgrapher;

use std::fmt::Debug;
use std::sync::Arc;
use warpgrapher::{
    Arguments, Error, ExecutionResult, Executor, GraphQLContext, Info, Neo4jEndpoint, Server,
    WarpgrapherConfig, WarpgrapherExtensions, WarpgrapherRequestContext, WarpgrapherResolvers,
};

mod extension;
use extension::{SampleExtension, SampleExtensionReqContext, UserProfile};

#[derive(Clone, Debug, Default)]
pub struct ReqContext {
    pub user: Option<UserProfile>,
}

impl WarpgrapherRequestContext for ReqContext {
    fn new() -> ReqContext {
        ReqContext { user: None }
    }
}

impl SampleExtensionReqContext for ReqContext {
    fn set_user(&mut self, user: UserProfile) -> () {
        self.user = Some(user);
    }
}

// define endpoint
pub fn whoami(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<(), ReqContext>>,
) -> ExecutionResult {
    let req_ctx = &executor.context().req_ctx.as_ref().unwrap();
    let user = req_ctx.user.as_ref().unwrap();
    Ok(user.clone().into())
}

fn main() -> Result<(), Error> {
    // database
    let db = Neo4jEndpoint::from_env("DB_URL")?;

    // config
    let config = WarpgrapherConfig::from_file("./src/config2.yml".to_string())
        .expect("Could not load config file");

    // resolvers
    let mut resolvers = WarpgrapherResolvers::<(), ReqContext>::new();
    resolvers.insert("WhoAmI".to_string(), Box::new(whoami));

    // extensions
    let sample_extension: SampleExtension<(), ReqContext> = SampleExtension::new();
    let mut extensions: WarpgrapherExtensions<(), ReqContext> = vec![];
    extensions.push(Arc::new(sample_extension));

    // build server with global context type
    let mut server: Server<(), ReqContext> = Server::new(config, db)
        .with_bind_port("5001".to_string())
        .with_playground_endpoint("/graphiql".to_owned())
        .with_resolvers(resolvers)
        .with_extensions(extensions)
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


```
query {
    WhoAmI
}
```

```json
{
  "data": {
    "WhoAmI": {
      "id": "MockId",
      "name": "MockUser",
      "role": "MockRole"
    }
  }
}
```
