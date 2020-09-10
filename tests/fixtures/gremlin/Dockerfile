FROM tinkerpop/gremlin-server:3.4

COPY tinkergraph-wg.properties conf/tinkergraph-wg.properties
COPY gremlin-server-wg.yaml conf/gremlin-server-wg.yaml
COPY server.jks conf/server.jks

CMD ["conf/gremlin-server-wg.yaml"]
