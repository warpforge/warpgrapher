# Relationships

The [Quickstart](../warpgrapher/quickstart.html) example used a very simple model with only one type, containing one property. The [Types](./types.html) section explored configuration file format and the resulting GraphQL schema in more detail. However, Warpgrapher can generate create, read, update, and delete operations for relationships between types as well. The configuration below includes describes two types and a relationship between them.

```yaml
version: 1
model:
  - name: User
    props:
      - name: email
        type: String
        required: false
  - name: Organization
    props:
      - name: name
        type: String
        required: false
    rels:
      - name: members
        nodes: [User]
        list: true
        props:
          - name: joinDate
            type: String
            required: false
```

The configuration above adds a second type, called `Organization`. The definition of the organization type contains the `rels` attribute, which was not seen in the earlier example. The `rels` attribute contains a list of permissible relationships between nodes. In this case, the configuration adds a `members` relationship from nodes of of the `Organization` type to nodes of the `User` type, indicating that certain users are members of an organization. The `name` attribute in the configuration is the name of the relationship and must be unique within the scope of that type. The `nodes` attribute is a list of other types that may be at the destination end of the relationship. In this case, the only type at may be a member is the `User` type, but in other use cases, the destination node might be allowed to be one of several types.  Lastly, the `list` attribute is `true`, indicating that an `Organization` may have more than one member.

## Relationship Configuration

The example configuration above is fairly simple, and does not make use of several optional attributes. The definition below shows the full set of configuration options that are permissible on a relationship.

```yaml
model:
  - name: String
    rels:
      - name: String
        nodes: [String]  # Values in the list must be other types in the model
        list: Boolean
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
        resolver: String
```

The snippet above shows that relationships are defined in a list under the `rels` attribute within a type definition. Each relationship has a `name` that must be unique within the scope of that type. The `nodes` attribute is a list of name of types within the model that can appear as destination nodes in the relationship. Note that the a type may appear in its own relationship's `nodes` lists. A node is permitted to have relationships to nodes of the same type.

If the `list` attribute is `true`, then a node may have relationships of the same type to multiple destination nodes, modeling one-to-many relationships. If `list` is false, then the node may only have a single relationship of that type, to a single destination node.

The `props` attribute on a relationship works the same way that the `props` attribute works on nodes, except that the properties are associated with the relationship rather than with the node.  See the description of the `props` attribute in the section on [types](./types.html) for more details.

Similarly, the `endpoints` attribute on relationships works the same way that it does on nodes. The individual boolean attributes within the `endpoints` object control whether Warpgrapher generates GraphQL schema elements for create, read, update, and delete operations. Just as with types, the default for all the boolean values is `true`, meaning that by default Warpgrapher creates schema elements and resolvers for all CRUD operations.

Lastly, the `resolver` attribute is also similar to the attribute of the same name on property definitions. The string in the `resolver` attribute is mapped to a custom-written Rust function provided when setting up the Warpgrapher engine. This allows systems using Warpgrapher to control the behavior of resolving some relationships. Use cases for this include dynamically-generated relationships that are computed at query time rather than being stored in the back-end data store.

## Generated Schema

This section describes each of the GraphQL schema elements that Warpgrapher generates for CRUD operations on relationships. Discussion of the schema elements related solely to types, absent relationships, was covered previously in the [types section](./types.html).

### Queries in a Model with Relationships

The top level GraphQL query object includes three (3) queries. This should make intuitive sense. The model has two nodes, `Organization` and `User`, and one relationship, the `OrganizationMembers` relationship from a source organization to a destination user that is a member of that organization. Warpgrapher's generated schema allows for querying either node type or the relationship between them. As will be discussed in detail below, the inputs to these query operations have a recursive structure, so that using the top level query for the relationship, it is possible to filter based on the properties of the source or destination nodes. Similarly, when querying for a node type, it is possible to add search parameters related to relationships, the destinations of those relationships, and so on.

```
type Query {
  Organization(
    partitionKey: String
    input: OrganizationQueryInput
  ): [Organization!]
  OrganizationMembers(
    partitionKey: String
    input: OrganizationMembersQueryInput
  ): [OrganizationMembersRel!]
  User(partitionKey: String, input: UserQueryInput): [User!]
  _version: String
}
```

### Querying for a Relationship

