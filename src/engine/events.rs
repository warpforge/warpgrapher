//! This module provides types for event handlers. An event can be any occurrence during query
//! processing to which a client library or application might want to add business logic. Examples
//! include the before or after the creation of a new node.

use crate::engine::config::Configuration;
use crate::engine::context::{GraphQLContext, RequestContext};
use crate::engine::database::{CrudOperation, Transaction};
use crate::engine::database::{DatabaseEndpoint, DatabasePool, NodeQueryVar, SuffixGenerator};
use crate::engine::objects::resolvers::visitors::{
    visit_node_create_mutation_input, visit_node_query_input, visit_node_update_input, visit_node_delete_input
};
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::juniper::BoxFuture;
use crate::Error;
use std::collections::HashMap;
use std::convert::TryInto;

/// Type alias for a function called before the engine is built and enable modifications to
/// the configuration. A common use case is adding common properties to all types in the model.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::config::{Configuration, Property};
///
/// fn before_engine_build_func(config: &mut Configuration) -> Result<(), Error> {
///     for t in config.model.iter_mut() {
///         let mut_props: &mut Vec<Property> = t.mut_props();
///         mut_props.push(Property::new(
///             "global_prop".to_string(),
///             false,
///             "String".to_string(),
///             false,
///             false,
///             None,
///             None
///         ));
///     }
///     Ok(())
/// }
/// ```
pub type BeforeEngineBuildFunc = 
    fn(&mut Configuration) -> Result<(), Error>;

/// Type alias for a function called before request execution allowing modifications
/// to the request context. Common use case includes pulling auth tokens from the
/// metadata and inserting user information into the request context. 
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::juniper::BoxFuture;
/// # use std::collections::HashMap;
/// type Rctx = ();
///
/// fn before_request(
///     mut rctx: Rctx, 
///     mut ef: EventFacade<Rctx>,
///     metadata: HashMap<String, String>
/// ) -> BoxFuture<Result<Rctx, warpgrapher::Error>> {
///     Box::pin(async move {
///         // modify request context 
///         Ok(rctx)
///     })
/// }
/// ```
pub type BeforeRequestFunc<R> = 
    fn(R, EventFacade<R>, HashMap<String, String>) -> BoxFuture<Result<R, Error>>;

/// Type alias for a function called after request execution allowing modifications
/// to the output value.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::juniper::BoxFuture;
/// type Rctx = ();
///
/// fn after_request(
///     rctx: Rctx, 
///     output: serde_json::Value,
/// ) -> BoxFuture<'static, Result<serde_json::Value, warpgrapher::Error>> {
///     Box::pin(async move {
///         // modify output
///         Ok(output)
///     })
/// }
/// ```
pub type AfterRequestFunc<R> = 
    fn(EventFacade<R>, serde_json::Value) -> BoxFuture<Result<serde_json::Value, Error>>;
// TODO: add facade

/// Type alias for a function called before a mutation event. The Value returned by this function
/// will be used as the input to the next before event function, or to the base Warpgrapher
/// resolver if there are no more before event functions.
///
/// The structure of `Value` depends on the type of CRUD operation (which can be accessed via the `op()`
/// method on `EventFacade`.) based on the list below:
///
/// CreateNode - `Type>CreateMutationInput`
/// UpdateNode - `<Type>UpdateInput`
/// DeleteNode - `<Type>DeleteInput`
///
/// You can refer to the generated GraphQL schema documentation for the data structures.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn before_user_create(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
///     Box::pin(async move {
///         // Normally work would be done here, resulting in some new value.
///         Ok(value)
///     })
/// }
/// ```
pub type BeforeMutationEventFunc<RequestCtx> =
    fn(Value, EventFacade<RequestCtx>) -> BoxFuture<Result<Value, Error>>;

