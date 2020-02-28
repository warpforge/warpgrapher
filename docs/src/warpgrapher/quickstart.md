# Quickstart

## Dependencies

Add warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = "0.1.0"
```

## Config

Create a warpgrapher config containing your application's data model. The next section explains the model in more details. 

`src/config.yml`

```yml
version: 1
model:

  # User
  - name: User
    props:
      - name: email
        type: String

  # Team
  - name: Team
    props:
      - name: name
        type: String
    rels:
      - name: users
        nodes: [User]
```

## Code 

Add the follow code to your project:

`src/main.rs`

```rust
extern crate warpgrapher;

use warpgrapher::{Error, WarpgrapherConfig, Server, Neo4jEndpoint};

fn main() -> Result<(), Error> {

    // define database
    let db = Neo4jEndpoint::from_env("DB_URL")?;

    // load config
    let config = WarpgrapherConfig::from_file("./src/config.yml".to_string())
        .expect("Could not load config file");

    // build server
    let mut server: Server<(), ()> = Server::new(config, db)
        .with_bind_port("5001".to_string())
        .with_playground_endpoint("/graphiql".to_owned())
        .build()
        .expect("Failed to build server");

    // run server
    println!("Warpgrapher Sample App: http://localhost:5001/graphiql");
    match server.serve(true) {
        Ok(_) => {},
        Err(_) => println!("Failed to start server"),
    };

    Ok(())
}
```

## Run

Configure database settings:

```bash
export DB_USERNAME=neo4j
export DB_PASSWORD=password123
export DB_URL=http://${DB_USERNAME}:${DB_PASSWORD}@127.0.0.1:7474/db/data
```

Start a 3.5 Neo4j database:

```bash
docker run -e NEO4JAUTH="${DB_USERNAME}:${DB_PASSWORD}" neo4j:3.5
```

Run warpgrapher app: 

```bash
cargo run
```

You should see the following output:
```
Warpgrapher Sample App: https://localhost:5001/graphiql
```

Congrats! Now you have a running warpgrapher app. 

## Explore

Navigate to the displayed URL. You should see the GraphQL Playground which allows you to interact directly with the API. 

#### List all `Users`

First, list all `User` nodes in the database:

```
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

GraphQL and Neo4j are all about relationships. Finally, create a relationship between the `Team` and `User` nodes:

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
    "Team": [
      {
        "id": "e5d5e19a-70bf-4d04-b32f-e61407100914",
        "name": "Blue Team",
        "users": {
          "dst": {
            "email": "user@example.com",
            "id": "0b2a6753-a5cf-46ea-b046-4935ea208950"
          }
        }
      }
    ]
  }
}
```