In the top level GraphQL query, note that a new query, called `OrganizationMembers` has been generated for the relationship. This query has an input parameter, `OrganizationMembersQueryInput` that provides search query arguments to select the set of relationships to be returned.

The `OrganizationMembersQueryInput` query parameter, defined below, provides a means to search for a given instance of a relationship.  It is possible to search based on an `id` or set of IDs, and the `joinDate` attribute allows queries based on the properties on the relationship. In addition to using the `id` or another property on the relationship, the `OrganizationMembersQueryInput` parameter also includes a `src` and a `dst` attribute. These attributes allow Warpgrapher clients to search for relationships based on properties of the source or destination nodes joined by the relationship.

```
input OrganizationMembersQueryInput {
  dst: OrganizationMembersDstQueryInput
  id: StringQueryInput
  joinDate: StringQueryInput
  src: OrganizationMembersSrcQueryInput
}
```

The two input objects for the `src` and `dst` input objects are shown below. Note that for the source query input, the only attribute is an `Organization` attribute that is an `OrganizationQueryInput` and that for the destination, the only attribute is a `User` attribute that is a `UserQueryInput`. There are two important observations here.  First, the reason for having the `OrganizationMembersDstQueryInput` object is that a relationship might have more than one node type as a possible destination. When building the GraphQL schema, Warpgrapher has to allow for the client to query any of those possible destination nodes. In this example, the only type of destination node is a `User`, so that's the only possibility shown below. If the nodes list had more types of nodes, any of those node types could be queried through the `OrganizationMembersDstQueryInput`.  The second observation is that both the `OrganizationQueryInput` and the `UserQueryInput` inputs are the same input parameters used to query for a set of nodes in the `Organization` and `User` root level GraphQL queries shown above.

```
input OrganizationMembersSrcQueryInput {
  Organization: OrganizationQueryInput
}

input OrganizationMembersDstQueryInput {
  User: UserQueryInput
}
```

We'll come back to the node-based query input in a moment, in the section below on Querying for a Node. First, the code snippet below shows the schema for output from the relationship query. The relationship includes four attributes, a unique identifier for the relationship called `id`, `joinDate` for the property configured on the relationship, and `src` and `dst` attributes that represent the source and destination nodes respectively.

```
type OrganizationMembersRel {
  dst: OrganizationMembersNodesUnion!
  id: ID!
  joinDate: String
  src: Organization!
}
```

The `src` attribute in the `OrganizationMembersRel` output type is an `Organization` type, which is exactly the same output type used for node queries, and so will be covered in the section on querying for nodes, below.  The `dst` attribute is a little more complex. Recall from the description of the configuration schema that Warpgrapher may connect from a source node type to a destination that can be one of many node types. A GraphQL union type is used to represent the multiple destination node types that may exist.  As shown in the schema snippet below, in this example of `OrganizationMembersNodesUnion`, there is only a single destination node type of User. A more complex configuration might have multiple node types in the union.

```
union OrganizationMembersNodesUnion = User
```

Note that the `User` type is the same type that is used to return users in queries for nodes.

### Querying for a Node

The root GraphQL `Query` object has queries for each of the node types in the configuration.  To see how relationships affect node queries, have a look at the `Organization` query, beginning with the `OrganizationQueryInput` definition in the snippet below. In addition to the `id` and `name` attributes for searching based on the scalar properties of the type, the schema also includes a `members` attribute, of type `OrganizationMembersQueryInput`.  This is the same input object described above that's used in the root level query for the `OrganizationMembers` relationship. This recursive schema structure is really quite powerful, as it allows the client to query for nodes based on a combination of the node's property values, the values of properties in the relationships that it has, and the values of properties in the destination nodes at the other end of those relationships, to any level of depth.  For example, it would be easy to construct a query that retrieves all of the organizations that contain a particular user as a member. For examples of relationship-based queries, see the chapter on [API usage](../api/intro.html).

```
input OrganizationQueryInput {
  id: StringQueryInput
  members: OrganizationMembersQueryInput
  name: StringQueryInput
}
```

Relationshps information can be navigated in the output type for the node, as well. The `Organization` output type shown in the snippet below includes both the scalar properties on the type, the `id` and `name`, as well as the relationship to the `members` of the Organization.  The `members` attribute includes an input of type `OrganizationMembersQueryInput`. This is the same input type that is used to query for members relationships from the GraphQL root query, desribed above. This means that when retrieving Organization nodes, it's possible to filter the set of members that you want to retrieve in a nested query. Again, the recursive structure of the schema generated by Warpgrapher allows you the flexibility to query to any level of depth in a sub-graph that is needed.

