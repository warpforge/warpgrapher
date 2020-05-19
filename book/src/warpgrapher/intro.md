# Warpgrapher

Warpgrapher is framework for developing graph-based API services. Describe the 
data model for which you want to run a web service. Wargrapher automatically 
generates a GraphQL schema from the data model, as well as a set of resolvers 
for basic reate, read, update, and delete (CRUD) operations on that data.

If you need more more sophisticated, custom queries and endpoints, you can
supply your own custom resolvers. Warpgrapher will automatically generate the
GraphQL configuration and invoke your custom resolvers when appropriate.

The project is currently in development. Prior to reaching v1.0.0:

1. Minor versions represent breaking changes.
2. Patch versions represent fixes and features.
3. There are no deprecation warnings between releases.