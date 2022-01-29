# Input Validation

In many cases, it's necessary to ensure that inputs are valid. What constitutes a valid input is up to the application, but it may mean that values have to be less than a certain length, within a certain range, and/or include or exclude certain characters. Warpgrapher makes it possible to write custom validation functions to reject invalid inputs.

## Configuration

In the configuration snippet below, the `name` property has a `validator` field with the name `NameValidator`. The `NameValidator` string will be used later to connect the Rust function with this definition in the schema.

```rust,no_run,noplayground
{{#include ../../../examples/validation/main.rs:12:20}}
```

## Implementation

The implementation below defines the input validation function itself. The function is relatively simple, rejecting the input if the name is "KENOBI".  All other names are accepted.

```rust,no_run,noplayground
{{#include ../../../examples/validation/main.rs:33:62}}
```

## Add Validators to the Engine

The validators, such as the one defined above, are packaged into a map from the name(s) used in the configuration to the Rust functions. The map is then provided to the Warpgrapher `Engine` as the engine is built.

```rust,no_run,noplayground
{{#include ../../../examples/validation/main.rs:76:84}}
```

## Example API Call

The follow example API call invokes the validator defined above.

```rust,no_run,noplayground
{{#include ../../../examples/validation/main.rs:86:98}}
```

## Full Example Source

See below for the full source code to the example above.

```rust,no_run,noplayground
{{#include ../../../examples/validation/main.rs}}
```


