# Warpgrapher
====================================
[![Crates.io](https://img.shields.io/crates/v/diesel.svg)](https://crates.io/crates/diesel)

Warpgrapher is a framework for creating GraphQL API services (backed by graph databases) from a domain data model. 

As a developer you can focus on defining your applications data model and warpgrapher takes care of generating a graph-based API for interacting with that model. In addition to generating CRUD APIs for interacting with the model, warpgrapher provides a set of advanced features to customize and extend your service. 

## Active Development

The project is currently in active development. Prior to reaching 1.0.0:

1. Minor versions represent breaking changes.
2. Patch versions represent fixes and features.
3. There are no deprecation warnings between releases.

## Quickstart

```toml
[dependencies]
warpgrapher = { version = "0.3.0", features = ["cosmos","neo4j"] }
```

```rust
use serde_json::json;
use std::convert::TryFrom;
use std::collections::HashMap;
use warpgrapher::engine::config::Configuration;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::juniper::http::GraphQLRequest;
use warpgrapher::Engine;

static CONFIG: &'static str = "
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
";

#[tokio::main]
async fn main() {
    // parse warpgrapher config
    let config = Configuration::try_from(CONFIG.to_string())
        .expect("Failed to parse CONFIG");

    // define database endpoint
    let db = Neo4jEndpoint::from_env()
        .expect("Failed to parse neo4j endpoint from environment")
        .pool()
        .await
        .expect("Failed to create neo4j database pool");

    // create warpgrapher engine
    let engine: Engine<(), ()> = Engine::new(config, db)
        .build()
        .expect("Failed to build engine");

    // execute graphql mutation to create new user
    let request = GraphQLRequest::new(
        "mutation UserCreate($input: UserCreateMutationInput) {
            UserCreate(input: $input) {
                id
                email
            }
        }
        ".to_string(),
        None,
        None
    );
    let metadata = HashMap::new();
    let result = engine.execute(&request, &metadata).unwrap();

    // display result
    println!("result: {:#?}", result);
}
```

## Documentation

See the [Warpgrapher Book](https://warpforge.github.io/warpgrapher/) for in-depth usage documentation. 

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See the [Contribution Guide](https://warpforge.github.io/warpgrapher/contribution). 

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


