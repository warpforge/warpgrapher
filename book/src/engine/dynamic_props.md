# Dynamic Props

When Warpgrapher auto-generates a CRUD endpoint, the values of Node and Relationship properties are retreived from the database and returned in a query. In some cases, however, it may be necessary to perform real-time computations to derive the value of a prop. We call these type of properties "dynamic properties", and Warpgrapher provides a mechanism to execute custom logic to resolve their values.

## Configuration

In the configuration below, `points` is a dynamic property on the `Project` type. It has an associated resolver name of `resolve_project_points`. That name will be used later to connect the Rust resolver function to this entry in the configuration.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_props/main.rs:13:21}}
```

## Implementation

The implementation below defines the resolver. In this example, the resolver simply returns a constant value. In a real system, the implementation might retrieve records and do some calculation to total up a number of points associated with a project.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_props/main.rs:33:40}}
```

## Add Resolvers to the Engine

The code in the snippet below adds the resolver function to a map. They key is the name for the custom resolver that was used in the configuration, above. The map is then passed to the Wargrapher engine, allowing the engine to find the resolver function when the dynamic property must be resolved.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_props/main.rs:55:66}}
```

## Example API Call

The following GraphQL query uses the dynamic resolver defined above.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_props/main.rs:67:80}}
```

## Full Example Source

See below for the full source code to the example above.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_props/main.rs}}
```




