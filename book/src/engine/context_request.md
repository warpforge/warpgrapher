# Request Context

The Request Context feature enables the creation of **mutable state through the lifecycle of a request**.

### Usage

#### 1. Define RequestContext struct

Define a struct that contains mutable information to be available for the lifetime of a request. The request context must implement `Clone`, `Debug`, `Sync`, `Send`, and Warpgrapher `RequestContext`. 

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:28:41}}
```

#### 2. Create Engine with RequestContext type parameter

The RequestContext is specified in the second type paramter of `Engine`. 

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:66:69}}
```

#### 3. Access Context inside resolver

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:43:47}}
```

### Full Example

View on [Github](https://github.com/warpforge/warpgrapher/blob/v0.7.1/examples/request_context/main.rs).
