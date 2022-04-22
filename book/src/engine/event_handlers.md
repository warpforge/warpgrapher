# Event Handlers

The earlier sections of the book covered a great many options for customizing the behavior of Warpgrapher, including input validation, request context, custom endpoints, and dynamic properties and relationships. Warpgrapher offers an additional API, the event handling API, to modify Warpgrapher's behavior at almost every point in the lifecycle of a request. Event handlers may be added before `Engine` creation, before or after request handling, and before or after nodes or relationships are created, read, updated, or deleted. This section will introduce the event handling API using an extended example of implementing a very simple authorization model. Each data record will be owned by one user, and only that user is entitled to read or modify that record.

## Configuration

Unlike some of the other customization points in the Warpgrapher engine, no special configuration is required for event handlers. They are created and added to the `Engine` using only Rust code. The data model used for this section's example is as follows.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:17:22}}
```

## Implementation

The example introduces four event hooks illustrating different lifecycle events. One event handler is set up for before the engine is built. It takes in the configuration and modifies it to insert an additional property allowing the system to track the owner of a given `Record`.  A second event handler runs before every request, inserting the current username into a request context so that the system can determine who is making a request, and thus whether that current user matches the ownership of the records being affected.  The remaining event handlers run after node read events and before node modification events, in order to enforce the access control rules.

### Before Engine Build

The following function is run before the engine is built. It takes in a mutable copy of the configuration to be used to set up Warpgrapher `Engine`. This allows before engine build event handlers to make any concievable modification to the configuration. They can add or remove endpoints, types, properties, relationships, dynamic resolvers, validation, or anything else that can be included in a configuration.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:58:75}}
```

In this example, the handler is iterating through the configuration, finding every type declared in the data model. To each type, the handler is adding a new owner property that will record the identity of the owner of the record. This will later be used to validate that only the owner can read and modify the data.

### Before Request Processing

The following event hook function is run before every request that is processed by the Warpgrapher engine. In a full system implementation, it would likely pull information from the `metadata` parameter, such as request headers like a JWT, that might be parsed to pull out user identity information. That data might then be used to look up a user profile in the database. In this case, the example simply hard-codes a username. It does, however, demonstrate the use of an application-specific request context as a means of passing data in for use by other event handlers or by custom resolvers.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:42:56}}
```

### Before Node Creation

The `insert_owner` event hook is run prior to the creation of any new nodes. The `Value` passed to the function is the GraphQL input type in the form of a Warpgrapher `Value`. In this case, the function modifies the input value to insert an additional property, the owner of the node about to be created, which is set to be the username of the current user. 

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:77:94}}
```

The modified input value is returned from the event hook, and when Warpgrapher continues executing the node creation operation, the owner property is included in the node creation operation, alongside all the other input properties.

### After Node Read

The `enforce_read_access` event hook, defined below, is set to run after each node read operation. The Rust function is passed a `Vec` of nodes that that were read. The event hook function iterates through the nodes that were read, pulling out their owner property. That owner property is compared with the current logged in username. If the two match, the node belongs to the user, and the node is retained in the results list.  If the two do not match, then the current logged in user is not the owner of the record, and the node is discarded from the results list without ever being passed back to the user.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:96:121}}
```

### Before Node Update and Delete

The `enforce_write_access` event hook, shown below, is set to run before each node update or delete operation. The Rust function is passed the `input` value that corresponds to the GraphQL schema `input` argument type for the update or delete operation. In this example implementation, the function executes the `MATCH` portion of the update or delete query, reading all the nodes that are intended to be modified. For each of the nodes read, the event handler tests whether the owner attribute is the current logged in username. If the two match, the node belongs to the current user, and it is kept in the result set. If the username does not match the owner property on the object, then the node is discarded.

Once the node list is filtered, the event handler constructs a new `MATCH` query that will match the unique identifiers of all the nodes remaining in the filtered list. This new `MATCH` query is returned from the event handler and used subsequently in Warpgrapher's automatically generated resolvers to do the update or deletion operation.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:123:172}}
```

Although not necessary for this use case, the event handler could have just east as easily modified the `SET` portion of the update query as the `MATCH`, in some way adjusting the values used to update an existing node.

## Add Handlers to the Engine

The event handlers are all added to an `EventHandlerBag` which is then passed to the Warpgrapher engine.  The registration function determines where in the life cycle the hook will be called, and in some cases, such as before and after node and relationship CRUD operation handlers, there are arguments to specify which nodes or relationships should be affected.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:186:198}}
```

## Example API Call

The following GraphQL query triggers at least the first several event handlers in the call. Other queries and mutations would be needed to exercise all of them.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs:200:212}}
```

## Full Example Source

See below for the full source code to the example above.

```rust,no_run,noplayground
{{#include ../../../examples/event_handlers/main.rs}}
```

