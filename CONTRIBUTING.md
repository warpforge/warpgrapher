
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

For Gremlin-based DB graphs:

```bash
export WG_GREMLIN_HOST=localhost
export WG_GREMLIN_READ_REPLICAS=localhost
export WG_GREMLIN_PORT=8182
export WG_GREMLIN_USE_TLS=false
export WG_GREMLIN_VALIDATE_CERTS=false
export WG_GREMLIN_BINDINGS=true
export WG_GREMLIN_LONG_IDS=true
export WG_GREMLIN_PARTITION=false
export WG_GREMLIN_SESSIONS=false
```

For Neo4J:

```bash
export WG_NEO4J_HOST=127.0.0.1
export WG_NEO4J_READ_REPLICAS=127.0.0.1
export WG_NEO4J_PORT=7687
export WG_NEO4J_USER=neo4j
export WG_NEO4J_PASS=*MY-DB-PASS*
```

### Run the Database

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

## Review Against API Style Guide

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
