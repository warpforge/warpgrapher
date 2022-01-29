# Request Context

In some cases, it's desirable to pass custom state information from your application into the Warpgrapher request cycle, so that your custom resolvers can make use of that information. The request context makes this passing of state possible.

## Define the RequestContext

Every system using Warpgrapher defines a struct that implements `RequestContext`. In addition to implementing the trait, that struct is free to carry additional state information. However, the context must implement `Clone`, `Debug`, `Sync`, `Send`, as well as Warpgrapher's `RequestContext` trait. See the code snippet below for an example.

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:26:38}}
```

## Engine Type Parameter

The struct that implements `RequestContext` is passed to the `Engine` as a type parameter, as shown in the code snippet below.

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:64:68}}
```

## Access the Context

Once passed to the `Engine`, the struct implementing `RequestContext` is available to functions that implement custom endpoints and resolvers, as shown in the snippet below.

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:40:46}}
```

## Full Example Source

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs}}
```
