# Node Read

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [All Nodes](#all-nodes)
* [Node with Matching Properties](#node-with-matching-properties)
* [Node with Matching Relationships](#node-with-matching-relationships)
* [Node with Matching Destinations](#node-with-matching-destinations)

## All Nodes

The GraphQL query below lists all organizations.

```
query {
  Organization {
    id
    name
  }
}
```

The output is as follows.

```
{
  "data": {
    "Organization": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Just Us League"
      },
      {
        "id": "5692bd2a-2bc9-4497-8285-1f7860478cd6",
        "name": "Consortia Unlimited"
      },
      {
        "id": "1eea1d47-1fe8-4bed-9116-e0037fbdb296",
        "name": "Warpforge"
      }
    ]
  }
}
```

## Node with Matching Properties

The GraphQL query below lists all organizations with the name `Warpforge`.

```
query {
  Organization(input: { name: { EQ: "Warpforge" } }) {
    id
    name
  }
}
```

The output is as follows.

```
{
  "data": {
    "Organization": [
      {
        "id": "1eea1d47-1fe8-4bed-9116-e0037fbdb296",
        "name": "Warpforge"
      }
    ]
  }
}
```

## Node with Matching Relationships

The GraphQL query below lists all organizations with members that joined in 2020.

```
query {
  Organization(
    input: { members: { joinDate: { CONTAINS: "2020" } } }
  ) {
    id
    name
    members {
      joinDate
      dst {
        ... on User {
          id
          email
        }
      }
    }
  }
}
```

The output is as follows:

```
{
  "data": {
    "Organization": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Just Us League",
        "members": [
          {
            "joinDate": "2020-02-20",
            "dst": {
              "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
              "email": "alistair@example.com"
            }
          }
        ]
      },
      {
        "id": "5692bd2a-2bc9-4497-8285-1f7860478cd6",
        "name": "Consortia Unlimited",
        "members": [
          {
            "joinDate": "2020-02-20",
            "dst": {
              "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
              "email": "alistair@example.com"
            }
          }
        ]
      }
    ]
  }
}
```

## Node with Matching Destinations

The GraphQL query below lists all the organizations of which the user `alistair@example.com` is a member.

```
query {
  Organization(
    input: {
      members: { dst: { User: { email: { EQ: "alistair@example.com" } } } }
    }
  ) {
    id
    name
    members {
      joinDate
      dst {
        ... on User {
          id
          email
        }
      }
    }
  }
}
```

The output is as follows:

```
{
  "data": {
    "Organization": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Just Us League",
        "members": [
          {
            "joinDate": "2020-02-20",
            "dst": {
              "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
              "email": "alistair@example.com"
            }
          }
        ]
      },
      {
        "id": "5692bd2a-2bc9-4497-8285-1f7860478cd6",
        "name": "Consortia Unlimited",
        "members": [
          {
            "joinDate": "2020-02-20",
            "dst": {
              "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
              "email": "alistair@example.com"
            }
          }
        ]
      }
    ]
  }
}
```