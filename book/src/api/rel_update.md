# Relationship Update

* [Update Relationship Properties](#update-relationship-properties)

## Update Relationship Properties

The GraphQL updates the date on a membership.

```
mutation {
  OrganizationMembersUpdate(
    input: {
      MATCH: {
        src: { Organization: { name: "Warpforge" } }
        dst: { User: { email: "alistair@example.com" } }
      }
      SET: { joinDate: "2021-12-31" }
    }
  ) {
    id
    joinDate
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
    "OrganizationMembersUpdate": [
      {
        "id": "21173765-b2a3-4bb1-bfa7-5787ef17d6a8",
        "joinDate": "2021-12-31",
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
