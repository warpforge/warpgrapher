//! This module provides types for event handlers. An event can be any occurrence during query
//! processing to which a client library or application might want to add business logic. Examples
//! include the before or after the creation of a new node.

use crate::engine::context::{GraphQLContext, RequestContext};
use crate::engine::database::{CrudOperation, NodeQueryVar, SuffixGenerator, Transaction};
use crate::engine::objects::resolvers::visitors;
use crate::engine::objects::{Node, Rel};
use crate::engine::schema::Info;
use crate::engine::value::Value;
use crate::Error;
use std::collections::HashMap;

/// Type alias for a function called before a mutation event. The Value returned by this function
/// will be used as the input to the next before event function, or to the base Warpgrapher
/// resolver if there are no more before event functions.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::value::Value;
///
/// fn before_user_create(value: Value) -> Result<Value, Error> {
///    // Normally work would be done here, resulting in some new value.
///    Ok(value)
/// }
/// ```
pub type BeforeMutationEventFunc<RequestCtx> =
    fn(Value, EventFacade<RequestCtx>) -> Result<Value, Error>;

/// Type alias for a function called before an event. The Value returned by this function will be
/// used as the input to the next before event function, or to the base Warpgrapher CRUD resolver
/// if there are no more before event functions.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::{BeforeQueryEventFunc, EventFacade};
/// # use warpgrapher::engine::value::Value;
///
/// fn before_user_read(value: Option<Value>, ef: EventFacade<()>) -> Result<Option<Value>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    Ok(value)
/// }
///
/// let f: Box<BeforeQueryEventFunc<()>> = Box::new(before_user_read);
/// ```
pub type BeforeQueryEventFunc<RequestCtx> =
    fn(Option<Value>, EventFacade<RequestCtx>) -> Result<Option<Value>, Error>;

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
/// # use warpgrapher::engine::event::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::engine::objects::Node;
///
/// fn after_user_create(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> Result<Vec<Node<()>>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    Ok(nodes)
/// }
/// ```
pub type AfterNodeEventFunc<RequestCtx> =
    fn(Vec<Node<RequestCtx>>, EventFacade<RequestCtx>) -> Result<Vec<Node<RequestCtx>>, Error>;

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
/// # use warpgrapher::engine::event::EventFacade;
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::engine::objects::Rel;
///
/// fn after_project_owner_create(rels: Vec<Rel<()>>, ef: EventFacade<()>) -> Result<Vec<Rel<()>>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    Ok(rels)
/// }
/// ```
pub type AfterRelEventFunc<RequestCtx> =
    fn(Vec<Rel<RequestCtx>>, EventFacade<RequestCtx>) -> Result<Vec<Rel<RequestCtx>>, Error>;

/// Collects event handlers for application during query processing.
///
/// Examples
///
/// ```rust
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::{EventHandlerBag, EventFacde};
///
/// fn before_user_create(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
///    // Normally work would be done here, resulting in some new value.
///    Ok(value)
/// }
///
/// let mut handlers = EventHandlerBag::<()>::new();
/// handlers.register_before_node_create(vec!["User".to_string()], before_user_create);
/// ```
#[derive(Clone)]
pub struct EventHandlerBag<RequestCtx: RequestContext> {
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

    /// Registers an event handler `f` to be called before a node of type `type_name` is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::{EventHandlerBag, EventFacade};
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_user_create(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn before_project_owner_create(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn after_user_create(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> Result<Vec<Node<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(nodes)
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
    ///
    /// fn after_project_owner_create(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   Result<Vec<Rel<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(rels)
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
    ///
    /// fn before_user_read(value_opt: Option<Value>, ef: EventFacade<()>) -> Result<Option<Value>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value_opt)
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
    ///
    /// fn before_project_owner_read(value_opt: Option<Value>, ef: EventFacade<()>) -> Result<Option<Value>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value_opt)
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
    ///
    /// fn after_user_read(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> Result<Vec<Node<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(nodes)
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
    ///
    /// fn after_project_owner_read(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   Result<Vec<Rel<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(rels)
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
    ///
    /// fn before_user_update(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn before_project_owner_update(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn after_user_update(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> Result<Vec<Node<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(nodes)
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
    ///
    /// fn after_project_owner_update(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   Result<Vec<Rel<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(rels)
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
    ///
    /// fn before_user_delete(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn before_project_owner_delete(value: Value, ef: EventFacade<()>) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(value)
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
    ///
    /// fn after_user_delete(nodes: Vec<Node<()>>, ef: EventFacade<()>) -> Result<Vec<Node<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(nodes)
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
    ///
    /// fn after_project_owner_delete(rels: Vec<Rel<()>>, ef: EventFacade<()>) ->
    ///   Result<Vec<Rel<()>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    Ok(rels)
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

pub struct EventFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub crud: CrudOperation,
    pub context: &'a GraphQLContext<RequestCtx>,
    pub transaction: &'a mut dyn Transaction<RequestCtx>,
    pub info: &'a Info,
}

impl<'a, RequestCtx> EventFacade<'a, RequestCtx>
where
    RequestCtx: RequestContext,
{
    pub(crate) fn new(
        crud: CrudOperation,
        context: &'a GraphQLContext<RequestCtx>,
        transaction: &'a mut dyn Transaction<RequestCtx>,
        info: &'a Info,
    ) -> Self {
        Self {
            crud,
            context,
            transaction,
            info,
        }
    }

    /*
    pub fn context<'a>(&self) -> &'a GraphQLContext<RequestCtx> {
        self.context
    }

    pub fn transaction<'a>(&self) -> &'a mut dyn Transaction<RequestCtx> {
        self.transaction
    }
    */

    pub fn read_nodes(
        &mut self,
        type_name: &str,
        input: Option<Value>,
    ) -> Result<Vec<Node<RequestCtx>>, Error> {
        let mut info = self.info.clone();
        info.name = "Query".to_string();
        let partition_key_opt = None;
        let mut sg = SuffixGenerator::new();
        let p = info.type_def()?.property(type_name)?;
        let itd = if info.name() == "Query" {
            p.input_type_definition(&info)?
        } else {
            info.type_def_by_name("Query")?
                .property(p.type_name())?
                .input_type_definition(&info)?
        };
        let node_var = NodeQueryVar::new(
            Some(p.type_name().to_string()),
            "node".to_string(),
            sg.suffix(),
        );
        let query_fragment = visitors::visit_node_query_input(
            &node_var,
            input,
            &Info::new(itd.type_name().to_owned(), info.type_defs()),
            partition_key_opt,
            &mut sg,
            self.transaction,
        )?;
        let results =
            self.transaction
                .read_nodes(&node_var, query_fragment, partition_key_opt, &info);
        results
    }
}
