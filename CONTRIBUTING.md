
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

To build for use with a Gremlin-based database, such as Apache Tinkerpop or AWS Neptune:

```bash
cargo build --features gremlin
```

To build for use with Neo4J:

```bash
cargo build --features neo4j
```

## Test

### Set Environment Variables

```bash
export WG_POOL_SIZE=1
```

Setting the pool size to one during testing helps assure that Warpgrapher will continue to operate 
correctly with constrianed connection pools, even when recursing through resolvers.

For Cosmos DB graphs:

```bash
export WG_COSMOS_HOST=*MY-COSMOS-DB*.gremlin.cosmos.azure.com
export WG_COSMOS_PORT=443
export WG_COSMOS_USER=/dbs/*MY-COSMOS-DB*/colls/*MY-COSMOS-COLLECTION*
export WG_COSMOS_PASS=*MY-COSMOS-KEY*
```

For Gremlin-based DB graphs:

```bash
export WG_GREMLIN_HOST=localhost
export WG_GREMLIN_PORT=8182
export WG_GREMLIN_CERT=true
export WG_GREMLIN_USE_TLS=true
export WG_GREMLIN_LONG_IDS=true
```

The `WG_GREMLIN_CERT` environment variable is true if Warpgrapher should ignore the validity of 
certificates. This may be necessary in a development or test environment, but should always be set
to false in production.

The `WG_GREMLIN_LONG_IDS` envrionment variable is true if Warpgrapher should use long integers as
vertex and edge identifiers. If false, Warpgrapher will send identifers to the database as strings.
In all cases, the client-facing GraphQL schema uses identifiers of the type ID, which GraphQL 
serializes as strings.

For Neo4J:

```bash
export WG_NEO4J_HOST=127.0.0.1
export WG_NEO4J_READ_REPLICAS=127.0.0.1
export WG_NEO4J_PORT=7687
export WG_NEO4J_USER=neo4j
export WG_NEO4J_PASS=*MY-DB-PASS*
```

### Run the Database

For Cosmos DB:

Cosmos DB is an Azure cloud service, so it's already running. Or, if you're using a local Cosmos
emulator, start the service based on its instructions. Note that when setting up the Cosmos 
database, you must 1) choose Gremlin (graph) API support, and 2) use the string `partitionKey` as 
the name of the partition key for the database.

For Gremlin-based databases:

Start your database in accordance with it's instructions.  For example, for the Apache Tinkerpop 
reference implementation, run:

```bash
docker run -it --rm -p 8182:8182 tinkerpop/gremlin-server:latest
```

For neo4j:

Note that Warpgrapher is only compatible with version 4.0 and later of Neo4J.

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
cargo test --features cosmos --tests -- --test-threads=1
```

For Gremlin-based DBs:

```bash
cargo test --features gremlin --tests -- --test-threads=1
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

```
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
cd book; ./build.sh
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


## Updating version prior to release

```bash
export OLD_VERSION=0.8.2
export NEW_VERSION=0.8.3
```

```bash
sed -i "s/${OLD_VERSION}/${NEW_VERSION}/g" ./src/lib.rs
sed -i "s/${OLD_VERSION}/${NEW_VERSION}/g" ./book/book.toml
sed -i "s/${OLD_VERSION}/${NEW_VERSION}/g" ./book/src/*/*.md
find . -type f -name "*.md" | xargs -0 sed -i '' -e "s/${OLD_VERSION}/${NEW_VERSION}/g"
```

```bash
cd ./book
./build.sh
```
