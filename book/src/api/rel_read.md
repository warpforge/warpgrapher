# Relationship Read

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [By Relationship Properties](#by-relationship-properties)
* [By Source Node](#by-source-node)
* [By Destination Node](#by-destination-node)

## By Relationship Properties

The GraphQL query below retrieves all the members who joined organizations on 2018-01-08.

```
query {
  OrganizationMembers(input: { joinDate: { EQ: "2018-01-08" } }) {
    id
    joinDate
    src {
      id
      name
    }
    dst {
      ... on User {
        id
        email
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationMembers": [
      {
        "id": "38cd72c8-75b5-4547-9829-38d6a6854eb9",
        "joinDate": "2018-01-08",
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "f2e894bf-e98e-48a7-b16a-adc95cd34ac3",
          "email": "javier@example.com"
        }
      }
    ]
  }
}
```

## By Source Node

The GraphQL query below retrieves all the members of users in the Warpforge organization.

```
query {
  OrganizationMembers(
    input: { src: { Organization: { name: { EQ: "Warpforge" } } } }
  ) {
    id
    joinDate
    src {
      id
      name
    }
    dst {
      ... on User {
        id
        email
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationMembers": [
      {
        "id": "3ab33be6-16a3-4e50-87b5-3bb7d195ea54",
        "joinDate": "2022-01-28",
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "c2b71308-2fd7-4d43-b037-30ec473e90a5",
          "email": "constantine@example.com"
        }
      },
      {
        "id": "21173765-b2a3-4bb1-bfa7-5787ef17d6a8",
        "joinDate": "2022-01-28",
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
          "email": "alistair@example.com"
        }
      },
      {
        "id": "38cd72c8-75b5-4547-9829-38d6a6854eb9",
        "joinDate": "2018-01-08",
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "f2e894bf-e98e-48a7-b16a-adc95cd34ac3",
          "email": "javier@example.com"
        }
      }
    ]
  }
}
```

## By Destination Node

The GraphQL query below retrieves all of the members of alistair@example.com.

```
query {
  OrganizationMembers(
    input: { dst: { User: { email: { EQ: "alistair@example.com" } } } }
  ) {
    id
    joinDate
    src {
      id
      name
    }
    dst {
      ... on User {
        id
        email
      }
    }
  }
}
```

The output is as follows.

```
{
  "data": {
    "OrganizationMembers": [
      {
        "id": "21173765-b2a3-4bb1-bfa7-5787ef17d6a8",
        "joinDate": "2022-01-28",
        "src": {
          "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
          "name": "Warpforge"
        },
        "dst": {
          "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
          "email": "alistair@example.com"
        }
      },
      {
        "id": "00051bc1-133c-445d-b00c-4faf61b2bffa",
        "joinDate": "2020-02-20",
        "src": {
          "id": "5692bd2a-2bc9-4497-8285-1f7860478cd6",
          "name": "Prophet and Loss Inc."
        },
        "dst": {
          "id": "de5e58cd-eb5e-4bf8-8a7a-9656999f4013",
          "email": "alistair@example.com"
        }
      }
    ]
  }
}
```