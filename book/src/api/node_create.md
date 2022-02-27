# Node Create

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [Node with No Relationships](#node-with-no-relationships)
* [Node Related to a New Node](#node-related-to-a-new-node)
* [Node Related to an Existing Node](#node-related-to-an-existing-node)

## Node with No Relationships

The GraphQL query below creates a new organization.

```
mutation {
  OrganizationCreate(input: { name: "Warpforge" }) {
    id
    name
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationCreate": {
      "id": "edff7816-f40c-4be1-904a-b7ab62e60be1",
      "name": "Warpforge"
    }
  }
}
```

## Node Related to a New Node

The GraphQL query below creates a new organization with a relationship to a member who is a new user.

```
mutation {
  OrganizationCreate(
    input: {
      name: "Just Us League"
      members: {
        joinDate: "2020-02-20",
        dst: { User: { NEW: { email: "alistair@example.com" } } }
      }
    }
  ) {
    id
    name
    members {
      id
      joinDate
      dst {
        ... on User {
          id
          email
        }
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationCreate": {
      "id": "a33ab37b-af51-4ccd-88ee-7d4d6eb75de9",
      "name": "Just Us League",
      "members": [
        {
          "id": "295d191f-0d66-484c-b1eb-39494f0ae8a0",
          "joinDate": "2020-02-20",
          "dst": {
            "id": "5ca84494-dd14-468e-812f-cb2da07157db",
            "email": "alistair@example.com"
          }
        }
      ]
    }
  }
}
```

## Node Related to an Existing Node

The GraphQL query below creates a new organization with new relationship to an existing member, alistair@example.com, the same user created in the example above.

```
mutation {
  OrganizationCreate(
    input: {
      name: "Consortia Unlimited"
      members: {
        joinDate: "2020-02-20",
        dst: { User: { EXISTING: { email: "alistair@example.com" } } }
      }
    }
  ) {
    id
    name
    members {
      id
      joinDate
      dst {
        ... on User {
          id
          email
        }
      }
    }
  }
}
```

The output is as follows:

```
{
  "data": {
    "OrganizationCreate": {
      "id": "9ecef884-2afc-457e-8486-e1f84c761050",
      "name": "Consortia Unlimited",
      "members": [
        {
          "id": "008fdc43-f3cf-48eb-a9e9-c5c753c65ee9",
          "joinDate": "2020-02-20",
          "dst": {
            "id": "5ca84494-dd14-468e-812f-cb2da07157db",
            "email": "alistair@example.com"
          }
        }
      ]
    }
  }
}
```

Note that the id for the member in this example is the same as that in the last example, because the relationship was created to the same user.