```
type Organization {
  id: ID!
  members(input: OrganizationMembersQueryInput): [OrganizationMembersRel!]
  name: String
}
```

### Mutations in a Model with Relationships

The GraphQL schema's top level mutation object contains nine (9) mutations. This should make intuitive sense. There are three mutations (create, update, and delete), and three kinds of things that can be mutated: organization nodes, user nodes, and membership relationships between organizations and nodes. There are quite a few nested input and output types contributing to these mutations. The high-level principle to keep in mind is that Warpgrapher allows recursive operations that support manipulation of whole sub-graphs at a time. For example, node mutations have nested input objects that allow manipulation of the relationships on those nodes, and the destination nodes at the end of those relationships, and so on.

```
type Mutation {
  OrganizationCreate(
    input: OrganizationCreateMutationInput!
    partitionKey: String
  ): Organization
  OrganizationDelete(partitionKey: String, input: OrganizationDeleteInput!): Int
  OrganizationMembersCreate(
    input: OrganizationMembersCreateInput!
    partitionKey: String
  ): [OrganizationMembersRel!]
  OrganizationMembersDelete(
    input: OrganizationMembersDeleteInput!
    partitionKey: String
  ): Int
  OrganizationMembersUpdate(
    partitionKey: String
    input: OrganizationMembersUpdateInput!
  ): [OrganizationMembersRel!]
  OrganizationUpdate(
    partitionKey: String
    input: OrganizationUpdateInput!
  ): [Organization!]
  UserCreate(input: UserCreateMutationInput!, partitionKey: String): User
  UserDelete(partitionKey: String, input: UserDeleteInput!): Int
  UserUpdate(partitionKey: String, input: UserUpdateInput!): [User!]
}
```

### Mutating a Relationship

#### Creating a Relationship

The snippet below contains the input for creation of one or more OrganizationMembers relationships. There are two attributes, `MATCH` and `CREATE`. The `MATCH` attribute is used to identify the organization or organizations that should be matched as the source of the relationship(s) to be created. It has the same type, `OrganizationQueryInput` that is used to query for nodes using the `Organization` query under the GraphQL `Query` root described above.  The match query may select more than one node, allowing similar relationships to be created in bulk. Matching existing source nodes is the only option when creating a relationship. If it is necessary to create the node at the source end of the relationship, see the node creation operation, in this case `OrganizationCreate` instead.

```
input OrganizationMembersCreateInput {
  CREATE: [OrganizationMembersCreateMutationInput!]
  MATCH: OrganizationQueryInput
}
```

The `CREATE` attribute has a type of `OrganizationMembersCreateMutationInput`. That input structure is shown in the schema snippet below. It includes the `joinDate` attribute on the relationship. The `id` object is accepted as an input to facilitate offline operation, in which the client may need to choose the unique identifier for the relationship. If the client does not choose the identifier, it will be randomly assigned by the Warpgrapher service.

```
input OrganizationMembersCreateMutationInput {
  dst: OrganizationMembersNodesMutationInputUnion!
  id: ID
  joinDate: String
}
```

The `dst` property in the `OrganizationMembersCreateMutationInput` above is of type `OrganizationMembersNodesMutationInputUnion`, which is included in the schema snippet below. Don't be intimidated by the lengthy name of the union type. Recall that in the configuration above, the destination type of a relationship is allowed to have more than one type. In this configuration, it only has one type, but the `OrganizationMembersNodesMutationInputUnion` is what allows the destination of the relationship to have multiple types. In this case, the only option is `User`, with a type of `UserInput`.

```
input OrganizationMembersNodesMutationInputUnion {
  User: UserInput
}
```

The `UserInput` type, which provides the destination node for the relationship(s) to be created, has two attributes. When using the `EXISTING` attribute, Warpgrapher search the graph database for a set of nodes matching the `UserQueryInput` search criteria and uses the results as the destination nodes for creation of the relationship(s). Note that this `UserQueryInput` type is the same input type that is used to query for users in the user query under the GraphQL root `Query`. No matter where in the recursive hierarchy, searhing for `User` nodes always uses the same input.  The `NEW` attribute creates a new `User` node as the destination of the relationship. Note that the `UserCreateMutationInput` input type is the same input type used to create a `User` node in the `UserCreate` mutation under the GraphQL root `Mutation` object.