/// Type alias for a function called before an event. The Value returned by this function will be
/// used as the input to the next before event function, or to the base Warpgrapher CRUD resolver
/// if there are no more before event functions.
///
/// The structure of `Value` is `<Type>QueryInput`.
///
/// You can refer to the generated GraphQL schema documentation for the data structures.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::{BeforeQueryEventFunc, EventFacade};
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn before_user_read(value: Option<Value>, ef: EventFacade<()>) -> BoxFuture<Result<Option<Value>, Error>> {
///     Box::pin(async move {
///        // Normally work would be done here, resulting in some new value.
///        Ok(value)
///     })
/// }
///
/// let f: Box<BeforeQueryEventFunc<()>> = Box::new(before_user_read);
/// ```
pub type BeforeQueryEventFunc<RequestCtx> =
    fn(Option<Value>, EventFacade<RequestCtx>) -> BoxFuture<Result<Option<Value>, Error>>;

/// Type alias for a function called after an event affecting a node. The output of this function
/// will be used as the input to the next after event function. If there are no additional after
/// event functions, then the result of this function will be returned as the result for base
/// Warpgrapher create, read, and update operations. For delete operations, the number of nodes
/// deleted will be returned instead.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::engine::objects::Node;
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn after_user_create(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Node<()>>, Error>> {
///    Box::pin(async move {
///       // Normally work would be done here, resulting in some new value.
///       Ok(nodes)
///    })
/// }
/// ```
pub type AfterNodeEventFunc<RequestCtx> = fn(
    Vec<Node<RequestCtx>>,
    EventFacade<RequestCtx>,
) -> BoxFuture<Result<Vec<Node<RequestCtx>>, Error>>;

/// Type alias for a function called after an event affecting a relationship. The output of this
/// function will be used as the input to the next after event function. If there are no additional
/// after event functions, then the result of this function will be returned as the result for base
/// Warpgrapher create, read, and update operations. For delete operations, the number of
/// relationships deleted will be returned instead.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::engine::objects::Rel;
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn after_project_owner_create(rels: Vec<Rel<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Rel<()>>, Error>> {
///    Box::pin(async move {
///       // Normally work would be done here, resulting in some new value.
///       Ok(rels)
///    })
/// }
/// ```
pub type AfterRelEventFunc<RequestCtx> = fn(
    Vec<Rel<RequestCtx>>,
    EventFacade<RequestCtx>,
) -> BoxFuture<Result<Vec<Rel<RequestCtx>>, Error>>;

/// Collects event handlers for application during query processing.
///
/// Examples
///
/// ```rust
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
/// # use warpgrapher::juniper::BoxFuture;
///
/// fn before_user_create(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
///    Box::pin(async move {
///       // Normally work would be done here, resulting in some new value.
///       Ok(value)
///    })
/// }
///
/// let mut handlers = EventHandlerBag::<()>::new();
/// handlers.register_before_node_create(vec!["User".to_string()], before_user_create);
/// ```
#[derive(Clone)]
pub struct EventHandlerBag<RequestCtx: RequestContext> {
    before_engine_build_handlers: Vec<BeforeEngineBuildFunc>,
    before_request_handlers: Vec<BeforeRequestFunc<RequestCtx>>,
    after_request_handlers: Vec<AfterRequestFunc<RequestCtx>>,
    before_create_handlers: HashMap<String, Vec<BeforeMutationEventFunc<RequestCtx>>>,
    after_node_create_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_create_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_read_handlers: HashMap<String, Vec<BeforeQueryEventFunc<RequestCtx>>>,
    after_node_read_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_read_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_update_handlers: HashMap<String, Vec<BeforeMutationEventFunc<RequestCtx>>>,
    after_node_update_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_update_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_delete_handlers: HashMap<String, Vec<BeforeMutationEventFunc<RequestCtx>>>,
    after_node_delete_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_delete_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
}

impl<RequestCtx: RequestContext> EventHandlerBag<RequestCtx> {
    /// Creates a new collection of event handlers
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    ///
    /// let handlers = EventHandlerBag::<()>::new();
    /// ```
    pub fn new() -> EventHandlerBag<RequestCtx> {
        EventHandlerBag {
            before_engine_build_handlers: vec![],
            before_request_handlers: vec![],
            after_request_handlers: vec![],
            before_create_handlers: HashMap::new(),
            after_node_create_handlers: HashMap::new(),
            after_rel_create_handlers: HashMap::new(),
            before_read_handlers: HashMap::new(),
            after_node_read_handlers: HashMap::new(),
            after_rel_read_handlers: HashMap::new(),
            before_update_handlers: HashMap::new(),
            after_node_update_handlers: HashMap::new(),
            after_rel_update_handlers: HashMap::new(),
            before_delete_handlers: HashMap::new(),
            after_node_delete_handlers: HashMap::new(),
            after_rel_delete_handlers: HashMap::new(),
        }
    }

    /// Registers an event handler `f` to be called before the engine is built
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::engine::config::Configuration;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_engine_build(config: &mut Configuration) -> Result<(), Error> {
    ///     Ok(())
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_engine_build(before_engine_build);
    /// ```
    pub fn register_before_engine_build(
        &mut self,
        f: BeforeEngineBuildFunc
    ) {
        self.before_engine_build_handlers.push(f);
    }
    
    // TODO: docs
    pub fn register_before_request(
        &mut self,
        f: BeforeRequestFunc<RequestCtx>
    ) {
        self.before_request_handlers.push(f);
    }
    
    // TODO: docs
    pub fn register_after_request(
        &mut self,
        f: AfterRequestFunc<RequestCtx>
    ) {
        self.after_request_handlers.push(f);
    }

    /// Registers an event handler `f` to be called before a node of type `type_name` is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_create(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///    Box::pin(async move {
    ///       // Normally work would be done here, resulting in some new value.
    ///       Ok(value)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_node_create(vec!["User".to_string()], before_user_create);
    /// ```
    pub fn register_before_node_create(
        &mut self,
        type_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.before_create_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.before_create_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_project_owner_create(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///    Box::pin(async move {
    ///       // Normally work would be done here, resulting in some new value.
    ///       Ok(value)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_rel_create(vec!["ProjectOwner".to_string()],
    ///     before_project_owner_create);
    /// ```
    pub fn register_before_rel_create(
        &mut self,
        rel_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.before_create_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.before_create_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Node;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_user_create(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Node<()>>, Error>> {
    ///    Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(nodes)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_node_create(vec!["User".to_string()], after_user_create);
    /// ```
    pub fn register_after_node_create(
        &mut self,
        names: Vec<String>,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        for name in names {
            if let Some(handlers) = self.after_node_create_handlers.get_mut(&name) {
                handlers.push(f);
            } else {
                self.after_node_create_handlers.insert(name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Rel;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_project_owner_create(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   BoxFuture<Result<Vec<Rel<()>>, Error>> {
    ///    Box::pin(async move {
    ///      // Normally work would be done here, resulting in some new value.
    ///      Ok(rels)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_rel_create(vec!["ProjectOwner".to_string()], after_project_owner_create);
    /// ```
    pub fn register_after_rel_create(
        &mut self,
        rel_names: Vec<String>,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.after_rel_create_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.after_rel_create_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before nodes of type `type_name` are read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_read(value_opt: Option<Value>, ef: EventFacade<()>) -> BoxFuture<Result<Option<Value>, Error>> {
    ///    Box::pin(async move {
    ///       // Normally work would be done here, resulting in some new value.
    ///       Ok(value_opt)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_node_read(vec!["User".to_string()], before_user_read);
    /// ```
    pub fn register_before_node_read(
        &mut self,
        type_names: Vec<String>,
        f: BeforeQueryEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.before_read_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.before_read_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a rel is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_project_owner_read(value_opt: Option<Value>, ef: EventFacade<()>) -> BoxFuture<Result<Option<Value>, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(value_opt)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_rel_read(vec!["ProjectOwner".to_string()], before_project_owner_read);
    /// ```
    pub fn register_before_rel_read(
        &mut self,
        rel_names: Vec<String>,
        f: BeforeQueryEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.before_read_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.before_read_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Node;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_user_read(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Node<()>>, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(nodes)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_node_read(vec!["User".to_string()], after_user_read);
    /// ```
    pub fn register_after_node_read(
        &mut self,
        type_names: Vec<String>,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.after_node_read_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.after_node_read_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a rel is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Rel;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_project_owner_read(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   BoxFuture<Result<Vec<Rel<()>>, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(rels)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_rel_read(vec!["ProjectOwner".to_string()], after_project_owner_read);
    /// ```
    pub fn register_after_rel_read(
        &mut self,
        rel_names: Vec<String>,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.after_rel_read_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.after_rel_read_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a node of type `type_name` is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_update(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(value)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_node_update(vec!["User".to_string()], before_user_update);
    /// ```
    pub fn register_before_node_update(
        &mut self,
        type_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.before_update_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.before_update_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeQueryEventFunc;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_project_owner_update(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(value)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_rel_update(vec!["ProjectOwner".to_string()],
    ///     before_project_owner_update);
    /// ```
    pub fn register_before_rel_update(
        &mut self,
        rel_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.before_update_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.before_update_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Node;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_user_update(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Node<()>>, Error>> {
    ///    Box::pin(async move {
    ///        // Normally work would be done here, resulting in some new value.
    ///        Ok(nodes)
    ///    })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_node_update(vec!["User".to_string()], after_user_update);
    /// ```
    pub fn register_after_node_update(
        &mut self,
        type_names: Vec<String>,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.after_node_update_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.after_node_update_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a rel is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Rel;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_project_owner_update(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   BoxFuture<Result<Vec<Rel<()>>, Error>> {
    ///     Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(rels)
    ///     })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_rel_update(vec!["ProjectOwnerRel".to_string()],
    ///     after_project_owner_update);
    /// ```
    pub fn register_after_rel_update(
        &mut self,
        rel_names: Vec<String>,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.after_rel_update_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.after_rel_update_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a node of type `type_name` is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_delete(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///     Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(value)
    ///     })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_node_delete(vec!["User".to_string()], before_user_delete);
    /// ```
    pub fn register_before_node_delete(
        &mut self,
        type_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.before_delete_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.before_delete_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called before a rel is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_project_owner_delete(value: Value, ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///     Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(value)
    ///     })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_before_rel_delete(vec!["ProjectOwnerRel".to_string()],
    ///     before_project_owner_delete);
    /// ```
    pub fn register_before_rel_delete(
        &mut self,
        rel_names: Vec<String>,
        f: BeforeMutationEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.before_delete_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.before_delete_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Node;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_user_delete(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> BoxFuture<Result<Vec<Node<()>>, Error>> {
    ///     Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(nodes)
    ///     })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_node_delete(vec!["User".to_string()], after_user_delete);
    /// ```
    pub fn register_after_node_delete(
        &mut self,
        type_names: Vec<String>,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        for type_name in type_names {
            if let Some(handlers) = self.after_node_delete_handlers.get_mut(&type_name) {
                handlers.push(f);
            } else {
                self.after_node_delete_handlers.insert(type_name, vec![f]);
            }
        }
    }

    /// Registers an event handler `f` to be called after a rel is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::engine::objects::Rel;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn after_project_owner_delete(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   BoxFuture<Result<Vec<Rel<()>>, Error>> {
    ///     Box::pin(async move {
    ///         // Normally work would be done here, resulting in some new value.
    ///         Ok(rels)
    ///     })
    /// }
    ///
    /// let mut handlers = EventHandlerBag::<()>::new();
    /// handlers.register_after_rel_delete(vec!["ProjectOwnerRel".to_string()],
    ///     after_project_owner_delete);
    /// ```
    pub fn register_after_rel_delete(
        &mut self,
        rel_names: Vec<String>,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        for rel_name in rel_names {
            if let Some(handlers) = self.after_rel_delete_handlers.get_mut(&rel_name) {
                handlers.push(f);
            } else {
                self.after_rel_delete_handlers.insert(rel_name, vec![f]);
            }
        }
    }

    pub(crate) fn before_engine_build(
        &self
    ) -> &Vec<BeforeEngineBuildFunc> {
        &self.before_engine_build_handlers
    }
    
    pub(crate) fn before_request(
        &self
    ) -> &Vec<BeforeRequestFunc<RequestCtx>> {
        &self.before_request_handlers
    }
    
    pub(crate) fn after_request(
        &self
    ) -> &Vec<AfterRequestFunc<RequestCtx>> {
        &self.after_request_handlers
    }

    pub(crate) fn before_node_create(
        &self,
        type_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_create_handlers.get(type_name)
    }

    pub(crate) fn before_rel_create(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_create_handlers.get(rel_name)
    }

    pub(crate) fn after_node_create(
        &self,
        type_name: &str,
    ) -> Option<&Vec<AfterNodeEventFunc<RequestCtx>>> {
        self.after_node_create_handlers.get(type_name)
    }

    pub(crate) fn after_rel_create(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<AfterRelEventFunc<RequestCtx>>> {
        self.after_rel_create_handlers.get(rel_name)
    }

    pub(crate) fn before_node_read(
        &self,
        type_name: &str,
    ) -> Option<&Vec<BeforeQueryEventFunc<RequestCtx>>> {
        self.before_read_handlers.get(type_name)
    }

    pub(crate) fn before_rel_read(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeQueryEventFunc<RequestCtx>>> {
        self.before_read_handlers.get(rel_name)
    }

    pub(crate) fn after_node_read(
        &self,
        type_name: &str,
    ) -> Option<&Vec<AfterNodeEventFunc<RequestCtx>>> {
        self.after_node_read_handlers.get(type_name)
    }

    pub(crate) fn after_rel_read(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<AfterRelEventFunc<RequestCtx>>> {
        self.after_rel_read_handlers.get(rel_name)
    }

    pub(crate) fn before_node_update(
        &self,
        type_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_update_handlers.get(type_name)
    }

    pub(crate) fn before_rel_update(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_update_handlers.get(rel_name)
    }

    pub(crate) fn after_node_update(
        &self,
        type_name: &str,
    ) -> Option<&Vec<AfterNodeEventFunc<RequestCtx>>> {
        self.after_node_update_handlers.get(type_name)
    }

    pub(crate) fn after_rel_update(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<AfterRelEventFunc<RequestCtx>>> {
        self.after_rel_update_handlers.get(rel_name)
    }

    pub(crate) fn before_node_delete(
        &self,
        type_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_delete_handlers.get(type_name)
    }

    pub(crate) fn before_rel_delete(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc<RequestCtx>>> {
        self.before_delete_handlers.get(rel_name)
    }

    pub(crate) fn after_node_delete(
        &self,
        type_name: &str,
    ) -> Option<&Vec<AfterNodeEventFunc<RequestCtx>>> {
        self.after_node_delete_handlers.get(type_name)
    }

    pub(crate) fn after_rel_delete(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<AfterRelEventFunc<RequestCtx>>> {
        self.after_rel_delete_handlers.get(rel_name)
    }
}

impl<RequestCtx: RequestContext> Default for EventHandlerBag<RequestCtx> {
    fn default() -> EventHandlerBag<RequestCtx> {
        EventHandlerBag {
            before_engine_build_handlers: vec![],
            before_request_handlers: vec![],
            after_request_handlers: vec![],
            before_create_handlers: HashMap::new(),
            after_node_create_handlers: HashMap::new(),
            after_rel_create_handlers: HashMap::new(),
            before_read_handlers: HashMap::new(),
            after_node_read_handlers: HashMap::new(),
            after_rel_read_handlers: HashMap::new(),
            before_update_handlers: HashMap::new(),
            after_node_update_handlers: HashMap::new(),
            after_rel_update_handlers: HashMap::new(),
            before_delete_handlers: HashMap::new(),
            after_node_delete_handlers: HashMap::new(),
            after_rel_delete_handlers: HashMap::new(),
        }
    }
}

/// Provides a simplified interface to utility operations inside an event handler.
///
/// [`EventFacade`]: ./struct.EventFacade.html
pub struct EventFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    op: CrudOperation,
    context: &'a GraphQLContext<RequestCtx>,
    transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
    info: &'a Info,
}

impl<'a, RequestCtx> EventFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        op: CrudOperation,
        context: &'a GraphQLContext<RequestCtx>,
        transaction: &'a mut <<<RequestCtx as RequestContext>::DBEndpointType as DatabaseEndpoint>::PoolType as DatabasePool>::TransactionType,
        info: &'a Info,
    ) -> Self {
        Self {
            op,
            context,
            transaction,
            info,
        }
    }

    /// Returns the context of the GraphQL request which in turn contains the
    /// application-defined request context.
    pub fn op(&self) -> &CrudOperation {
        &self.op
    }

    /// Returns the context of the GraphQL request which in turn contains the
    /// application-defined request context.
    pub fn context(&self) -> &'a GraphQLContext<RequestCtx> {
        self.context
    }

    /// Provides an abstracted database read operation using warpgrapher inputs. This is the
    /// recommended way to read data in a database-agnostic way that ensures the event handlers
    /// are portable across different databases.
    ///
    /// # Arguments
    ///
    /// * `type_name` - String reference represing name of node type (ex: "User").
    /// * `input` - Optional `Value` describing which node to match. Same input structure passed to a READ crud operation (`<Type>QueryInput`).
    /// * `partition_key_opt` - Optional `Value` describing the partition key if the underlying database supports it.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::EventFacade;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_read(value: Value, mut ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///     Box::pin(async move {
    ///         let nodes_to_be_read = ef.read_nodes("User", Some(value.clone()), None).await?;
    ///         // modify value before passing it forward ...
    ///         Ok(value)
    ///     })
    /// }
    /// ```
    pub async fn read_nodes(
        &mut self,
        type_name: &str,
        input: Option<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        let mut info = self.info.clone();
        info.name = "Query".to_string();

        let mut sg = SuffixGenerator::new();
        let node_var =
            NodeQueryVar::new(Some(type_name.to_string()), "node".to_string(), sg.suffix());

        let query_fragment = visit_node_query_input::<RequestCtx>(
            &node_var,
            input,
            &Info::new(format!("{}QueryInput", type_name.to_string()), info.type_defs()),
            //&Info::new(type_name.to_string(), info.type_defs()),
            partition_key_opt,
            &mut sg,
            self.transaction,
        )
        .await?;

        let results = self
            .transaction
            .read_nodes(&node_var, query_fragment, partition_key_opt, &info)
            .await;
        results
    }

    /// Provides an abstracted database create operation using warpgrapher inputs. This is the
    /// recommended way to create nodes in a database-agnostic way that ensures the event handlers
    /// are portable across different databases.
    ///
    /// # Arguments
    ///
    /// * `type_name` - String reference represing name of node type (ex: "User").
    /// * `input` - `Value` describing the node to create.
    /// * `partition_key_opt` - Optional `Value` describing the partition key if the underlying database supports it.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::EventFacade;
    /// # use warpgrapher::engine::value::Value;
    /// # use warpgrapher::juniper::BoxFuture;
    ///
    /// fn before_user_read(value: Value, mut ef: EventFacade<()>) -> BoxFuture<Result<Value, Error>> {
    ///     Box::pin(async move {
    ///         let new_node = ef.create_node("Team", value.clone(), None).await?;
    ///         Ok(value)
    ///     })
    /// }
    /// ```
    pub async fn create_node(
        &mut self,
        type_name: &str,
        input: impl TryInto<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Node<RequestCtx>, Error> {
        let mut sg = SuffixGenerator::new();
        let node_var =
            NodeQueryVar::new(Some(type_name.to_string()), "node".to_string(), sg.suffix());
        let result = visit_node_create_mutation_input(
            &node_var,
            input.try_into().map_err(|_e| Error::TypeConversionFailed { src: "".to_string(), dst: "".to_string()})?,
            &Info::new(type_name.to_string(), self.info.type_defs()),
            partition_key_opt,
            &mut sg,
            self.transaction,
            self.context(),
        )
        .await;
        result
    }

    pub async fn update_node(
        &mut self,
        type_name: &str,
        input: impl TryInto<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        let mut sg = SuffixGenerator::new();
        let node_var =
            NodeQueryVar::new(Some(type_name.to_string()), "node".to_string(), sg.suffix());
        let result = visit_node_update_input(
            &node_var,
            input.try_into().map_err(|_e| Error::TypeConversionFailed { src: "".to_string(), dst: "".to_string()})?,
            &Info::new(format!("{}UpdateInput", type_name.to_string()), self.info.type_defs()),
            partition_key_opt,
            &mut sg,
            self.transaction,
            self.context(),
        )
        .await;
        result
    }
    
    pub async fn delete_node(
        &mut self,
        type_name: &str,
        input: impl TryInto<Value>,
        partition_key_opt: Option<&Value>,
    ) -> Result<i32, Error> {
        let mut sg = SuffixGenerator::new();
        let node_var =
            NodeQueryVar::new(Some(type_name.to_string()), "node".to_string(), sg.suffix());
        let result = visit_node_delete_input(
            &node_var,
            input.try_into().map_err(|_e| Error::TypeConversionFailed { src: "".to_string(), dst: "".to_string()})?,
            &Info::new(format!("{}DeleteInput", type_name.to_string()), self.info.type_defs()),
            partition_key_opt,
            &mut sg,
            self.transaction,
            self.context(),
        )
        .await;
        result
    }

}
