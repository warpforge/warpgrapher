# Defined Endpoints

In addition to the CRUD endpoints auto-generated for each type, Warpgrapher provides the ability to define additional endpoints. 

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
use std::collections::HashMap;
use warpgrapher::engine::resolvers::{ResolverFacade, ExecutionResult};
use warpgrapher::value::Value;

// resolver that returns a Scalar (String)
fn resolve_getappname(
  context: ResolverFacade<(), ()>
) -> ExecutionResult {

  facade.resolve_scalar("MyAppName")
}

// resolver that returns a Node (Team)
fn resolve_getlargestteam(
  facade: ResolverFacade<(), ()>
) -> ExecutionResult {

  // query database to get team ...
  let mut hm = HashMap::new();
  hm.insert("name".to_string(), Value::String("Blue Team".to_string()));
  hm.insert("size".to_string(), Value::Int64(5));
  
  let largest_team_node = facade.create_node(("Team", &hm);

  context.resolve_node(&larget_team_node)
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