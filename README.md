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

# Getting Started

See the [Quickstart]() section of the Warpgrapher Book. 

# Documentation

See the [Warpgrapher Book]() for in-depth usage documentation. 

## Contributing

Note that the steps below are for doing development on the Warpgrapher itself,
to contribute to the project. In order to develop with Warpgrapher, on your own
project, see the Documentation.

### Clone the Warpgrapher Repository

```
git clone https://github.com/warpforge/warpgrapher.git
```

### Build Warpgrapher

```bash
cargo build
```

### Test

Set env variables:

```bash
export DB_PASS=my-db-pass
export DB_URL=http://neo4j:${DB_PASS}@127.0.0.1:7474/db/data
```

Run neo4j database:

```bash
docker run --rm -e NEO4J_AUTH="neo4j/${DB_PASS}" -p 7474:7474 -p 7687:7687 neo4j:3.5
```

Run unit tests:

```bash
cargo test --lib
```

Run all tests (unit and integration):

```bash
cargo test
```

Note that integration tests must be run sequentially in a single thread to avoid conflicting with one another.

Run specific test:

```bash
cargo test <TEST_NAME>
```

Run specific module:

```bash
cargo test server::graphql::tests
```

Print to console when running tests:

```bash
cargo test -- --nocapture
```

Test coverage:

```bash
cargo tarpaulin -o Html
```

Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Format code

```bash
cargo fmt
```
