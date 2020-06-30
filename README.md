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
warpgrapher = { version = "0.2.0", features = ["cosmos","neo4j"] }
```

# Getting Started

See the [Quickstart](https://warpforge.github.io/warpgrapher/warpgrapher/quickstart.html) section of the Warpgrapher Book. 

# Documentation

See the [Warpgrapher Book](https://warpforge.github.io/warpgrapher/) for in-depth usage documentation. 

# Contributing

Note that the steps below are for doing development on the Warpgrapher itself,
to contribute to the project. In order to develop with Warpgrapher, on your own
project, see the Documentation.

## Clone the Warpgrapher Repository

```
git clone https://github.com/warpforge/warpgrapher.git
```

## Build Warpgrapher

To build for use with Cosmos DB:

```bash
cargo build --features cosmos
```

To build for use with Neo4J:

```bash
cargo build --features neo4j
```

## Test

### Set Environment Variables

For Cosmos DB graphs:

```bash
export WG_COSMOS_HOST=*MY-COSMOS-DB*.gremlin.cosmos.azure.com
export WG_COSMOS_PORT=443
export WG_COSMOS_USER=/dbs/*MY-COSMOS-DB*/colls/*MY-COSMOS-COLLECTION*
export WG_COSMOS_PASS=*MY-COSMOS-KEY*
```

For Neo4J:

```bash
export WG_NEO4J_HOST=127.0.0.1
export WG_NEO4J_PORT=7687
export WG_NEO4J_USER=neo4j
export WG_NEO4J_PASS=*MY-DB-PASS*
```

### Run the Database

For Cosmos DB:

Cosmos DB is an Azure cloud service, so it's already running. Or, if you're using a local Cosmos
emulator, start the service based on its instructions.

For neo4j:

Note that Warpgrapher is only compatible with Neo4J up to version 3.5. (If anyone knows of a Rust
driver that works with Neo4J version 4, please open an issue and point us to it!)

```bash
docker run --rm -e NEO4J_AUTH="${WG_NEO4J_USER}/${WG_NEO4J_PASS}" -p 7474:7474 -p 7687:7687 neo4j:4.1
```

### Run Tests

Run unit tests.

```bash
cargo test --lib
```

Run all tests (unit and integration).

For Cosmos DB:

```bash
cargo test --features cosmos -- --test-threads=1
```

For Neo4J:

```bash
cargo test --features neo4j -- --test-threads=1
```

For all databases:

```bash
cargo test --all-features -- --test-threads=1
```

Enable full logging and stack traces when running tests:

```bash
RUST_BACKTRACE=full RUST_LOG=warpgrapher cargo test --features *DB_FEATURE* -- --nocapture --test-threads=1
```

## Lint Code

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Check Dependencies for Vulnerabilities

```bash
cargo audit
```

## Format code

```bash
cargo fmt
```

## Generate Documentation

```bash
book/build.sh
```

## Check API Docs for Dead Links

Reorganizing types and functions in a crate can leave dead cross-reference links in the 
documentation. Use the cargo-deadlinks subcommand to check for these dead links.

```bash
cargo deadlinks
```

## Review Against API Style GUide

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


