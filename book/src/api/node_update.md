# Node Update

* [Update props on node with matching props](#update-props-on-node-with-matching-props)
* [Update props on node with a relationship to a node containing matching props](#update-props-on-node-with-a-relationship-to-a-node-containing-matching-props)
* [Update node to add (multi-node) relationships](#update-node-to-add-multi-node-relationships)

### Update props on node with matching props

To update the `status` of the `Project` node with `name == "Project 2"`:

```
mutation {
    ProjectUpdate(
        input: {
            $MATCH: {
                name: "Project 2"
            },
            $SET: {
                status: "INACTIVE"
            }
        }
    )
    { ... }
}
```

### Update props on node with a relationship to a node containing matching props

To update the status of any `Project` nodes containing an `owner` relationship to the `User` with `name  == "Smith"`:

```
mutation {
    ProjectUpdate(
        input: {
            $MATCH: {
                owner: {
                    dst: {
                        User: {
                            name: "Smith"
                        }
                    }
                }
            },
            $SET: {
                status: "INACTIVE"
            }
        }
    )
    { ... }
}
```

### Update node to add `(multi-node)` relationships

To update a specific `Project` node to add several `issue` relationships to new `Feature` and `Bug` nodes, call `ProjectUpdate` with:

```
mutation {
    ProjectUpdate(
        input: {
            $MATCH: {
                id: "<ID>"
            },
            $SET: {
                issues: [
                    {
                        $ADD: {
                            dst: {
                                Feature: {
                                    $NEW: {
                                        name: "Add login page"
                                    }
                                }
                            }
                        }
                    },
                    {
                        $ADD: {
                            dst: {
                                Bug: {
                                    $NEW: {
                                        name: "Fix auth bug"
                                    }
                                }
                            }
                        }
                    }
                ]
            }
        }
    ) 
    { ... }
}
```