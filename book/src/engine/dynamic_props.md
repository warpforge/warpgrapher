# Dynamic Props

When Warpgrapher auto-generates a CRUD endpoint, the values of Node and Relationship properties are retreived from the database and returned in a query. In some cases, however, it may be necessary to perform real-time computations to derive the value of a prop. We call these type of properties "dynamic properties", and Warpgrapher provides a mechanism to execute custom logic to resolve the value of the prop. 

## Usage

#### 1. Mark a properties as dynamic by setting the resolver field

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:11:20}}
```

#### 2. Define custom logic that resolve the prop value

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:22:30}}
```

#### 3. Add prop resolver when building `Engine`

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:44:52}}
```
