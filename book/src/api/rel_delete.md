# Relationship Delete

* [Delete Relationship](#delete-relationship)

## Delete relationship

The GraphQL

```
mutation {
  OrganizationMembersDelete(
    input: {
      MATCH: {
        src: { Organization: { name: { EQ: "Warpforge" } } }
        dst: { User: { email: { EQ: "constantine@example.com" } } }
      }
    }
  )
}
```

The output is as follows.

```json
{
  "data": {
    "OrganizationMembersDelete": 1
  }
}
```
