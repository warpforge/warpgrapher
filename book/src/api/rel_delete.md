# Relationship Delete

* [Delete relationships](#delete-relationships)

### Delete relationships

```
mutation {
    ProjectOwnerDelete(input: {
        $MATCH: {
            src: {
                Project: {
                    name: "Project0"
                }
            }
        }
    })
}
```

Output:
```json
{
    "data": {
        "ProjectOwnerDelete": 1
    }
}
```
