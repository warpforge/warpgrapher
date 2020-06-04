# Dynamic Props

When Warpgrapher auto-generates a CRUD endpoint, the values of Node and Relationship properties are retreived from the database and returned in a query. In some cases, however, it may be necessary to perform real-time computations to derive the value of a prop. We call these type of properties "dynamic properties", and Warpgrapher provides a mechanism to execute custom logic to resolve the value of the prop. 

## Usage

#### 1. Mark a properties as dynamic by setting the resolver field

```config
version: 1
model: 
 - name: Project
   properties: 
    - name: points
      type: int
      resolver: project_points
```

#### 2. Define custom logic that resolve the prop value

```rust
fn resolve_projectpoints(
    context: ResolverContext<AppGlobalContext, ()>
) -> ExecutionResult {

    // compute value ...
    let value = 
    
    context.resolve_scalar(value)
}
```

#### 3. Add prop resolver when building `Engine`

```rust
let mut resolvers = Resolvers<(), ()>::new();
resolvers.insert("project_points".to_string, Box::new(resolve_projectpoints));

let engine = Engine<(), ()>::new(config, db)
    .with_resolvers(resolvers)
    .build()
```