# Node Operations

**[Node Read](#node-read)**
   * [Read All Nodes](#read-all-nodes)
   * [Read nodes with matching props](#read-nodes-with-matching-props)
   * [Return nodes with relationships to other nodes with matching props](#return-nodes-with-relationships-to-other-nodes-with-matching-props)

**[Node Create](#node-create)**
   * [Create node with no relationships](#create-node-with-no-relationships)
   * [Create node with relationship to a NEW node](#create-node-with-relationship-to-a-new-node)
   * [Create node with relationship to an EXISTING node](#create-node-with-relationship-to-an-existing-node)

**[Node Update](#node-update)**
   * [Update props on node with matching props](#update-props-on-node-with-matching-props)
   * [Update props on node with a relationship to a node containing matching props](#update-props-on-node-with-a-relationship-to-a-node-containing-matching-props)
   * [Update node to add (multi-node) relationships](#update-node-to-add-multi-node-relationships)

**[Node Delete](#node-delete)**
   * [Delete node with matching props](#delete-node-with-matching-props)

# `Node Read`

#### Return all nodes 

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

#### Return nodes with matching props

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

#### Return nodes with relationships to other nodes with matching props

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

# `Node Create`

#### Create node with no relationships

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

#### Create node with relationship to a `NEW` node

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
                        $NEW: {
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

#### Create node with relationship to an `EXISTING` node

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
                        $EXISTING: {
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

# `Node Update`

#### Update props on node with matching props

To update the `status` of the `Project` node with `name == "Project 2"`:

```
mutation {
    ProjectCreate(
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

#### Update props on node with a relationship to a node containing matching props

To update the status of any `Project` nodes containing an `owner` relationship to the `User` with `name  == "Smith"`:

```
mutation {
    ProjectCreate(
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

#### Update node to add `(multi-node)` relationships

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

# `Node Delete`

#### Delete node with matching props

To delete `Project` nodes with status "INACTIVE", call `ProjectDelete` with: 

```
mutation {
    ProjectDelete(
        input: {
            $MATCH: {
                status: "INACTIVE"
            },
            $DELETE: {
                force: true
            }
        }
    )
    { ... }
}
```

The `force` flag will delete nodes even if they have existing relationships. 

