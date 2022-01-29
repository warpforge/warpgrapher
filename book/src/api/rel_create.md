# Relationship Create

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [Between Existing Nodes](#between-existing-nodes)
* [From an Existing to a New Node](#from-an-existing-to-a-new-node)
  
## Between Existing Nodes

The GraphQL query below creates a new membership relationship between two existing nodes, adding alistair@example.com to the Warpforge projct.

```
mutation {
  OrganizationMembersCreate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      CREATE: {
        props: { joinDate: "2022-01-28" }
        dst: { User: { EXISTING: { email: { EQ: "alistair@example.com" } } } }
      }
    }
  ) {
    id
    props {
      joinDate
    }
    src {
      id
      name
    }
    dst {
      ... on User {
        id
        email
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationMembersCreate": [
      {
        "id": "21173765-b2a3-4bb1-bfa7-5787ef17d6a8",
        "props": {
          "joinDate": "2022-01-28"
        },
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
          "email": "alistair@example.com"
        }
      }
    ]
  }
}
```

## From an Existing to a New Node

The GraphQL below creates a new membership relationship from an existing organization to a newly created user.

```
mutation {
  OrganizationMembersCreate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      CREATE: {
        props: { joinDate: "2022-01-28" }
        dst: { User: { NEW: { email: "constantine@example.com" } } }
      }
    }
  ) {
    id
    props {
      joinDate
    }
    src {
      id
      name
    }
    dst {
      ... on User {
        id
        email
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationMembersCreate": [
      {
        "id": "3ab33be6-16a3-4e50-87b5-3bb7d195ea54",
        "props": {
          "joinDate": "2022-01-28"
        },
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "c2b71308-2fd7-4d43-b037-30ec473e90a5",
          "email": "constantine@example.com"
        }
      }
    ]
  }
}
```