```
input UserInput {
  EXISTING: UserQueryInput
  NEW: UserCreateMutationInput
}
```

The output of creating one or more relationships, `OrganizationMembersRel`, is the same output type returned from querying for the organization's members relationship, as was described in the section on queries, above. It contains the newly created relationship.

#### Updating a Relationship

The input for a relationship update mutation, `OrganizationMembersUpdateInput` is shown in the schema snippet below. The update input consists of two parts. The `MATCH` attribute is a query input to identify the relationships that should be updated. Note that the match input type, `OrganizationMembersQueryInput` is the same input type used to provide search parameters when searching for relationships under the `OrganizationMembers` query under the GraphQL root `Query` object.  The `SET` attribute is used to describe the changes that should be made to values in the relationship(s) matched by the `MATCH` parameter, and potentially the sub-graph beneath.

```
input OrganizationMembersUpdateInput {
  MATCH: OrganizationMembersQueryInput
  SET: OrganizationMembersUpdateMutationInput!
}
```

The `SET` input is of type `OrganizationMembersUpdateMutationInput`, shown in the snippet below. The `joinDate` attribute is the same input type used during relationship creation operations, described in the section above. The `src` and `dst` attributes allow a single update to provide new values not only for the relationship properties, but also properties on the source and destination nodes at the ends of the relationship.

```
input OrganizationMembersUpdateMutationInput {
  dst: OrganizationMembersDstUpdateMutationInput
  joinDate: String
  src: OrganizationMembersSrcUpdateMutationInput
}
```

The source and destination node input types are shown in the schema snippets below. Note that the types, `OrganizationUpdateMutationInput` and `UserUpdateMutationInput` are the same input types used for the `SET` attributes in the single node update operation, described in in the section on single-node mutation operations below. Thus, we have hit the point where the GraphQL schema structure that Warpgrapher generates is recursive. A relationship update mutation can update the properties on the relationship, as described just above, or using this recursive input structure, reach down into the source and destination nodes at the ends of the relationship and edit their properties as well.

```
input OrganizationMembersSrcUpdateMutationInput {
  Organization: OrganizationUpdateMutationInput
}

input OrganizationMembersDstUpdateMutationInput {
  User: UserUpdateMutationInput
}
```

The output for updating one or more relationships, `OrganizationMembersRel`, is the same output type returned from querying for an organization's members relationship, as was described in the section on queries, above. For update operations, it returns the list of relationships that were updated in the mutation.

#### Deleting a Relationship

The input for a relationship delete mutation, `OrganizationMembersDeleteInput`, is shown in the schema snippet below. The `MATCH` attribute is used to query for the relationships that are desired to be deleted. Note that the input type, `OrganizationMembersQueryInput` is the same input type used to query for relationships under the relationship query in the GraphQL root `Query` object, described in the section on querying, above.

```
input OrganizationMembersDeleteInput {
  MATCH: OrganizationMembersQueryInput
  dst: OrganizationMembersDstDeleteMutationInput
  src: OrganizationMembersSrcDeleteMutationInput
}
```

The src and destination delete mutation inputs are not particularly interesting for this simple schema. The input type for the src of the relationship contains a single `Organization` attribute that has the same type as the deletion input for an `OrganizationDelete` mutation. However, the only option in that type is deletion of members, which is what is already being done. On the destination side, because the `User` type has no relationships of its own, the `UserDeleteMutationInput` object is empty altogether. Thus, for the most part, the `src` and `dst` attriubtes on the `OrganizationMembersDeleteInput` are not particularly useful, though in more complex models, they allows the possibility of deleting multiple nodes and relationships in a single query.

```
input OrganizationMembersSrcDeleteMutationInput {
  Organization: OrganizationDeleteMutationInput
}

input OrganizationMembersDstDeleteMutationInput {
  User: UserDeleteMutationInput
}

input OrganizationDeleteMutationInput {
  members: [OrganizationMembersDeleteInput!]
}

input UserDeleteMutationInput
```

The output from the relationship deletion mutation is an integer with a count of the relationships deleted.

### Mutating a Node

