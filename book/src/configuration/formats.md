# Warpgrapher Config

The [Quickstart](../warpgrapher/quickstart.html) demonstrated using a string constant to hold the Warpgrapher configuration. It is also possible to read the configuration from a YAML file or to build a configuration programmatically using the configuration module's API. The following three configurations are all equivalent.

## String Configuration 

The following is the string constant from the [Quickstart](../warpgrapher/quickstart.html).

```rust,no_run,noplayground
{{#include ../../../examples/quickstart/main.rs:9:17}}
```

## YAML File Configuration

The same configuration can be created in a YAML file, as follows.

`config.yaml`

```rust,no_run,noplayground
{{#include ../../../examples/yaml_config/config.yaml}}
```

The configuration can then be loaded and used to set up a `Configuration` struct.

`main.rs`

```rust,no_run,noplayground
{{#include ../../../examples/yaml_config/main.rs:23:24}}
```

## Programmatic Configuration

The code below shows the creation of the same configuration programmatically.

```rust,no_run,noplayground
{{#include ../../../examples/programmatic_config/main.rs:20:38}}
```

The programmatic version includes some function arguments that do not appear in the YAML versions of the configuration, because they take on default values when omitted from a YAML configuration.  For example, the `UsesFilter` on a property allows granular control over whether a property is included in create, read, update, and delete operations.  This allows, among other things, the creation of read-only attributes.  Similarly, the `EndpointsFilter` determines whether the `User` type has create, read, update, and delete operations exposed in the GraphQL schema. For example, if users are created by a separate account provisioning system, it might be desirable to filter out the create operation, so that the GraphQL schema doesn't allow the possibility of creating new users.