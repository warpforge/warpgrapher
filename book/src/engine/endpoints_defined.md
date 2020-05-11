# Defined Endpoints

In addition to the CRUD endpoints auto-generated for each type, a Warpgrapher app serve additional defined GraphQL endpoints (root queries/mutations).

## Usage

#### 1. Add Endpoints to Config

The following config specified no types in the `model` section (so no CRUD endpoints will be generated), but defines several endpoints of varying inputs and outputs. 

```yaml
version: 1
model:

  # Team
  - name: Team
    props:
    - name: name
      type: String
    - name: size
      type: Int

endpoints:

  # GetAppName
  - name: GetAppName
    class: Query
    input: null
    output:
      type: String

  # GetLargetTeam
  - name: GetLargestTeam
    class: Query
    input: null
    output:
      type: Team
```

#### 2. Implement endpoint resolver logic

```rust
use warpgrapher::engine::resolvers::{ResolverContext, ExecutionResult};

// resolver that returns a Scalar (String)
fn resolve_getappname(
  context: ResolverContext<(), ()>
) -> ExecutionResult {

  context.resolve_scalar("MyAppName")
}

// resolver that returns a Node (Team)
fn resolve_getlargestteam(
  context: ResolverContext<(), ()>
) -> ExecutionResult {

  // query database to get team ...
  let largest_team_node = GraphNode {
      typename: "Team",
      props: json!({
        "name": "Blue Team",
        "size": 5
      })
      .as_object()
      .unwrap(),
    }
  )

  context.resolve_node(larget_team_node)
}
```

#### 3. Add resolvers when building `Engine`

```rust
use warpgrapher::Engine;

let mut resolvers = Resolvers<(), ()>::new();
resolvers.insert("GetAppName".to_string, Box::new(resolve_getappname));
resolvers.insert("GetLargestTeam".to_string, Box::new(resolve_getlargestteam));

let engine = Engine<(), ()>::new(config, db)
    .with_resolvers(resolvers)
    .build();
```

#### 4. Call Defined Endpoints

```
query {
  GetAppName
}
```

```json
{
  "data": {
    "GetAppName": "MyAppName"
  }
}
```

```
query {
  GetLargestTeam {
    id
    name
    size
  }
}
```

```json
{
  "data": {
    "GetLargestTeam": {
      "id": "123456789012345670",
      "name": "Blue Team",
      "size": 5
    }
  }
}
```