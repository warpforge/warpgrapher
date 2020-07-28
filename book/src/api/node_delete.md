# Node Delete

* [Delete node with matching props](#delete-node-with-matching-props)

### Delete node with matching props

To delete `Project` nodes with status "INACTIVE", call `ProjectDelete` with: 

```
mutation {
    ProjectDelete(
        input: {
            $MATCH: {
                status: "INACTIVE"
            },
            $DELETE: {
            }
        }
    )
    { ... }
}
```