In many ways, modifying a node in a data model that includes relationships is similar to what was described in the node-only portion of the book, previously. Thus, this section doesn't repeat that same content, instead focusing only on the changes the come from having a relationship in the mix.

#### Creating a Node

The snippet below contains the input for creation of an organization. Note the `members` attribute, of type `OrganizationMembersCreateMutationInput`, which allows for the creation of members attributes in the same mutation that creates the organization.  The `OrganizationMembersCreateMutationInput` input type is the same one that is used for the `CREATE` attribute in the `OrganizationMembersCreate` mutation under the root GraphQL `mutation` object. Thus, when creating a node, you can create members for it using the same full flexbility provided by the mutation dedicated to creating relationships. The recursive nature of the creation inputs allows for the creation of entire sub-graphs.

```
input OrganizationCreateMutationInput {
  id: ID
  members: [OrganizationMembersCreateMutationInput!]
  name: String
}
```

The rest of the inputs and output for the node creation mutation are the same as those described previously for a simpler model without relationships.

### Updating a Node

The `OrganizationUpdateInput` for making changes to organizations looks similar to the input types used for objects that don't have relationships. It has a `MATCH` attribute to select the objects to update, and a `SET` attribute to describe the changes to be made. The difference is in the 

```
input OrganizationUpdateInput {
  MATCH: OrganizationQueryInput
  SET: OrganizationUpdateMutationInput
}

input OrganizationUpdateMutationInput {
  members: [OrganizationMembersChangeInput!]
  name: String
}
```

The differences for the inclusion of relationships begin in the `OrganizationUpdateMutationInput` input type used to set new values for the nodes to be updated, which includes a `members` attribute of type `OrganizationMembersChangeInput`. There are three changes one could make to a relationship: add one or more new relationships to destination nodes, delete one or more relationships to destination nodes, or keep the relationships to the same set of destination nodes but make changes to the properties of one or more of those destination nodes.  Those options are captured in the `OrganizationMembersChangeInput` input type in the schema snippet below.

```
input OrganizationMembersChangeInput {
  ADD: OrganizationMembersCreateMutationInput
  DELETE: OrganizationMembersDeleteInput
  UPDATE: OrganizationMembersUpdateInput
}
```

The `OrganizationMembersCreateMutationInput` input type for the `ADD` operation is the same one that was described above as the `CREATE` attribute the section on mutations to create new relationships. This makes sense, as in this context it is already clear what the source node or nodes are, and the `ADD` attribute need only create the new relationships to be added. Similarly, the `OrganizationMembersDeleteInput` used for the `DELETE` attribute here is the same one that is used for the `OrganizationMembersDelete` operation under the root GraphQL `Mutation` type. The match will be scoped to the relationships under the source node(s) selected by the `OrganizationUpdateInput` `MATCH` query. As expected, the same is true for the `OrganizationMembersUpdateInput` input type used for the `UPDATE` attribute. It's the same as the input used for the `OrganizationMembersUpdate` mutation under the root GraphQL `Mutation` type.

### Deleting a Node

The `OrganizationDeleteInput` input type, shown in the schema snippet below, looks similar to the one for nodes without relationships. However, the `OrganizationDeleteMutationInput` is different, as it includes a `members` attribute of type `OrganizationMembersDeleteInput`, which is the same type used for the `OrganizationMembersDelete` mutation under the GraphQL root `Mutation` type. In the case of this model, this additional input does little. In a more complex model with multiple types of relationships, however, it would allow for deletion of whole subgraphs of nodes and relationships.

```
input OrganizationDeleteInput {
  DELETE: OrganizationDeleteMutationInput
  MATCH: OrganizationQueryInput
}

input OrganizationDeleteMutationInput {
  members: [OrganizationMembersDeleteInput!]
}
```

## Full Schema Listing

The full schema for the example above is included below.

