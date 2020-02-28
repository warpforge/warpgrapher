# Relationship Operations


**[Rel Read](#rel-read)**
  * [Find relationships by src node](#find-relationships-by-src-node)
  * [Find relationships by dst node](#find-relationships-by-dst-node)
  * [Find relationships by rel props](#find-relationships-by-rel-props)

**[Rel Create](#rel-create)**
  * [Create relationship between pair of existing nodes](#create-relationship-between-pair-of-existing-nodes)
  * [Create relationship from existing node to new node](#create-relationship-from-existing-node-to-new-node)

**[Rel Update](#rel-update)**
  * [Update relationship props](#update-relationship-props)

**[Rel Delete](#rel-delete)**
  * [Delete relationships](#delete-relationships)

# `Rel Read`

#### Find relationships by src node

```
query {
    ProjectIssues(input: {
        src: {
            Project: {
                name: "Project 0"
            }
        }
    })
    {
        id
    }
}
```

#### Find relationships by dst node

```
query {
    ProjectIssues(input: {
        dst: {
            Feature: {
                name: "Add new button"
            }
        }
    })
    {...}
}
```


#### Find relationships by rel props

```
query {
    ProjectOwner(input: {
        props: {
            since: "2015"
        }
    })
    {...}
}
```


# `Rel Create`

#### Create relationship between pair of existing nodes

```
mutation {
    ProjectOwnerCreate(input: {
        $MATCH: {
            name: "Project0"
        },
        $CREATE: {
            dst: {
                User: {
                    $EXISTING: {
                        name: "Jane"
                    }
                }
            }
        } 
    })
    {...}
}
```


#### Create relationship between from existing node to new node

```
mutation {
    ProjectOwnerCreate(input: {
        $MATCH: {
            name: "Project0"
        },
        $CREATE: {
            dst: {
                User: {
                    $NEW: {
                        name: "Joe"
                    }
                }
            }
        } 
    })
    {...}
}
```

# `Rel Update`

#### Update relationships props

```
mutation {
    ProjectOwnerUpdate(input: {
        $MATCH: {
            props: {
                sin
            }
        }
    })
    {
        id
    }
}
```


# `Rel Delete`

#### Delete relationships

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

