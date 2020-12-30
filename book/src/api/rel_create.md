# Relationship Create
  
* [Create relationship between pair of existing nodes](#create-relationship-between-pair-of-existing-nodes)
* [Create relationship from existing node to new node](#create-relationship-from-existing-node-to-new-node)

### Create relationship between pair of existing nodes

```
mutation {
    ProjectOwnerCreate(input: {
        MATCH: {
            name: "Project0"
        },
        CREATE: {
            dst: {
                User: {
                    EXISTING: {
                        name: "Jane"
                    }
                }
            }
        } 
    })
    {...}
}
```

### Create relationship between from existing node to new node

```
mutation {
    ProjectOwnerCreate(input: {
        MATCH: {
            name: "Project0"
        },
        CREATE: {
            dst: {
                User: {
                    NEW: {
                        name: "Joe"
                    }
                }
            }
        } 
    })
    {...}
}
```