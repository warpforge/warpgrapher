# Relationship Read

* [Find relationships by src node](#find-relationships-by-src-node)
* [Find relationships by dst node](#find-relationships-by-dst-node)
* [Find relationships by rel props](#find-relationships-by-rel-props)

### Find relationships by src node

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

### Find relationships by dst node

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

### Find relationships by rel props

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