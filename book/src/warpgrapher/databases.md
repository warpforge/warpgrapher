# Databases

Warppgrapher supports using either of two database back-ends for graph data:

1. Azure Cosmos DB
2. Gremlin-based Databases (e.g. Apache Tinkerpop and AWS Neptune)
3. Neo4J

Using each of the databases requires correctly selecting a crate feature and 
setting up appropriate environment variables to allow Warpgrapher to connect 
with the database.

## Azure Cosmos DB

Add Warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.8.0", features = ["cosmos"] }
```

Then set up environment variables to contact your Cosmos DB:

```bash
export WG_COSMOS_HOST=*MY-COSMOS-DB*.gremlin.cosmos.azure.com
export WG_COSMOS_PORT=443
export WG_COSMOS_USER=/dbs/*MY-COSMOS-DB*/colls/*MY-COSMOS-COLLECTION*
export WG_COSMOS_PASS=*MY-COSMOS-KEY*
```

Note that when setting up your Cosmos database, you must configure it to offer a Gremlin graph API.

Note also that you must set your partition key to be named `partitionKey`.

Be advised that Gremlin traversals are not executed atomically within Cosmos DB. A traversal may 
fail part way through if, for example, one reaches the read unit capacity limit.  See 
[this article](https://medium.com/@jayanta.mondal/cosmos-db-graph-gremlin-api-how-to-executing-multiple-writes-as-a-unit-via-a-single-gremlin-2ce82d8bf365) 
for details. The workaround proposed in the article helps, but even idempotent queries do not 
guarantee atomicity.  Warpgrapher does not use idempotent queries with automated retries to overcome
this shortcoming of Cosmos DB, so note that if using Cosmos, there is a risk that a failed query 
could leave partially applied results behind.

## Gremlin-Based Database

Add Warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.8.0", features = ["gremlin"] }
```

Then set up environment variables to contact your Gremlin-based DB:

```bash
export WG_GREMLIN_HOST=localhost
export WG_GREMLIN_PORT=8182
export WG_GREMLIN_USER=stephen
export WG_GREMLIN_PASS=password
export WG_GREMLIN_CERT=true
export WG_GREMLIN_UUID=true
```

The `WG_GREMLIN_CERT` environment variable is true if Warpgrapher should ignore the validity of 
certificates. This may be necessary in a development or test environment, but should always be set
to false in production.

The `WG_GREMLIN_UUID` environment variable is set to true if Wargrapher is connecting to a back-end,
like Apache Tinkerpop, that uses a UUID type for the identifier of a node or vertex. If the back-end
uses a `String` type that contains a string representation of an identifier, such as Cosmos DB, then
set this evironment variable to `false`.

If you do not already have a Gremlin-based database running, you can run one using Docker:

```bash
docker build -t gremlin -f tests/fixtures/gremlin/Dockerfile tests/fixtures/gremlin
docker run --rm -p 8182:8182 gremlin:latest
```

To use an interactive gremlin console to manually inspect test instances, run

```bash
docker build -t gremlin-console -f tests/fixtures/gremlin-console/Dockerfile tests/fixtures/gremlin-console
docker run -i --net=host --rm gremlin-console:latest
```

In the console, connect to the remote graph:

```
:remote connect tinkerpop.server conf/remote.yaml
:remote console
```

## Neo4J

Add Warpgrapher to your project config:

```toml
[dependencies]
warpgrapher = { version = "0.8.0", features = ["neo4j"] }
```

Then set up environment variables to contact your Neo4J DB:

```bash
export WG_NEO4J_HOST=127.0.0.1
export WG_NEO4J_PORT=7687
export WG_NEO4J_USER=neo4j
export WG_NEO4J_PASS=*MY-DB-PASSWORD*
```

If you do not already have a Neo4J database running, you can run one using Docker:

```bash
docker run -e NEO4JAUTH="${WG_NEO4J_USER}:${WG_NEO4J_PASS}" neo4j:4.1
```