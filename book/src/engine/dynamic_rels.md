# Dynamic Relationships

Dynamic relationships are similiar to Dynamic Props. Instead of returning values contained in the database, Dynamic relationships allows values to be computed at request time. 

## Usage

#### 1. Mark rel as dynamic by setting the resolver field

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:13:28}}
```

#### 2. Define custom logic that resolve the prop value

```rust,no_run,noplayground
{{#include ../../../examples/request_context/main.rs:30:48}}
```
