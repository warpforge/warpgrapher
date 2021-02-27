# Dynamic Relationships

Dynamic relationships are similiar to Dynamic Props. Instead of returning values contained in the database, Dynamic relationships allows values to be computed at request time. 

## Usage

#### 1. Mark rel as dynamic by setting the resolver field

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:15:28}}
```

#### 2. Define custom logic that resolve the prop value

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:41:61}}
```

#### 3. Add the custom relationship resolver to the engine

```rust,no_run,noplayground
{{#include ../../../examples/dynamic_rels/main.rs:75:86}}
```
