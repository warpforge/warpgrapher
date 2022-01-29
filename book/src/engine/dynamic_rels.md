# Dynamic Relationships

Dynamic relationships are similiar to dynamic properties, but returning dynamically calculated relationships to other nodes as opposed to individual properties.

## Configuration

The configuration below includes a dynamic resolver called `resolve_project_top_contributor` for the `top_contributor` relationship. That resolver name will be used later to associate a Rust function to carry out the dynamic resolution.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:14:27}}
```

## Implementation

The next step is to define the custom resolution function in Rust. In this example, the custom relationship resolver creates a hard-coded node and relationship. In a real system, the function might load records and do some calculation or analytic logic to determine who is the top contributor to a project, and then return that user.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:40:60}}
```

## Add the Resolver to the Engine

The resolver is added to a map associated with the name used in the configuration, above. The map is then passed to the Warpgrapher engine. This allows the engine to find the Rust function implementing the custom resolver when it is needed.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:75:86}}
```

## Example API Call

The following GraphQL query uses the dynamic resolver defined above.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:88:107}}
```

## Full Example Source

See below for the full source code to the example above.

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs}}
```
