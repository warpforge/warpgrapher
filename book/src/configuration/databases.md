# Databases

Warpgrapher translates GraphQL queries into CRUD operations against a back-end data store, based on a configuration specifying a data model. The tutorial will return to the topic of the [configuration file](./config.html) soon, but the first step is configuring Warpgrapher to integrate with the back-end database. Without a graph database behind it, Warpgrapher's functionality is sharply limited.

Warppgrapher supports several database back-ends for graph data:

1. Apache Tinkerpop
2. AWS Neptune (Cypher variant)
3. Azure Cosmos DB (Gremlin variant)
4. Neo4J

It may be possible to use Warpgrapher with other graph databases. The list above is the set that the maintainers have used previosuly. Using each of the databases above requires selecting the [appropriate crate feature](./intro.html) and setting up environment variables to provide connection information to Warpgrapher, as described below.

Regardless of database, export an environment variable to control the size of the database 
connection pool:

```bash
export WG_POOL_SIZE=8
```

If the `WG_POOL_SIZE` variable is not set, Warpgrapher defaults to a pool the same size as the 
number of CPUs detected. If the number of CPUs cannot be detected, Warpgrapher defaults to a pool
of 8 connections. 

## Gremlin-Based Databases

For all gremlin-based databases, such as Apache Tinkerpop and Azure Cosmos DB the
following environment variables control connection to the database.

- WG_GREMLIN_HOST is the host name for the database to which to connect.
- WG_GREMLIN_READ_REPICA provides a separate host name for read-only replica nodes, if being 
used for additional scalability. If not set, the read pool connects to the same host as the
read/write connection pool.
- WG_GREMLIN_PORT provides the port to which Warpgrapher should connect.
- WG_GREMLIN_USER is the username to use to authenticate to the database, if required.
- WG_GREMLIN_PASS is the password to use to authenticate to the database, if required.
- WG_GREMLIN_USE_TLS is set to `true` if Warpgrapher should connect to the database over a TLS 
connection, and `false` if not using TLS. Defaults to `true`.
- WG_GREMLIN_VALIDATE_CERTS is set to `true` if Warpgrapher should validate the certificate used
for a TLS connection, and `false`. Defaults to `true`. Should only be set to false in non-production
environments.
- WG_GREMLIN_LONG_IDS is set to `true` if Warpgrapher should use long integers for vertex and edge
identifiers. If `false`, Warpgrapher uses strings. Defaults to `false`. Consult your graph database's documentation to determine what values are valid for identifiers.
- WG_GREMLIN_PARTITIONS is set to `true` if Warpgrapher should require a partition ID, and false if 
Warpgrapher should ignore or omit partition IDs. Defaults to `false`.
- WG_GREMLIN_SESSIONS is set to `true` if Warpgrapher mutations should be conducted within a single
Gremlin session, which in some databases provides transactional semantics, and `false` if sessions 
should not be used. Defaults to `false`.
- WG_GREMLIN_VERSION may be set to `1`, `2`, or `3`, to indicate the version of GraphSON 
serialization that should be used in communicating with the database. Defaults to `3`.

Example configurations for supported databases are shown below. In many cases, some environment 
variables are omitted for each database where the defaults are correct.

### Apache Tinkerpop

Add Warpgrapher to your project config with the gremlin feature enabled.

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.10.1", features = ["gremlin"] }
```

Set up environment variables to contact your Gremlin-based DB:

```bash
export WG_GREMLIN_HOST=localhost
export WG_GREMLIN_PORT=8182
export WG_GREMLIN_USER=username
export WG_GREMLIN_PASS=password
export WG_GREMLIN_USE_TLS=true
export WG_GREMLIN_VALIDATE_CERTS=true
export WG_GREMLIN_LONG_IDS=true
```

The `WG_GREMLIN_CERT` environment variable is true if Warpgrapher should ignore the validity of 
certificates. This may be necessary in a development or test environment, but should always be set
to false in production.

If you do not already have a Gremlin-based database running, you can run one using Docker:

```bash
docker run -it --rm -p 8182:8182 tinkerpop/gremlin-server:latest
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

### AWS Neptune

Add Warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.10.1", features = ["cypher"] }
```

Then set up environment variables to contact your Neptune DB:

```bash
export WG_CYPHER_HOST=127.0.0.1
export WG_CYPHER_READ_REPLICAS=127.0.0.1
export WG_CYPHER_PORT=7687
export WG_CYPHER_USER=
export WG_CYPHER_PASS=
```

### Azure Cosmos DB

Add Warpgrapher to your project config:

`cargo.toml`

```toml
[dependencies]
warpgrapher = { version = "0.10.1", features = ["gremlin"] }
```

Then set up environment variables to contact your Cosmos DB:

```bash
export WG_GREMLIN_HOST=*MY-COSMOS-DB*.gremlin.cosmos.azure.com
export WG_GREMLIN_PORT=443
export WG_GREMLIN_USER=/dbs/*MY-COSMOS-DB*/colls/*MY-COSMOS-COLLECTION*
export WG_GREMLIN_PASS=*MY-COSMOS-KEY*
export WG_GREMLIN_USE_TLS=true
export WG_GREMLIN_VALIDATE_CERTS=true
export WG_GREMLIN_PARTITIONS=true
export WG_GREMLIN_VERSION=1
```

Note that when setting up your Cosmos database, you must configure it to offer a Gremlin graph API.

Note also that you must set your partition key to be named `partitionKey`, as this name for the partition key is hard-coded into Warpgrapher.  (This could be changed. If that would be helpful to you, [file an issue](https://github.com/warpforge/warpgrapher/issues) with a feature request to make the partition key name configurable.

Be advised that Gremlin traversals are not executed atomically within Cosmos DB. A traversal may 
fail part way through if, for example, one reaches the read unit capacity limit.  See 
[this article](https://medium.com/@jayanta.mondal/cosmos-db-graph-gremlin-api-how-to-executing-multiple-writes-as-a-unit-via-a-single-gremlin-2ce82d8bf365) 
for details. The workaround proposed in the article helps, but even idempotent queries do not 
guarantee atomicity.  Warpgrapher does not use idempotent queries with automated retries to overcome
this shortcoming of Cosmos DB, so note that if using Cosmos, there is a risk that a failed query 
could leave partially applied results behind.

## Neo4J

Add Warpgrapher to your project config.

```toml
[dependencies]
warpgrapher = { version = "0.10.1", features = ["cypher"] }
```

Then set up environment variables to contact your Neo4J DB.

```bash
export WG_CYPHER_HOST=127.0.0.1
export WG_CYPHER_READ_REPLICAS=127.0.0.1
export WG_CYPHER_PORT=7687
export WG_CYPHER_USER=neo4j
export WG_CYPHER_PASS=*MY-DB-PASSWORD*
```

Note that the `WG_CYPHER_READ_REPLICAS` variable is optional. It is used for Neo4J cluster 
configurations in which there are both read/write nodes and read-only replicas. If the 
`WG_CYPHER_READ_REPLICAS` variable is set, read-only queries will be directed to the read replicas,
whereas mutations will be sent to the instance(s) at `WG_CYPHER_HOST`.

If you do not already have a Neo4J database running, you can run one using Docker:

```bash
docker run -e NEO4J_AUTH="${WG_CYPHER_USER}/${WG_CYPHER_PASS}" neo4j:4.4
```