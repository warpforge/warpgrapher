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
warpgrapher = { version = "0.2.0", features = ["cosmos"] }
```

Then set up environment variables to contact your Cosmos DB:

```bash
export WG_COSMOS_HOST=*MY-COSMOS-DB*.gremlin.cosmos.azure.com
export WG_COSMOS_PORT=443
export WG_COSMOS_LOGIN=/dbs/*MY-COSMOS-DB*/colls/*MY-COSMOS-COLLECTION*
export WG_COSMOS_PASS=*MY-COSMOS-KEY*
```

## Neo4J

Add Warpgrapher to your project config:

```toml
[dependencies]
warpgrapher = { version = "0.2.0", features = ["neo4j"] }
```

Then set up environment variables to contact your Neo4J DB:

```bash
export WG_NEO4J_USERNAME=neo4j
export WG_NEO4J_PASSWORD=password123
export WG_NEO4J_URL=http://neo4j:${WG_NEO4J_PASSWORD}@127.0.0.1:7474/db/data
```

If you do not already have a Neo4J database running, you can run one using Docker:

```bash
docker run -e NEO4JAUTH="${WG_NEO4J_USERNAME}:${WG_NEO4J_PASSWORD}" neo4j:3.5
```