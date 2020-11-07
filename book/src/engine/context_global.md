# Global Context

The GlobalContext feature enable the creation of **global state that is accessible across different event hook points**, including inside function resolvers.

The example below will demonstrate how to set a `tenant_id` variable in the global context that can be accessed within a function resolver and returned to the caller. 

### Usage

#### 1. Define GlobalContext struct

Define a struct containing all the owned variables of the global state. The struct must implement `Clone`, `Debug`, `Sync`, `Send` and warpgrapher `GlobalContext`. 

```rust,no_run,noplayground
{{#include ../../../examples/global_context/main.rs:26:31}}
```

#### 2. Create GlobalContext instance

Create an instance of the global context struct with initial values. 

```rust,no_run,noplayground
{{#include ../../../examples/global_context/main.rs:57:59}}
```

#### 3. Create an `Engine` with global context

The global context is passed to the engine via the builder pattern as an argument to the `.with_global_ctx()` method. The type of the global context must also be specified as the first type parameter of `Engine`. 

```rust,no_run,noplayground
{{#include ../../../examples/global_context/main.rs:66:70}}
```

#### 4. Use GlobalContext in a resolver

The global context is now accessible in function resolvers.

```rust,no_run,noplayground
{{#include ../../../examples/global_context/main.rs:33:42}}
```

### Full Example

View on [Github](https://github.com/warpforge/warpgrapher/blob/v0.5.0/examples/global_context/main.rs).
