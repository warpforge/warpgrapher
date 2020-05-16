# Dynamic Relationships

Dynamic relationships are similiar to Dynamic Props. Instead of returning values contained in the database, Dynamic rels allows values to be computed at request time. 

## Usage

#### 1. Mark rel as dynamic by setting the resolver field

```config
version: 1
model: 
 - name: Project
   props: []
   rels:
    - name: topcontributor
      nodes: [User]
      props: []
      resolver: project_topcontributor
```

#### 2. Define custom logic that resolve the prop value

```rust
fn resolve_project_topcontributor(
    context: ResolverContext<AppGlobalContext, ()>
) -> ExecutionResult {

    // compute ...
    let rel = GraphRel {
        id: "1234567890",
        props: None,
        dst: GraphNode {
            typename: "User",
            props: json!({
                "id": "1234567890",
                "name": "Joe"
            })
        }
    };
    
    context.resolve_scalar(rel)
}
```

#### 3. Add prop resolver when building `Engine`

```rust
let mut resolvers = Resolvers<(), ()>::new();
resolvers.insert("project_topcontributor".to_string, Box::new(resolve_project_topcontributor));

let engine = Engine<(), ()>::new(config, db)
    .with_resolvers(resolvers)
    .build()
```