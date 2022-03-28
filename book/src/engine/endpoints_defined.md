# Defined Endpoints

In addition to the CRUD endpoints auto-generated for each type, Warpgrapher provides the ability to define additional custom endpoints. 

## Configuration

The schema for an endpoint entry in the Warpgrapher configuration is as follows.

```
endpoints:
  - name: String
    class: String         # /Mutation | Query/
    input:                # null if there is no input parameter
      type: String
      list: Boolean
      required: Boolean
    output:               # null if there is no input parameter
      type: String
      list: Boolean       # defaults to false
      required: Boolean   # defaults to false
```

The `name` of the endpoint will be used later as the key to a hash of endpoint resolution fuctions. It uniquely identified this endpoint. The `class` attribute tells Warpgrapher whether this endpoint belongs under the root query or root mutation object. The convention is that any operation with side effects, modifying the persistent data store, should be a mutation. Read-only operations are queries.  The `input` attribute allows specification of an input to the endpoint function. The input type may be a scalar GraphQL type -- `Boolean`, `Float`, `ID`, `Int`, or `String` -- or it may be a type defined elsewhere in the `model` section of the Warpgrapher configuration.  The `list` determines whether the input is actually a list of that type rather than a singular instance.  If the `required` attribute is true, the input is required.  If `false`, the input is optional.  The `output` attribute describes the value returned by the custom endpoint. It has fields similar to `input`, in that it includes `type`, `lsit`, and `required` attributes.


The following configuration defines a custom endpoints, `TopIssue`.


```yaml
{{#include ../../../examples/endpoints/main.rs:14:27}}
```

## Implementation

To implement the custom endpoint, a resolver function is defined, as follows. In this example, the function just puts together a static response and resolves it. A real system would like do some comparison of nodes and relationships to determine the top issue, and dynamically return that record.

```rust
{{#include ../../../examples/endpoints/main.rs:40:53}}
```

## Add Resolvers to the Warpgrapher Engine

To add the custom endpoint resolver to the engine, it must be associated with the name the endpoint was given in the configuration above. The example code below creates a `HashMap` to map from the custom endpoint name and the implementing function. That map is then passed to the `Engine` when it is created.

```rust
{{#include ../../../examples/endpoints/main.rs:67:75}}
```

## Example of Calling the Endpoint

The code below calls the endine with a query that exercises the custom endpoint.

```
{{#include ../../../examples/endpoints/main.rs:78:88}}
```

## Full Example Source

See below for the full code for the example above.

```
{{#include ../../../examples/endpoints/main.rs}}
```
