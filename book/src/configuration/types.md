# Types

The [Quickstart](../warpgrapher/quickstart.html) presented a first example of a Warpgrapher configuration, shown again here.

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:9:16}}
```

## Type Configuration

Recall that the `version` value is used to indicate the configuration file format version to be used. Right now, the only valid value is 1.  The next element in the configuration a a data model.  The `model` object is a list of types. The example shown in the [Quickstart](../warpgrapher/quickstart.html) uses many defaults for simplicity. The definition below shows the full range of options for property definitions. Don't worry about relationships between types for the moment. Those are covered in the [next section](./relationships.html).

```yaml
model:
  - name: String
    props:
      - name: String
        uses:
          create: Boolean
          query: Boolean
          update: Boolean
          output: Boolean
        type: String  # Boolean | Float | ID | Int | String
        required: Boolean
        list: Boolean
        resolver: String
        validator: String
    endpoints:
      read: Boolean
      create: Boolean
      update: Boolean
      delete: Boolean
```

Right under the model object is a list of types. The first attribute describing a type is a name. In the example from the [Quickstart](../warpgrapher/quickstart.html), the name of the type is `User`.

The second attribute describing a type is `props`. The `props` attribute is a list of properties that are stored on nodes of that type. Each property is described the several configuration attributes, as follows.

The `name` attribute is a string that identifies the property. It must be unique within the scope of the type. In the [Quickstart](../warpgrapher/quickstart.html) example, the sole property on the User type is named email.

The `uses` attribute is an object that contains four fields within it, `create`, `query`, `update`, and `output`, each a boolean value. The fields within the `uses` attribute control whether the property is present in various parts of the GraphQL schema. If the `create` attribute is true, then the property will be included in the GraphQL input for creation operations. If false, the property will be omitted from creation operations. If the `query` attribute is true, the property will be included in the GraphQL schema for search query input. If false, the property will be omitted from search query operations. If the `update` attribute is true, the property will be included in the GraphQL schema input for updating existing nodes. If false, the property will be omitted from the update schema. Lastly, if the `output` attribute is true, the property will be included in the GraphQL schema for nodes returned to the client. If false, the property will be omitted from the output.

By default, all `uses` boolean attributes are true, meaning that the property is included in all relevant areas of the GraphQL schema. Selectively setting some of the `uses` attributes handles uses cases where a property should not be available for some operations. For example, one might set the `create` attribute to false if a property is a calculated value that should never be set directly.  One might set `update` to false to make an attribute immutable -- for example, the `email` property of the `User` type might have `update` set to false if GraphQL clients should not be able to tamper with the identities of users.  One might set `output` to false for properties that should never be read through the GraphQL interface, such as for keeping people from reading out a password property.

The `type` attribute of the property definition is a String value that must take on a value of `Boolean`, `Float`, `ID`, `Int`, or `String`, defining type of the property.

If the `required` attribute of the property definition is false, the property is not required (it is optional). By default this attribute is true, which means it must be provided when nodes of this type are created (unless hidden from the `create` use) and it must be present (non-null) when retrieving the node from Warpgrapher (again, unless hidden from the `output` use).

If the `list` attribute of the property definition is true, the property is a list of scalar values of `type`. If `list` is false, the property is only a single value of that scalar type.

The `resolver` attribute is a text key that is used to identify a custom-written resolver function. Warpgrapher allows applications to define custom resolvers that do more or different things than the default CRUD operations automatically provided by Warpgrapher itself.  For example, a custom resolver might dynamically calculate a value, such as a total or average, rather than just returning a value from the database.  Custom resolvers for [dynamic properties](../engine/dynamic_props.html) are covered in greater detail later in the book.

The `validator` attribute is a text key that is used to identify a fuction that validates an input. For example, a validation function might check an email against and email validation regex. [Validation functions](../engine/validators.html) are covered in greater detail later in the book.

Note that the `endpoints` attribute is on the `type` definition, not the `property` definition, as indicated by the indentation in the YAML example above. The `endpoints` attribute is somewhat similar to the `uses` boolean, but at the level of the whole type rather than a single property. If the `read` attribute is true, Warpgrapher will generate a query in the GraphQL schema so that node of this type can be retrieved. If false, no query will be generated. If the `create` attribute is true, Warpgrapher will generate a node creation mutation in the GraphQL schema. If false, no creation mutation will be generated. If the `update` attribute is true, Warpgrapher will generate a node update mutation in the GraphQL schema. If false, no update mutation will be generated. Lastly, if the `delete` attribute is true, Warpgrapher will generate a node deletion mutation in the GraphQL schema. If false, no delete mutation will be generated.

## Generated Schema

Warpgrapher uses the configuration described above to automatically generate a GraphQL schema and default resolver to create, read, update, and delete nodes of the types defined in the configuration's model section.  The remainder of this section walks through the contents of the schema in detail.

The top level GraphQL Query has two queries within it, as shown below. The `_version` query returns a scalar `String` with the version of the GraphQL service. The value returned is set with the [with_version](https://docs.rs/warpgrapher/latest/warpgrapher/engine/struct.EngineBuilder.html#method.with_version) method on the `EngineBuilder`.

```
type Query {
  User(input: UserQueryInput, partitionKey: String): [User!]
  _version: String
}
```

The `User` query, above, is generated by Warpgrapher for the retrieval of User nodes. The query takes two parameters, an `input` parameter that provides any search parameters that narrow down the set of Users to be retrieved, and a `partitionKey`.  The `partitionKey` is described in more detail in the [Databases](./databases.md) section of the book.  The query returns a `User` type.

The `UserQueryInput`, defined in the schema snippet below, is use to provide search parameters to identify the `User` nodes to return to the client. The `User` node configuration had only one property, `email`. Warpgrapher automatically adds an `id` property that contains a unique identifier for nodes. In the GraphQL schema, the id is always represented as a string. However, in some Gremlin back-ends, the id may be required to be an integer, in which case the id field in the GraphQL schema will be a String that can be successfully parsed into an integer value. 

```
input UserQueryInput {
  email: StringQueryInput
  id: StringQueryInput
}
```

Note that the types of both `email` and `id` are `StringQueryInput`, not a simple `String` scalar. This is because the query input allows for more than just an exact match.

```
input StringQueryInput {
  CONTAINS: String
  EQ: String
  GT: String
  GTE: String
  IN: [String!]
  LT: String
  LTE: String
  NOTCONTAINS: String
  NOTEQ: String
  NOTIN: [String!]
}
```

The `StringQueryInput` has various options for matching a String more flexibly than an exact match. The `CONTAINS` operator looks for the associated String value anywhere in the target property (e.g. the `email` or `id` properties of a `User` node).  `EQ` looks for an exact match.  `GT` and `GTE` are greater-than and great-than-or-equals, which are useful for searching for ranges based on alphabetization, as do `LT` and `LTE`.  The `IN` operators allows for searching for any string that is within a given set of Strings.  `NOTCONTAINS` is the opposite of `CONTAINS`, looking for property values that do not contain the provided String.  `NOTEQ` looks for non-matching Strings. And finally, `NOTIN` matches property values that do not appear in the provided set of Strings.

```
type User {
  email: String
  id: ID!
}
```

The `User` type is the definition of the output type for the `User` GraphQL query. The names are the same, but these are two distinct things in the GraphQL schema -- the `User` query returns an array of zero or more `User` types.  The `User` type is two fields, and `id` and an `email`.  The id is a unique identifier for that node, which may be an integer or a UUID, depending on the graph database used. The `email` string is the single property that was defined on the example schema.

```
type Mutation {
  UserCreate(input: UserCreateMutationInput!, partitionKey: String): User
  UserDelete(input: UserDeleteInput!, partitionKey: String): Int
  UserUpdate(partitionKey: String, input: UserUpdateInput!): [User!]
}
```

In addition to providing queries to retrieve existing nodes, Warpgrapher also automatically generates GraphQL schema elements and resolvers for create, update, and delete operations. The schema snippet above shows the mutations that are generated for the `User` node in the example configuration.  All three of the mutations take a `partitionKey`, which was described in the section on queries, above. Additionally, all three mutations take an `input` value, that provides the information necessary to complete the create, update, or delete operation, respectively.  Creation operations return the created node. Update operations return all the nodes that were matched and updated.  Lastly, the delete operation returns the number of nodes that were deleted.  The input arguments are detailed below.

```
input UserCreateMutationInput {
  email: String
  id: ID
}
```

The `UserCreateMutationInput` mutation input includes the email property defined in the example configuration. It also includes an `id` property. Note that the `id` property is optional. If not provided by the client, it will be set to a unique identifier by the Warpgrapher server. The reason that clients are permitted to set the `id` when creating nodes is to allow for offline mode support, which may require the creation of identifiers within local caches that should remain the same after synchronization with the server.

```
input UserDeleteInput {
  DELETE: UserDeleteMutationInput
  MATCH: UserQueryInput
}

