# Node Create

* [Create node with no relationships](#create-node-with-no-relationships)
* [Create node with relationship to a NEW node](#create-node-with-relationship-to-a-new-node)
* [Create node with relationship to an EXISTING node](#create-node-with-relationship-to-an-existing-node)

### Create node with no relationships

To create a new `Project` node by itself:

```
mutation {
    ProjectCreate(
        input: {
            name: "Project 1",
            status: "ACTIVE"
        }
    )
    {
        id
        name
    }
}
```

Output:
```json
{
  "data": {
    "ProjectCreate": {
      "id": "a55e1dae-a87f-4ca1-8724-b967a678b942",
      "name": "Project 1"
    }
  }
}
```

### Create node with relationship to a `NEW` node

To create a new `Project` node with an `owner` relationship to a new `User` node:

```
mutation {
    ProjectCreate(
        input: {
            name: "Project 2",
            status: "ACTIVE",
            owner: {
                dst: {
                    User: {
                        NEW: {
                            name: "Smith"
                        }
                    }
                }
            }
        }
    )
    { ... }
}
```

### Create node with relationship to an `EXISTING` node

To create a new `Project` node with an `owner` relationship to an existing `User` node:

```
mutation {
    ProjectCreate(
        input: {
            name: "Project 3",
            status: "ACTIVE",
            owner: {
                dst: {
                    User: {
                        EXISTING: {
                            name: "Oneill"
                        }
                    }
                }
            }
        }
    )
    { ... }
}
```