# Node Read

* [Read All Nodes](#read-all-nodes)
* [Read nodes with matching props](#read-nodes-with-matching-props)
* [Return nodes with relationships to other nodes with matching props](#return-nodes-with-relationships-to-other-nodes-with-matching-props)


### Return all nodes 

To return all `Project` nodes:

```
query {
    Project {
        id
        name
        status
    }
}
```

### Return nodes with matching props

To return all `Project` nodes with `status == "ACTIVE"`:

```
query {
    Project(
        input: {
            status: "ACTIVE"
        }
    )
    {
        id
        name
    }
}
```

### Return nodes with relationships to other nodes with matching props

To returns all `Project` nodes containing an `owner` relationship to a `User` node with `name="Halsey"`:

```
query {
    Project(
        input: {
            owner: {
                dst: {
                    User: {
                        name: "Halsey"
                    }
                }
            }
        }
    )
    { ... }
}
```