
**[Node Read](#node-read)**

   * [Read All Nodes](#read-all-nodes)
   * [Read nodes with matching props](#read-nodes-with-matching-props)
   * [Read nodes with matching dst node props](#read-nodes-with-matching-dst-node-props) 
   * Read by traversing relationship upwards

**[Node Create](#node-create)**

   * [Create node](#create-node)
   * [Create node with relationships to new nodes](#create-node-with-relationships-to-new-nodes)
   * [Create Node with relationships to existing nodes](#create-node-with-relationships-to-existing-nodes)
  
**[Node Update](#node-update)**

   * [Update props on node with matching props](#update-node-properties)
   * [Create Relationships to new Nodes](#create-relationships-to-new-nodes)
   * [Create Relationships to existing Nodes](#create-relationships-to-existing-nodes)

**[Node Delete](#node-delete)**
   
   * [Delete Nodes that match Properties](#delete-nodes-that-match-properties)
   * [Delete Nodes that match Relationships](#delete-nodes-that-match-relationships)

**[Relationship Read](#relationship-read)**

   * [Read All Relationships](#read-all-relationships)
   * [Read Relationships that match Properties](#read-relationships-that-match-properties)
   * [Read Relationships that match source Node](#read-relationships-that-match-source-node)
   * [Read Relationships that match destination Node](#read-relationships-that-match-destination-node)

**[Relationship Create](#relationship-create)**

   * [Create relationship to new Node](#create-relationship-to-new-node)
   * [Create relationship to existing Node](#create-relationship-to-existing-node)

**[Relationship Update](#relationship-update)**

   * [Update Properties on Relationships that match Properties](#update-properties-on-relationships-that-match-properties)
   * [Update Properties on Relationships that match source Node](#update-properties-on-relationships-that-match-source-node)
   * [Update Properties on Relationships that match destination Node](#update-properties-on-relationships-that-match-destination-node)

**[Relationship Delete](#relationship-delete)**

   * [Delete Relationships that match Properties](#delete-relationships-that-match-properties)
   * [Delete Relationships that match source Node](#delete-relationships-that-match-source-node)
   * [Delete Relationships that match destination Node](#delete-relationships-that-match-destination-node)

# Node Read

### Read all Nodes

This query will return all `Project`. 

**GraphQL:**

```
query {
  Project {   ⌉  GqlNodeObject
    id        |
    name      |
  }           ｣
}
```

```json
[
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "ORION"
  },
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "SPARTAN-II"
  },
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "SPARTAN-III"
  },
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "SPARTAN-IV"
  }
]
```

### Read nodes with matching props

This query will return all *Projects* with a status of "ACTIVE".

**GraphQL:**

```
query {
  Project(input: {         ⌉  GqlNodeQueryInput
    status: "ACTIVE"       |
  })                       ｣
  {                        ⌉  GqlNodeObject
    id                     |   
    name                   |
  }                        ｣
}
```

```json
[
  {
    "__typename": "Project",
    "id": "<ID>",
    "name": "SPARTAN-II"
  },
  {
    "__typename": "Project",
    "id": "<ID>",
    "name": "SPARTAN-III"
  }
]
```


### Read nodes with matching dst node props

This query will return all *Project* nodes with "Catherine Halsey" as the owner.

**GraphQL:**

```
query {
  Project(input: {                  ⌉  GqlNodeQueryInput
    owner: {                        |  ⌉  GqlRelQueryInput
      dst: {                        |  |  ⌉  GqlRelNodesQueryInputUnion
        User: {                     |  |  |  ⌉  GqlNodeQueryInput
          name: "Catherine Halsey"  |  |  |  |
        }                           |  |  |  ｣
      }                             |  |  ｣
    }                               |  ｣
  }) {                              ｣
    id                              ⌉  GqlRelObject
    name                            |
  }                                 ｣
}
```

```json
[
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "SPARTAN-II"
  },
  {
    "__typename": "Project"
    "id": "<ID>",
    "name": "MJOLNIR"
  },
]
```

# Node Create

### Create node

This mutation will create a new *Project* with *name* and *status* properties. 

**GraphQL:**

```
mutation{
  ProjectCreate(input: {    ⌉  GqlNodeCreateMutationInput
    name: "ORION",          |
    status: "INACTIVE"      |
  })                        ｣
  {                         ⌉  GqlNodeObject
    id                      |
    name                    |
    status                  |
  }                         ｣
}
```

```json
{
  "__typename": "Project",   
  "id": "<ID>",
  "name": "ORION",
  "status": "INACTIVE"
}
```

#### Create node with relationships to new nodes

This mutation will create a new *Project*, a new *User*, and an *owner* relationship between them. 

**GraphQL:**

```
mutation {
  ProjectCreate(input: {                 ⌉  GqlNodeCreateMutationInput
    name: "SPARTAN-II",                  |
    status: "ACTIVE",                    |
    owner: {                             |  ⌉  GqlRelCreateMutationInput
      props: {                           |  |  ⌉  GqlRelPropsInput
        since: "2517"                    |  |  |
      }                                  |  |  ｣
      dst: {                             |  |  ⌉  GqlRelNodesMutationInputUnion  
        User: {                          |  |  |  ⌉  GqlNodeInput
          CREATE: {                      |  |  |  |  ⌉  GqlNodeCreateMutationInput
            name: "Catherine Halsey"     |  |  |  |  |
          }                              |  |  |  |  ｣
        }                                |  |  |  ｣
      }                                  |  |  ｣
    }                                    |  ｣
  })                                     ｣
  {                                      ⌉  GqlNodeObject
    id                                   |
    name                                 |
    status                               |
    owner {                              |  ⌉  GqlRelObject
      id                                 |  |
      props {                            |  |  ⌉  GqlRelPropsObject
        since                            |  |  |
      }                                  |  |  ｣
      dst {                              |  |  ⌉  GqlRelNodesUnion
        ...on User {                     |  |  |  ⌉  GqlNodeObject
          id                             |  |  |  |
          name                           |  |  |  |
        }                                |  |  |  ｣
      }                                  |  |  ｣
    }                                    |  ｣
  }                                      ｣
}                                        
```

```json
{
  "__typename": "Project",   
  "id": "<ID>",
  "name": "SPARTAN-II",
  "status": "ACTIVE",
  "owner": {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "props": {
      "since": "2517"
    },
    "dstNode": {
      "__typename": "User",
      "id": "<ID>",
      "name": "Catherine Halsey"
    }
  }
}
```

#### Create node with relationships to existing nodes

This query will create a new `Project`, and an *owner* relationship to an existing *User* node. 

```
mutation {
  ProjectCreate(input: {                 ⌉  GqlNodeCreateMutationInput
    name: "SPARTAN-II",                  |
    status: "ACTIVE",                    |
    owner: {                             |  ⌉  GqlRelCreateMutationInput
      props: {                           |  |  ⌉  GqlRelPropsInput
        since: "2517"                    |  |  |
      }                                  |  |  ｣
      dst: {                             |  |  ⌉  GqlRelNodesMutationInputUnion  
        User: {                          |  |  |  ⌉  GqlNodeInput
          MATCH: {                       |  |  |  |  ⌉  GqlNodeQueryInputInput
            name: "Catherine Halsey"     |  |  |  |  |
          }                              |  |  |  |  ｣
        }                                |  |  |  ｣
      }                                  |  |  ｣
    }                                    |  ｣
  })                                     ｣
  {                                      ⌉  GqlNodeObject
    id                                   |
    name                                 |
    status                               |
    owner {                              |  ⌉  GqlRelObject
      id                                 |  |
      props {                            |  |  ⌉  GqlRelPropsObject
        since                            |  |  |
      }                                  |  |  ｣
      dst {                              |  |  ⌉  GqlRelNodesUnion
        ...on User {                     |  |  |  ⌉  GqlNodeObject
          id                             |  |  |  |
          name                           |  |  |  |
        }                                |  |  |  ｣
      }                                  |  |  ｣
    }                                    |  ｣
  }                                      ｣
}                                        
```

```json
{
  "__typename": "Project",   
  "id": "<ID>",
  "name": "SPARTAN-II",
  "status": "ACTIVE",
  "owner": {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "props": {
      "since": "2517"
    },
    "dstNode": {
      "__typename": "User",
      "id": "<ID>",
      "name": "Catherine Halsey"
    }
  }
}
```

## Node Update

#### Update props on 

This query will match on all *Project* nodes with the property "name" equal to "ORION" and update their property "status" to "ACTIVE". 

**GraphQL:**

```
mutation {
  ProjectUpdate(input: {        ⌉  GqlNodeUpdateInput
    match: {                    |  ⌉  GqlNodeQueryInput
      name: "ORION"             |  |
    },                          |  ｣
    update: {                   |  ⌉  GqlNodeUpdateMutationInput
      status: "ACTIVE"          |  |
    }                           |  ｣
  })                            ｣
  {                             ⌉  GqlNodeObject
    id                          |
    name                        |
    status                      |
  }                             ｣
}
```

```json
[
  {
     "__typename": "Project",
     "id": "<ID>",
     "name": "ORION",
     "status": "ACTIVE"
  }
]
```

**SDK:**

```js
let update_project = client.update("Project", )
```

#### Create Relationships to new Nodes

This query will match on any `Project` with `name`="ORION" and create an `owner` relationship to a new `User`.

```
mutation {
  ProjectUpdate(input: {        ⌉  GqlNodeUpdateInput
    match: {                    |  ⌉  GqlNodeQueryInput
      name: "ORION"             |  |
    },                          |  ｣
    update: {                   |  ⌉  GqlNodeUpdateMutationInput
      owner: {                  |  |  ⌉  GqlRelUpdateMutationInput
        dst: {                  |  |  |  ⌉  GqlRelDstUpdateMutationInput
          User: {               |  |  |  |  ⌉  GqlNodeInput
            create: {           |  |  |  |  |  ⌉  GqlNodeCreateMutationInput
              name: "Halsey"    |  |  |  |  |  |
            }                   |  |  |  |  |  ｣
          }                     |  |  |  |  ｣
        }                       |  |  |  ｣
      }                         |  |  ｣
    }                           |  ｣
  })                            ｣
  {                             ⌉  GqlNodeObject
    id                          |
    name                        |
    status                      |
  }                             ｣
}
```

#### Create Relationships to existing Nodes

This query will match on any `Project` with `name`="ORION" and create an `owner` relationship to an existing `User`.

```
mutation {
  ProjectUpdate(input: {        ⌉  GqlNodeUpdateInput
    match: {                    |  ⌉  GqlNodeQueryInput
      name: "ORION"             |  |
    },                          |  ｣
    update: {                   |  ⌉  GqlNodeUpdateMutationInput
      owner: {                  |  |  ⌉  GqlRelUpdateMutationInput
        dst: {                  |  |  |  ⌉  GqlRelDstUpdateMutationInput
          User: {               |  |  |  |  ⌉  GqlNodeInput
            match: {            |  |  |  |  |  ⌉  GqlNodeCreateMutationInput
              name: "Halsey"    |  |  |  |  |  |
            }                   |  |  |  |  |  ｣
          }                     |  |  |  |  ｣
        }                       |  |  |  ｣
      }                         |  |  ｣
    }                           |  ｣
  })                            ｣
  {                             ⌉  GqlNodeObject
    id                          |
    name                        |
    status                      |
  }                             ｣
}
```

## Node Delete

Endpoint ([View Schema]()):

```
type Mutation {
    ProjectDelete (input: ProjectQueryInput): Int
}
```

#### Delete Nodes that match Properties

This query will delete all *Project* nodes with the "status" property equal to "INACTIVE" and return the number of nodes that were deleted. This query will fail if the matched nodes have existing relationships. 

**GraphQL:**

```
mutation{
  ProjectDelete(input: {    ⌉  GqlNodeDeleteInput
    match: {                |  | GqlQueryInput
      status: "INACTIVE"    |  |
    }
  })                        ｣
}
```

```json
"<Num>"
```

**SDK:**

```js
const deletedProjectIds = await wg.Project.delete({status: "INACTIVE"})
```

#### Force Delete Nodes that match Properties

This query will delete any `Project` node with `status` equal to "INACTIVE" and return the number of nodes deletedj. Any existing relationships to the matched nodes will be deleted.

**GraphQL:**

```
mutation{
  ProjectDelete(input: {    ⌉  GqlNodeDeleteInput
    match: {                |  ⌉ GqlQueryInput
      status: "INACTIVE"    |  |
    },                      |  ｣
    delete: {               |  ⌉ GqlNodeDeleteMutationInput
      force: true           |  |
    }                       |  ｣
  })                        ｣
}
```

```json
   "<Num>"
```

#### Delete Nodes that match Relationships

This query will delete any `Project` with an `owner` relationship to a `User` node with `name` equal to "Catherine Halsey". This query must specify the `force` flag or it will fail to delete any matched nodes. 

**GraphQL:**

```
mutation {
  ProjectDelete(input: {               ⌉  GqlNodeQueryInput
    match: {                           |  ⌉  GqlNodeQueryInput
      owner: {                         |  |  ⌉  GqlRelQueryInput
        dst: {                         |  |  |  ⌉  GqlRelNodesQueryInputUnion 
          User: {                      |  |  |  |  ⌉  GqlNodeQueryInput 
            name: "Catherine Halsey"   |  |  |  |  |
          }                            |  |  |  |  ｣
        }                              |  |  |  ｣
      }                                |  |  ｣
    },                                 |  ｣
    delete: {                          |  ⌉  GqlNodeDeleteMutationInput
      force: true                      |  |
    }                                  |  ｣
  })                                   ｣
}
```

```json
<Num>
```

## Relationship Read

Endpoint ([View Schema]()):

```
type Query{
    ProjectOwner (input: ProjectOwnerQueryInput): [ProjectOwnerRel]
}
```

#### Read all Relationships

This query will return all `owner` relationships belonging to *any* `Project`. 

**GraphQL:**

```
query {
  ProjectOwner {       ⌉  GqlRelObject
    id                 |
    src {              |  ⌉  GqlNodeObject
      id               |  |
      name             |  |
    }                  |  ｣
    dst {              |  ⌉  GqlRelNodesUnion
      ... on User {    |  |  ⌉  GqlNodeObject  
        id             |  |  |
        name           |  |  |
      }                |  |  ｣
    }                  |  ｣
    props {            |  ⌉  GqlRelPropsObject
      since            |  |
    }                  |  ｣
  }                    ｣  
}                    
```
   
```json
[
  {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "src": {
      "__typename": "Project"
      "id": "<ID>",
      "name": "SPARTAN-II"
    }
    "dst": {
      "__typename": "User",
      "id": "<ID>",
      "name": "Catherine Halsey"
    },
    "props": {
      "since": "2520"
    }
  },
  {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "src": {
      "__typename": "Project"
      "id": "<ID>",
      "name": "SPARTAN-III"
    }
    "dst": {
      "__typename": "User",
      "id": "<ID>",
      "name": "James Ackerson"
    },
    "props": {
      "since": "2520"
    }
  }
]
```   

#### Read relationships with matched props

This query will return `issues` relationships belonging to any `Project` with `components` equal to "MJOLNIR". 

**GraphQL:**

```
query {
  ProjectIssues(input: {       ⌉  GqlRelQueryInput
    props: {                   |  ⌉  GqlRelPropsInput
      components: "MJOLNIR"    |  |
    }                          |  ｣
  })                           ｣
  {                            ⌉  GqlRelObject
    id                         |
    src {                      |  ⌉  GqlNodeObject
      id                       |  |
      name                     |  |
    }                          |  ｣
    dst {                      |  ⌉  GqlRelNodesUnion 
      ... on User {            |  |  ⌉  GqlNodeObject
        id                     |  |  |
        name                   |  |  |
      }                        |  |  ｣
    }                          |  |
    props {                    |  ⌉  GqlRelPropsObject
      components               |  |
    }                          |  ｣
  }                            ｣
}
```

```json
[
  {
    "__typename": "ProjectIssuesRel",
    "id": "<ID>",
    "src": {
      "__typename": "Project",
      "id": "<ID>",
      "name": "SPARTAN-II"
    },
    "dst": {
      "__typename": "Feature",
      "id": "<ID>",
      "name": "Improve integration with MJOLNIR MKV"
    },
    "props": {
      "components": "MJOLNIR"
    }
  },
  {
    "__typename": "ProjectIssuesRel",
    "id": "<ID>",
    "src": {
      "__typename": "Project",
      "id": "<ID>",
      "name": "SPARTAN-V"
    },
    "dst": {
      "__typename": "Feature",
      "id": "<ID>",
      "name": "Improve integration with MJOLNIR MKVIII"
    },
    "props": {
      "components": "MJOLNIR"
    }
  }
]
```

#### Read relationships with matched src node props

This query will return all `owner` relationships for any `Project` with `status` equal to "INACTIVE".

```
query {
  ProjectOwner(input: {       ⌉  GqlRelQueryInput
    src: {                    |  ⌉  GqlRelSrcQueryInput
      Project: {              |  |  ⌉  GqlNodeQueryInput
        status: "INACTIVE"    |  |  |
      }                       |  |  ｣
    }                         |  ｣
  })                          ｣
  {                           ⌉  GqlRelObject
    id                        |
    src {                     |  ⌉  GqlNodeObject  
      id                      |  |
      name                    |  |
    }                         |  |
    dst {                     |  |  GqlRelNodesUnion
      ... on User {           |  |  ⌉  GqlNodeObject
        id                    |  |  |
        name                  |  |  |
      }                       |  |  ｣
    }                         |  ｣
  }                           ｣
}
```

```json
[
  {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "src": {
      "__typename": "Project"
      "id": "<ID>",
      "name": "ORION"
    }
    "dst": {
      "__typename": "User",
      "id": "<ID>",
      "name": "UNKNOWN"
    },
    "props": {
      "since": "UNKOWN"
    }
  }
]
```

#### Read relationships with matched dst node props

This query will return all `issues` relationships for any `Project` with an `owner` relationship to a `User` with `name` equal to "Halsey". 

```
query {
  ProjectOwner(input: {       ⌉  GqlRelQueryInput
    dst: {                    |  ⌉  GqlNodeQueryInput
      User: {
        status: "INACTIVE"      |  |
      }
    }                         |  ｣
  })                          ｣
  {                           ⌉  GqlRelObject
    id                        |
    src {                     |  ⌉  GqlNodeObject  
      id                      |  |
      name                    |  |
    }                         |  |
    dst {                     |  |  GqlRelNodesUnion
      ... on User {           |  |  ⌉  GqlNodeObject
        id                    |  |  |
        name                  |  |  |
      }                       |  |  ｣
    }                         |  ｣
  }                           ｣
}
```

## Relationship Create

Endpoint ([View Schema]()):

```graphql
type Mutation {
    ProjectOwnerCreate (input: ProjectOwnerCreateInput): ProjectOwnerRel
}
```

#### Create relationship to new node

```
mutation {
  ProjectOwnerCreate(input: {         ⌉  GqlRelCreateInput
    match: {                          |  ⌉  GqlNodeQueryInput
      name: "SPARTAN-III"             |  |
    },                                |  ｣
    create: {                         |  |  GqlRelCreateMutationInput
      props: {                        |  |  ⌉  GqlRelPropsInput
        since: "2040"                 |  |  |
      },                              |  |  ｣
      dst: {                          |  |  ⌉  GqlRelNodesMutationInputUnion
        User: {                       |  |  |  ⌉  GqlNodeInput
          CREATE: {                   |  |  |  |  ⌉  GqlNodeCreateMutationInput
            name: "James Ackerson"    |  |  |  |  |
          }                           |  |  |  |  ｣
        }                             |  |  |  ｣
      }                               |  |  ｣
    }                                 |  ｣
  })                                  ｣
  {                                   ⌉  GqlRelObject
    id                                |
    props {                           |  ⌉ GqlRelPropsObject
      since                           |  |   
    }                                 |  ｣
    src {                             |  ⌉  GqlNodeObject
      id                              |  |
      name                            |  |
    }                                 |  |
    dst {                             |  ⌉  GqlRelNodesUnion
      ... on User {                   |  |  ⌉  GqlNodeObject
        id                            |  |  |
        name                          |  |  |
      }                               |  |  ｣
    }                                 |  ｣
  }                                   ｣
}
```

```json
{
  "__typename": "ProjectOwnerRel",
  "id": "<ID>",
  "props": {
    "since": "2040"
  },
  "srcNode": {
     "__typename": "Project",
     "id": "<ID>",
     "name": "SPARTAN-III"
  },
  "dstNode": {
     "__typename": "User",
     "id": "<ID>",
     "name": "James Ackerson"
  }
}
```

Note: For [Single Node Relationships]() (like Project.owner in this case), this endpoint will replace the previous relationship. 

#### Create relationship to existing node

This query will create and return an "owner" relationship between *Project* "SPARTAN-IV" and *User* "Musa Ghanem". 

```
mutation {
  ProjectOwnerCreate(input: {     ⌉  GqlRelCreateInput
    match: {                      |  ⌉  GqlNodeQueryInput
      name: "SPARTAN-IV"          |  |
    },                            |  ｣  
    create: {                     |  ⌉  GqlRelCreateMutationInput  
      props: {                    |  |  ⌉  GqlRelPropsInput
        since: "2553"             |  |  |
      },                          |  |  ｣
      dst: {                      |  |  ⌉ GqlRelNodesMutationInputUnion
        User: {                   |  |  |  ⌉ GqlNodeInput
          MATCH: {                |  |  |  |  ⌉  GqlNodeQueryInput
            name: "Musa Ghanem"   |  |  |  |  |
          }                       |  |  |  |  ｣
        }                         |  |  |  ｣
      }                           |  |  ｣
    }                             |  ｣
  })                              ｣
  {
    id
    props {
      since
    }
    srcNode {
      id
      name
    }
    dstNode {
      id
      name
    }
  }
}
```

```json
{
  "__typename": "ProjectOwnerRel",
  "id": "<ID>",
  "props": {
    "since": "2040"
  },
  "srcNode": {
     "__typename": "Project",
     "id": "<ID>",
     "name": "SPARTAN-IV"
  },
  "dstNode": {
     "__typename": "User",
     "id": "<ID>",
     "name": "Musa Ghanem"
  }
}
```

## Relationship Update

#### Update props on relationships with matched props

This query will match all `ProjectOwner` relationships with `since` equal to "2050" and update that same prop to "2060".

```
mutation {
  ProjectOwnerUpdate(input: {     ⌉  GqlRelUpdateInput
    match: : {                    |  ⌉  GqlRelQueryInput
      props: {                    |  |  ⌉  GqlRelPropsInput
        since: "2050"             |  |  |
      }                           |  |  ｣
    },                            |  ｣
    update: {                     |  ⌉  GqlRelCreateMutationInput
      props: {                    |  |  ⌉  GqlRelPropsInput
        since: "2060"             |  |  |
      }                           |  |  ｣
    }                             |  ｣
  })                              ｣
  {
    id
    props {
      since
    }
  }
}
```

```json
[
  {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "props": {
      "since": "2060"
    }
  }
]
```

#### Update props on relationships with matched dst node props

This query will match `ProjectOwner` relationships with a dst `User` with `name` equal to "Halsey", and update the `since` prop of that relationship to "2060". 

```
mutation {
  ProjectOwnerUpdate(input: {     ⌉  GqlRelUpdateInput
    match: : {                    |  ⌉  GqlRelQueryInput
      dst: {                      |  |  ⌉  GqlRelNodesQueryInputUnion
        User: {                   |  |  |  ⌉  GqlNodeQueryInput
          name: "Halsey"          |  |  |  |
        }                         |  |  |  ｣
      }                           |  |  ｣
    },                            |  ｣
    update: {                     |  ⌉  GqlRelUpdateMutationInput
      props: {                    |  |  ⌉  GqlRelPropsInput
        since: "2060"             |  |  |
      }                           |  |  ｣
    }                             |  ｣
  })                              ｣
  {
    id
    props {
      since
    }
  }
}
```

```json
[
  {
    "__typename": "ProjectOwnerRel",
    "id": "<ID>",
    "props": {
      "since": "2060"
    }
  }
]
```


#### Extend graph via relationships with matched src node relationship

This query will match `ProjectOwner` relationships where the src `Project` has an `issues` relationship to a `Bug` with `severity` "HIGH", and update the relationship's `since` value to "2060". 

```
mutation {
  ProjectOwnerUpdate(input: {            ⌉  GqlRelUpdateInput
    match: : {                           |  ⌉  GqlRelQueryInput
      src: {                             |  |  ⌉  GqlRelSrcQueryInput
        Project: {                       |  |  |  ⌉  GqlNodeQueryInput
          issues: {                      |  |  |  |  ⌉  GqlRelQueryInput
            dst: {                       |  |  |  |  |  ⌉  GqlRelDstQueryInput
              Bug: {                     |  |  |  |  |  |  ⌉  GqlNodeQueryInput
                severity: "HIGH"         |  |  |  |  |  |  |
              }                          |  |  |  |  |  |  ｣
            }                            |  |  |  |  |  ｣
          }                              |  |  |  |  ｣
        }                                |  |  |  ｣
      }                                  |  |  ｣
    },                                   |  ｣
    update: {                            |  ⌉  GqlRelUpdateMutationInput
      src: {                             |  |  ⌉  GqlRelSrcUpdateMutationInput
        Project: {                       |  |  |  ⌉  GqlNodeUpdateMutationInput
          issues: {                      |  |  |  |  ⌉  GqlRelUpdateMutationInput
            dst: {                       |  |  |  |  |  ⌉  GqlRelDstUpdateMutationInput
              Feature: {                 |  |  |  |  |  |  ⌉  GqlNodeInput
                match: {                 |  |  |  |  |  |  |  ⌉  GqlNodeQueryInput
                  name: "something"      |  |  |  |  |  |  |  |
                }                        |  |  |  |  |  |  |  ｣
              }                          |  |  |  |  |  |  ｣
            }                            |  |  |  |  |  ｣
          }                              |  |  |  |  ｣
        }                                |  |  |  ｣
      }                                  |  |  ｣
    }                                    |  ｣
  })                                     ｣
  {
    id
    props {
      since
    }
  }
}
```

## Relationship Delete

#### Delete relationships that matches props

This query will delete all `issues` relationships where the component is "MJOLNIR" and will return the IDs of the deleted relationships. 

```
mutation {
  ProjectIssuesDelete(input: {     ⌉  GqlRelDeleteInput
    match: {                       |  ⌉  GqlRelQueryInput
      props: {                     |  |  ⌉  GqlRelPropsInput
        component: "MJOLNIR"       |  |  |
      }                            |  |  ｣
    }                              |  ｣
  })                               ｣
  {
    id
    name
  }
}
```

```json
[
  "<ID>",
  "<ID>"
]
```

#### Delete relationships that match source node props

This query will delete all the `issues` relationships from *Project* "ORION". 

```
mutation {
  ProjectIssuesDelete(input: {     ⌉  GqlRelDeleteInput
    match: {                       |  ⌉  GqlRelQueryInput
      src: {                       |  |  ⌉  GqlRelSrcQueryInput
        Project: {                 |  |  |  ⌉  GqlNodeQueryInput
          name: "ORION"            |  |  |  |
        }                          |  |  |  ｣
      }                            |  |  ｣
    }                              |  ｣
  })                               ｣
  {
    id
    name
  }
}
```

```json
[
  "<ID>",
  "<ID>"
]
```

#### Delete Relationships that match destination Node

This query will delete all `owner` relationships to *User* "Catherine Halsey".

```
mutation {
  ProjectOwnerDelete(input: {      ⌉  GqlRelDeleteInput
    match: {                       |  ⌉  GqlRelQueryInput
      dst: {                       |  |  ⌉  GqlRelSrcQueryInput
        User: {                    |  |  |  ⌉  GqlNodeQueryInput
          name: "Halsey"           |  |  |  |
        }                          |  |  |  ｣
      }                            |  |  ｣
    }                              |  ｣
  })                               ｣
  {
    id
    name
  }
}
```

```json
[
  "<ID>",
  "<ID>"
]
```