input UserDeleteMutationInput
```

The `UserDeleteInput` input is used to identify which nodes to delete. Note that the `MATCH` part of the argument is the very same `UserQueryInput` type used in the `User` query schema element above. So searching for which nodes to delete is the same input format used to search for nodes to return in a read query.  The `UserDeleteMutationInput` is empty right now, and may be omitted. It will become relevant later, in the discussion on relationships between nodes.

```
input UserUpdateInput {
  MATCH: UserQueryInput
  SET: UserUpdateMutationInput
}

input UserUpdateMutationInput {
  email: String
}
```

Lastly, the `UserUpdateInput` input is provided to the udpate mutation in order to select the nodes that need to be updated and describe the update to be applied.  The `MATCH` attribute is used to identify what nodes require the update. Note that the type of the `MATCH` attribute is `UserQueryInput`, which is the same type used for searching for nodes in the GraphQL query above.  The `SET` attribute is used to provide the new values to which the matching nodes should be set. In this example, it is a single String value for the `email` of the `User`. Note that `id`s are set only at creation. They cannot be updated later.

## Full Schema Listing

The full schema, described in pieces above, is included below:

```
input UserDeleteInput {
  DELETE: UserDeleteMutationInput
  MATCH: UserQueryInput
}

input UserQueryInput {
  email: StringQueryInput
  id: StringQueryInput
}

type Mutation {
  UserCreate(input: UserCreateMutationInput!, partitionKey: String): User
  UserDelete(input: UserDeleteInput!, partitionKey: String): Int
  UserUpdate(partitionKey: String, input: UserUpdateInput!): [User!]
}

input UserUpdateMutationInput {
  email: String
}

type Subscription

input UserUpdateInput {
  MATCH: UserQueryInput
  SET: UserUpdateMutationInput
}

type Query {
  User(input: UserQueryInput, partitionKey: String): [User!]
  _version: String
}

input UserDeleteMutationInput

type User {
  email: String
  id: ID!
}

input UserCreateMutationInput {
  email: String
  id: ID
}

input StringQueryInput {
  CONTAINS: String
  EQ: String
  GT: String
  GTE: String
  IN: [String!]
  LT: String
  LTE: String
  NOTCONTAINS: String
  NOTEQ: String
  NOTIN: [String!]
}
```
