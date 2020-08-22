
## Explore

Navigate to the displayed URL. You should see the GraphQL Playground which allows you to interact directly with the API. 

#### List all `Users`

First, list all `User` nodes in the database:

```graphql
query {
  User {
    id
    email
  }
}
```

You should expect to see an empty list since the database is empty:

```json
{
  "data": {
    "User": []
  }
}
```

#### Create a new `User`

Next, create a new user:

```
mutation {
  UserCreate(input: {
    email: "user@example.com"
  }) {
    id
    email
  }
}
```

The response should display the newly created user:

```json
{
  "data": {
    "UserCreate": {
      "email": "user@example.com",
      "id": "0b2a6753-a5cf-46ea-b046-4935ea208950"
    }
  }
}
```

(Your `id` will of course differ). 

#### Create a new `Team`

Now, create a `Team` node:

```
mutation {
  TeamCreate(input: {
    name: "Blue Team"
  }) {
    id
    name
  }
}
```

Like before, you should see the newly created `Team` node:

```json
{
  "data": {
    "TeamCreate": {
      "id": "d381a0f7-8a01-49e3-80ff-15ba01f3604f",
      "name": "Blue Team"
    }
  }
}
```

#### Add `User` to `Team`

GraphQL and Neo4j are all about relatinships. Create a `users` relationship between the `Team` and `User` nodes:

```
mutation {
  TeamUsersCreate(input: {
    match: {
      name: "Blue Team"
    },
    create: {
      dst: {
        User: {
          EXISTING: {
            email: "user@example.com"
          }
        }
      }
    }
  }) {
    id
  }
}
```

```json
{
  "data": {
    "TeamUsersCreate": {
      "id": "e5d5e19a-70bf-4d04-b32f-e61407100914",
    }
  }
}
```

#### Execute nested query

Finally, query the `Team` and all related `User` nodes under the `users` relationship:

```
query {
  Team {
    id
    name
    users {
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

```json
{
  "data": {
    "Team": [
      {
        "id": "cbd63d09-13b9-4199-b926-94716b2a547c",
        "name": "Blue Team",
        "users": {
          "dst": {
            "email": "user@example.com",
            "id": "78c71745-6362-49b2-8b6d-3e19de3f4efc"
          }
        }
      }
    ]
  }
}
```