# Resolvers

Resolvers are Warpgrapher's mechanism for implementing custom logic for Endpoints, Fields, and Validators. Resolvers take the form of regular rust functions with a specific signature. 

### Endpoint Resolvers

```rust
pub fn endpoint_resolver(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<(), ReqContext>>,
) -> ExecutionResult {
    /* logic */
}
```

### Field Resolver

```rust
pub fn field_resolver(
    _info: &Info,
    _args: &Arguments,
    executor: &Executor<GraphQLContext<(), ReqContext>>,
) -> ExecutionResult {
    /* logic */
}
```

### Validator Resolver

```rust
pub fn validator(value: &Value) -> Result<(), Error> {
    /* logic */
}
```

### Adding Resolvers

To pass resolvers to Warpgrapher, use the `with_resolver` attribute. 

```rust

// resolvers
let mut resolvers = WarpgrapherResolvers::<GlobalContext, ReqContext>::new();
resolvers.insert(
    "ProjectCount".to_string(),
    Box::new(project_count::resolver),
);
resolvers.insert(
    "ProjectPoints".to_string(),
    Box::new(project_points::resolver),
);

// server
let mut server: Server<(), ()> = Server::new(config, db)
    .with_resolvers(resolvers)
    .build();
```