```
input OrganizationMembersDeleteInput {
  MATCH: OrganizationMembersQueryInput
  dst: OrganizationMembersDstDeleteMutationInput
  src: OrganizationMembersSrcDeleteMutationInput
}

input OrganizationCreateMutationInput {
  id: ID
  members: [OrganizationMembersCreateMutationInput!]
  name: String
}

input OrganizationMembersCreateInput {
  CREATE: [OrganizationMembersCreateMutationInput!]
  MATCH: OrganizationQueryInput
}

input OrganizationMembersSrcQueryInput {
  Organization: OrganizationQueryInput
}

type Mutation {
  OrganizationCreate(
    input: OrganizationCreateMutationInput!
    partitionKey: String
  ): Organization
  OrganizationDelete(partitionKey: String, input: OrganizationDeleteInput!): Int
  OrganizationMembersCreate(
    input: OrganizationMembersCreateInput!
    partitionKey: String
  ): [OrganizationMembersRel!]
  OrganizationMembersDelete(
    input: OrganizationMembersDeleteInput!
    partitionKey: String
  ): Int
  OrganizationMembersUpdate(
    partitionKey: String
    input: OrganizationMembersUpdateInput!
  ): [OrganizationMembersRel!]
  OrganizationUpdate(
    partitionKey: String
    input: OrganizationUpdateInput!
  ): [Organization!]
  UserCreate(input: UserCreateMutationInput!, partitionKey: String): User
  UserDelete(partitionKey: String, input: UserDeleteInput!): Int
  UserUpdate(partitionKey: String, input: UserUpdateInput!): [User!]
}

input OrganizationMembersChangeInput {
  ADD: OrganizationMembersCreateMutationInput
  DELETE: OrganizationMembersDeleteInput
  UPDATE: OrganizationMembersUpdateInput
}

input UserUpdateMutationInput {
  email: String
}

input UserDeleteInput {
  DELETE: UserDeleteMutationInput
  MATCH: UserQueryInput
}

input OrganizationMembersNodesMutationInputUnion {
  User: UserInput
}

input UserInput {
  EXISTING: UserQueryInput
  NEW: UserCreateMutationInput
}

input OrganizationQueryInput {
  id: StringQueryInput
  members: OrganizationMembersQueryInput
  name: StringQueryInput
}

union OrganizationMembersNodesUnion = User

type Query {
  Organization(
    partitionKey: String
    input: OrganizationQueryInput
  ): [Organization!]
  OrganizationMembers(
    partitionKey: String
    input: OrganizationMembersQueryInput
  ): [OrganizationMembersRel!]
  User(partitionKey: String, input: UserQueryInput): [User!]
  _version: String
}

input OrganizationMembersDstDeleteMutationInput {
  User: UserDeleteMutationInput
}

input OrganizationMembersUpdateInput {
  MATCH: OrganizationMembersQueryInput
  SET: OrganizationMembersUpdateMutationInput!
}

input OrganizationMembersSrcUpdateMutationInput {
  Organization: OrganizationUpdateMutationInput
}

input UserUpdateInput {
  MATCH: UserQueryInput
  SET: UserUpdateMutationInput
}

input OrganizationUpdateInput {
  MATCH: OrganizationQueryInput
  SET: OrganizationUpdateMutationInput
}

type OrganizationMembersRel {
  dst: OrganizationMembersNodesUnion!
  id: ID!
  joinDate: String
  src: Organization!
}

input OrganizationMembersUpdateMutationInput {
  dst: OrganizationMembersDstUpdateMutationInput
  joinDate: String
  src: OrganizationMembersSrcUpdateMutationInput
}

input OrganizationMembersSrcDeleteMutationInput {
  Organization: OrganizationDeleteMutationInput
}

input UserQueryInput {
  email: StringQueryInput
  id: StringQueryInput
}

input OrganizationMembersQueryInput {
  dst: OrganizationMembersDstQueryInput
  id: StringQueryInput
  joinDate: StringQueryInput
  src: OrganizationMembersSrcQueryInput
}

input OrganizationDeleteMutationInput {
  members: [OrganizationMembersDeleteInput!]
}

type Organization {
  id: ID!
  members(input: OrganizationMembersQueryInput): [OrganizationMembersRel!]
  name: String
}

input OrganizationUpdateMutationInput {
  members: [OrganizationMembersChangeInput!]
  name: String
}

type Subscription

input OrganizationMembersCreateMutationInput {
  dst: OrganizationMembersNodesMutationInputUnion!
  id: ID
  joinDate: String
}

input UserDeleteMutationInput

input OrganizationMembersDstUpdateMutationInput {
  User: UserUpdateMutationInput
}

type User {
  email: String
  id: ID!
}

input OrganizationMembersDstQueryInput {
  User: UserQueryInput
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

input OrganizationDeleteInput {
  DELETE: OrganizationDeleteMutationInput
  MATCH: OrganizationQueryInput
}
```