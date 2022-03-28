# Node Delete

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [Node with Matching Properties](#node-with-matching-properties)

## Node with Matching Properties

The GraphQL query below deletes a node based on matching against its properties.

```
mutation {
  OrganizationDelete(
    input: { MATCH: { name: { EQ: "Harsh Truth Heavy Industries" } } }
  )
}
```

The output is as follows, indicating that one organization was successfully deleted.

```
{
  "data": {
    "OrganizationDelete": 1
  }
}
```
