# Quickstart

This guide will walk you through creating a brand new project using the Warpgrapher engine served over HTTP using actix-web. The back-end graph database in this example is Neo4J. 

## Dependencies

Add warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
actix = "0.9.0"
actix-web = "2.0.0"
warpgrapher = { version = "0.2.0", features = ["neo4j"] }
```

## Config

Create a warpgrapher config containing your application's data model. The next section explains the model in more details. 

`src/config.yml`

```yml
version: 1
model:

  # User
  - name: User
    properties:
      - name: email
        type: String

  # Team
  - name: Team
    properties:
      - name: name
        type: String
    relationships:
      - name: users
        nodes: [User]
```

## Code 

Add the follow code to your project:

`src/main.rs`

```rust
use actix::System;
use actix_http::error::Error;
use actix_web::web::{Data, Json};
use actix_web::{web, App, HttpResponse, HttpServer};
use std::collections::HashMap;
use warpgrapher::engine::database::neo4j::Neo4jEndpoint;
use warpgrapher::engine::database::DatabaseEndpoint;
use warpgrapher::{playground_source, Config, Engine, GraphQLRequest};

#[derive(Clone)]
struct ActixServerAppData {
    engine: Engine<(), ()>,
}

async fn graphql(
    data: Data<ActixServerAppData>,
    req: Json<GraphQLRequest>,
) -> Result<HttpResponse, Error> {
    let metadata: HashMap<String, String> = HashMap::new();
    let resp = &data.engine.execute(req.into_inner(), metadata);
    match resp {
        Ok(body) => Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body.to_string())),
        Err(e) => Ok(HttpResponse::InternalServerError()
            .content_type("application/json")
            .body(e.to_string())),
    }
}

#[allow(clippy::ptr_arg)]
pub fn run_actix_server(engine: Engine<(), ()>) {
    let sys = System::new("warpgrapher-quickstart");
    let app_data = ActixServerAppData { engine };
    HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .route("/graphql", web::post().to(graphql))
            .route(
                "/graphiql",
                web::get().to(|| {
                    HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(playground_source(&"/graphql"))
                }),
            )
    })
    .bind("127.0.0.1:5000")
    .expect("Failed to start server")
    .run();
    let _ = sys.run();
}

fn main() -> Result<(), Error> {
    // load warpgrapher config
    let config =
        Config::from_file("./src/config.yml".to_string()).expect("Failed to load config file");

    // define database endpoint
    let db = Neo4jEndpoint::from_env().unwrap();

    // create warpgrapher engine
    let engine: Engine<(), ()> = Engine::new(config, db.pool().expect("Failed to build db pool"))
        .build()
        .expect("Failed to build engine");

    // serve the warpgrapher engine on actix webserver
    println!("Warpgrapher quickstart app: http://localhost:5000/graphiql");
    run_actix_server(engine);

    Ok(())
}
```

## Run

Configure database settings:

```bash
export WG_NEO4J_USERNAME=neo4j
export WG_NEO4J_PASSWORD=password123
export WG_NEO4J_URL=http://neo4j:${WG_NEO4J_PASSWORD}@127.0.0.1:7474/db/data
```

Start a 3.5 Neo4j database:

```bash
docker run -e NEO4JAUTH="${WG_NEO4J_USERNAME}:${WG_NEO4J_PASSWORD}" neo4j:3.5
```

Run quickstart app: 

```bash
cargo run
```

You should see the following output:
```
Warpgrapher Quickstart App: https://localhost:5000/graphiql
```

Congrats! Now you have a running warpgrapher app. 

## Explore

Navigate to the displayed URL. You should see the GraphQL Playground which allows you to interact directly with the API. 

#### List all `Users`

First, list all `User` nodes in the database:

```graphql
query {
  User {
    id
    email
  }
}
```

You should expect to see an empty list since the database is empty:

```json
{
  "data": {
    "User": []
  }
}
```

#### Create a new `User`

Next, create a new user:

```
mutation {
  UserCreate(input: {
    email: "user@example.com"
  }) {
    id
    email
  }
}
```

The response should display the newly created user:

```json
{
  "data": {
    "UserCreate": {
      "email": "user@example.com",
      "id": "0b2a6753-a5cf-46ea-b046-4935ea208950"
    }
  }
}
```

(Your `id` will of course differ). 

#### Create a new `Team`

Now, create a `Team` node:

```
mutation {
  TeamCreate(input: {
    name: "Blue Team"
  }) {
    id
    name
  }
}
```

Like before, you should see the newly created `Team` node:

```json
{
  "data": {
    "TeamCreate": {
      "id": "d381a0f7-8a01-49e3-80ff-15ba01f3604f",
      "name": "Blue Team"
    }
  }
}
```

#### Add `User` to `Team`

GraphQL and Neo4j are all about relationships. Create a `users` relationship between the `Team` and `User` nodes:

```
mutation {
  TeamUsersCreate(input: {
    match: {
      name: "Blue Team"
    },
    create: {
      dst: {
        User: {
          EXISTING: {
            email: "user@example.com"
          }
        }
      }
    }
  }) {
    id
  }
}
```

```json
{
  "data": {
    "TeamUsersCreate": {
      "id": "e5d5e19a-70bf-4d04-b32f-e61407100914",
    }
  }
}
```

#### Execute nested query

Finally, query the `Team` and all related `User` nodes under the `users` relationship:

```
query {
  Team {
    id
    name
    users {
      dst {
        ... on User {
          id
          email
        }
      }
    }
  }
}
```

```json
{
  "data": {
    "Team": [
      {
        "id": "cbd63d09-13b9-4199-b926-94716b2a547c",
        "name": "Blue Team",
        "users": {
          "dst": {
            "email": "user@example.com",
            "id": "78c71745-6362-49b2-8b6d-3e19de3f4efc"
          }
        }
      }
    ]
  }
}
```