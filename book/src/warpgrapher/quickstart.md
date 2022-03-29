# Quickstart

This guide will walk through creating a brand new project using the Warpgrapher engine. The quickstart example will create a very simple service. It will store email addresses for users. Warpgrapher is far more capable, allowing storage and retrieval of complex data models with many relationships. But for now, to get started quickly, begin with as simple a data model as possible.

This quickstart assumes a working knowledge of Rust, GraphQL, and at least one graph database. For example, we don't cover creating a new Rust project using `cargo init`.

## Configuration

First, set up the `Cargo.toml` file to import Warpgrapher as a dependency. There are crate features for each of the databases supported as a back-end.  Use the `gremlin` feature to support Gremlin-based databases such as Apache Tinkerpop and Azure CosmosDB. Use `cypher` to support Cypher-based databases, such as AWS Neptune and Neo4J. This tutorial example uses Neo4J.

`Cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.10.3", features = ["cypher"] }
```

The `src/main.rs` file begins with a definition of the data model for the example:

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:9:17}}
```

Configurations are written in YAML. Although this example uses a static string for convenience, configurations may be stored in standalone files, or assembled from multiple parts.

The example configuration illustrates several principles in configuring a Warpgrapher engine. The configuration format itself is versioned, for backward compatibility. The `version: 1` line notes that this configuration uses version 1 of the configuration file format.  Until Warpgrapher reaches version 1.0, breaking changes in the config file format are permitted. After 1.0, breaking changes will trigger an increment to the configuration version.

The configuration contains a `model` object. The model is a list of types present in the data model. In this case, the data model has only a single type called `User`. Type definitions contain one or more properties on the type, listed under `props`. In this example, the `props` list contains only one property, named `email`. The `email` property is of type `String`.

Altogether, this configuration defines a very simple data model. That data model keeps records about users, and the one property tracked for users is their email address.

## Source Code

Once the configuration describing the data model is in place, it takes relatively little code to get a Warpgrapher engine up and running, ready to handle all the basic CRUD operations for that data.

The example creates a request context for the engine. The request context does two things. First, it tells the engine which type of database endpoint to use, which is Neo4J in this case. Second, the context provides a way for systems built on Warpgrapher to pass application-specific data into the engine for later use by custom-written endpoints and resolvers. In this example, there's no such custom data, so the context is empty other than designating a `DBEndpointType` of `CypherEndpoint`.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:20:27}}
```

The Warpgrapher engine is asynchronous, so the main function is set up to be executed by Tokio in this example.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:29:30}}
```

Warpgrapher is invoked to parse the configuration string created in `CONFIG` above.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:31:32}}
```

Next, the databse endpoint is configured using a set of environment variables. See below for the correct environment variables and values.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:34:39}}
```

The configuration and database created above are passed to the Warpgrapher engine, as follows.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:41:44}}
```

At this point, the Warpgrapher engine is created and ready to field queries. The remainder of the source code in the example file created a simple query to demonstrate that the query engine is functioning.  It creates a sample GraphQL query, submits the query to the Warpgrapher engine, and then prints out the query results to stdout. In a realistic system, the Warpgrapher engine would be invoked from the handler function of an HTTP server.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:46:62}}
```

## Database

Configure database settings using the following environment variables:

```bash
export WG_CYPHER_HOST=127.0.0.1
export WG_CYPHER_PORT=7687
export WG_CYPHER_USER=neo4j
export WG_CYPHER_PASS=*MY-DB-PASSWORD*
```

Start a 4.1 Neo4j database:

```bash
docker run --rm -p 7687:7687 -e NEO4J_AUTH="${WG_CYPHER_USER}/${WG_CYPHER_PASS}" neo4j:4.4
```

## Run

Run the example using `cargo` as follows.

```bash
cargo run
```

The output from the example should look something like the following.

```
result: Object({
    "data": Object({
        "UserCreate": Object({
            "id": String(
                "7e1e3497-dcfd-4579-b690-86b110c8f96a",
            ),
            "email": String(
                "a@b.com",
            ),
        }),
    }),
})
```

The identifier will be a different UUID than the one shown above, of course.

## Full Example Code

The full example source code listing is below:

`src/main.rs`

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs}}
```
