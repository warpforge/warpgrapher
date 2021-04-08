# Quickstart

This guide will walk you through creating a brand new project using the Warpgrapher engine served over HTTP using actix-web. The back-end graph database in this example is Neo4J. 

## Source

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.8.3", features = ["neo4j"] }
```

`src/main.rs`

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs}}
```

## Database

Configure database settings:

```bash
export WG_NEO4J_HOST=127.0.0.1
export WG_NEO4J_PORT=7687
export WG_NEO4J_USER=neo4j
export WG_NEO4J_PASS=*MY-DB-PASSWORD*
```

Start a 4.1 Neo4j database:

```bash
docker run --rm -p 7687:7687 -e NEO4J_AUTH="${WG_NEO4J_USER}/${WG_NEO4J_PASS}" neo4j:4.1
```

## Run

```bash
cargo run
```
