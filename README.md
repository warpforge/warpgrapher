# Warpgrapher

Warpgrapher makes it painless to create web services with graph-based data
models. Describe the data model for which you want to run a web service.
Wargrapher automatically generates a GraphQL schema from the data model, as well
as a set of resolvers for basic reate, read, update, and delete (CRUD)
operations on that data.

If you need more more sophisticated, custom queries and endpoints, you can
supply your own custom resolvers. Warpgrapher will automatically generate the
GraphQL configuration and invoke your custom resolvers when appropriate.

The project is currently in development. Prior to reaching v1.0.0:

1. Minor versions represent breaking changes.
2. Patch versions represent fixes and features.
3. There are no deprecation warnings between releases.

# Usage

Add this to your `Cargo.toml`:
```toml
[dependencies]
warpgrapher = "0.1.1"
```

# Getting Started

See the [Quickstart]() section of the Warpgrapher Book. 

# Documentation

See the [Warpgrapher Book]() for in-depth usage documentation. 

# Contributing

Note that the steps below are for doing development on the Warpgrapher itself,
to contribute to the project. In order to develop with Warpgrapher, on your own
project, see the Documentation.

## Clone the Warpgrapher Repository

```
git clone https://github.com/warpforge/warpgrapher.git
```

## Build Warpgrapher

To build for use with Graphson2 graph engines:

```bash
cargo build --features graphson2
```

To build for use with Neo4J:

```bash
cargo build --features neo4j
```

## Test

Set env variables.

For Graphson2 graphs:

```bash
export WG_GRAPHSON2_URL=http://localhost/
export WG_GRAPHSON2_LOGIN=username
export WG_GRAPHSON2_PASS=my-db-pass
```

For Neo4J:

```bash
export DB_PASS=my-db-pass
export WG_NEO4J_URL=http://neo4j:${DB_PASS}@127.0.0.1:7474/db/data
```

Run the database.

For Graphson2:

Commands to run the database will vary, depending on the server, e.g. Tinkerpop vs. CosmosDB.

For neo4j:

```bash
docker run --rm -e NEO4J_AUTH="neo4j/${DB_PASS}" -p 7474:7474 -p 7687:7687 neo4j:3.5
```

Run unit tests:

```bash
cargo test --lib
```

Run all tests (unit and integration).

For Graphson2:

```bash
cargo test --features graphson2 -- --test-threads=1
```

For Neo4J:

```bash
cargo test --features neo4j -- --test-threads=1
```

For all databases:

```bash
cargo test --all-features -- --test-threads=1
```

Run specific test:

```bash
cargo test --features DB_FEATURE <TEST_NAME> -- --test-threads=1
```

Run specific module:

```bash
cargo test --features DB_FEATURE server::graphql::tests -- --test-threads=1
```

Print to console when running tests:

```bash
cargo test --features DB_FEATURE -- --nocapture --test-threads=1
```

Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Format code

```bash
cargo fmt
```

## Review against API style guidelines

Review your change against the following Rust language API style guidelines.

https://rust-lang.github.io/api-guidelines/

If reviewing a PR, use the following as a review checklist:

https://rust-lang.github.io/api-guidelines/checklist.html

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

# License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.