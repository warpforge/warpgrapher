# Databases

Warppgrapher supports using either of two database back-ends for graph data:

1. Azure Cosmos DB
2. Neo4J

Using each of the databases requires correctly selecting a crate feature and 
setting up appropriate environment variables to allow Warpgrapher to connect 
with the database.

## Azure Cosmos DB

Add Warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.3.0", features = ["cosmos"] }
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

## Neo4J

Add Warpgrapher to your project config:

```toml
[dependencies]
warpgrapher = { version = "0.2.0", features = ["neo4j"] }
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