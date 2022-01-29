# Node Update

The GraphQL API examples below use the example schema described in the [Relationships](../configuration/relationships.html) section of the book. The unique IDs for nodes and relationships  in the examples below may differ than other sections and chapters of the book.

* [Match Node Properties](#match-node-properties)
* [Match Destination Properties](#match-destination-properties)
* [Add a Destination Node](#add-a-destination-node)
* [Update a Destination Node](#update-a-destination-node)
* [Delete a Relationship](#delete-a-relationship)

## Match Node Properties

The GraphQL query below match a node based on its properties and updates it.

```
mutation {
  OrganizationUpdate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      SET: { name: "Harsh Truth Heavy Industries" }
    }
  ) {
    id
    name
  }
}

```

The output is as follows.

```
{
  "data": {
    "OrganizationUpdate": [
      {
        "id": "1eea1d47-1fe8-4bed-9116-e0037fbdb296",
        "name": "Harsh Truth Heavy Industries"
      }
    ]
  }
}
```

## Match Destination Properties

The GraphQL query below matches a node based on properties on a desination node to which it is related, then updates it.

```
mutation {
  OrganizationUpdate(
    input: {
      MATCH: {
        members: { dst: { User: { email: { EQ: "balthazar@example.com" } } } }
      }
      SET: { name: "Prophet and Loss Inc." }
    }
  ) {
    id
    name
    members {
      id
      props {
        joinDate
      }
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

The output is as follows.

```
{
  "data": {
    "OrganizationUpdate": [
      {
        "id": "5692bd2a-2bc9-4497-8285-1f7860478cd6",
        "name": "Prophet and Loss Inc.",
        "members": [
          {
            "id": "78acc7ac-2153-413d-a8d7-688e472340d5",
            "props": {
              "joinDate": "2021-01-02"
            },
            "dst": {
              "id": "ea2a1b68-fda2-4adb-9c80-554761a1c97b",
              "email": "balthazar@example.com"
            }
          },
          {
            "id": "00051bc1-133c-445d-b00c-4faf61b2bffa",
            "props": {
              "joinDate": "2020-02-20"
            },
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

## Add a Destination Node

The GraphQL query below updates the Warpforge organization to add an additional, newly created user.  If an `EXISTING` attribute were used in place of `NEW` in the query below, one could query for existing users to add to the organization.

```
mutation {
  OrganizationUpdate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      SET: {
        members: {
          ADD: {
            props: { joinDate: "2018-01-08" }
            dst: { User: { NEW: { email: "constantine@example.com" } } }
          }
        }
      }
    }
  ) {
    id
    name
    members {
      id
      props {
        joinDate
      }
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

The output is as follows.

```
{
  "data": {
    "OrganizationUpdate": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Warpforge",
        "members": [
          {
            "id": "38cd72c8-75b5-4547-9829-38d6a6854eb9",
            "props": {
              "joinDate": "2018-01-08"
            },
            "dst": {
              "id": "f2e894bf-e98e-48a7-b16a-adc95cd34ac3",
              "email": "constantine@example.com"
            }
          },
          {
            "id": "bd302b7f-8a3f-49ab-aac3-c3348d8b8d94",
            "props": {
              "joinDate": "2020-02-20"
            },
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

## Update a Destination Node

The GraphQL query below updates a value on a destination node.

```
mutation {
  OrganizationUpdate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      SET: {
        members: {
          UPDATE: {
            MATCH: {
              dst: { User: { email: { EQ: "constantine@example.com" } } }
            }
            SET: { dst: { User: { email: "javier@example.com" } } }
          }
        }
      }
    }
  ) {
    id
    name
    members {
      id
      props {
        joinDate
      }
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

The output is as follows.

```
{
  "data": {
    "OrganizationUpdate": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Warpforge",
        "members": [
          {
            "id": "38cd72c8-75b5-4547-9829-38d6a6854eb9",
            "props": {
              "joinDate": "2018-01-08"
            },
            "dst": {
              "id": "f2e894bf-e98e-48a7-b16a-adc95cd34ac3",
              "email": "javier@example.com"
            }
          },
          {
            "id": "bd302b7f-8a3f-49ab-aac3-c3348d8b8d94",
            "props": {
              "joinDate": "2020-02-20"
            },
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

## Delete a Relationship

The GraphQL query below deletes the relationship from the Warpforge organization to alistair@example.com, removing them as a member of the organization.

```
mutation {
  OrganizationUpdate(
    input: {
      MATCH: { name: { EQ: "Warpforge" } }
      SET: {
        members: {
          DELETE: {
            MATCH: { dst: { User: { email: { EQ: "alistair@example.com" } } } }
          }
        }
      }
    }
  ) {
    id
    name
    members {
      id
      props {
        joinDate
      }
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

The output is as follows.

```
{
  "data": {
    "OrganizationUpdate": [
      {
        "id": "85faa40f-04a8-4f0a-ae44-804604b4ef4c",
        "name": "Warpforge",
        "members": [
          {
            "id": "38cd72c8-75b5-4547-9829-38d6a6854eb9",
            "props": {
              "joinDate": "2018-01-08"
            },
            "dst": {
              "id": "f2e894bf-e98e-48a7-b16a-adc95cd34ac3",
              "email": "javier@example.com"
            }
          }
        ]
      }
    ]
  }
}
```