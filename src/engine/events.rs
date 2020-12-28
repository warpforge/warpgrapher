//! This module provides types for event handlers. An event can be any occurrence during query
//! processing to which a client library or application might want to add business logic. Examples
//! include the before or after the creation of a new node.

use crate::engine::context::RequestContext;
use crate::engine::objects::{Node, Rel};
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
/// # use warpgrapher::engine::events::BeforeMutationEventFunc;
/// # use warpgrapher::engine::value::Value;
///
/// fn before_user_create(value: Value) -> Result<Value, Error> {
///    // Normally work would be done here, resulting in some new value.
///    value
/// }
///
/// let f: Box<BeforeMutationEventFunc> = Box::new(before_user_create);
/// ```
pub type BeforeMutationEventFunc = fn(Value) -> Result<Value, Error>;

/// Type alias for a function called before an event. The Value returned by this function will be
/// used as the input to the next before event function, or to the base Warpgrapher CRUD resolver
/// if there are no more before event functions.
///
/// # Examples
///
/// ```rust
/// # use warpgrapher::Error;
/// # use warpgrapher::engine::events::BeforeQueryEventFunc;
/// # use warpgrapher::engine::value::Value;
///
/// fn before_user_read(value: Option<Value>) -> Result<Option<Value>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    value
/// }
///
/// let f: Box<BeforeQueryEventFunc> = Box::new(before_user_read);
/// ```
pub type BeforeQueryEventFunc = fn(Option<Value>) -> Result<Option<Value>, Error>;

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
/// # use warpgrapher::engine::events::AfterNodeEventFunc;
/// # use warpgrapher::engine::value::Value;
///
/// fn after_user_create(nodes: Vec<Node>) -> Result<Vec<Node>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    nodes
/// }
///
/// let f: Box<AfterNodeEventFunc> = Box::new(after_user_create);
/// ```
pub type AfterNodeEventFunc<RequestCtx> =
    fn(Vec<Node<RequestCtx>>) -> Result<Vec<Node<RequestCtx>>, Error>;

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
/// # use warpgrapher::engine::events::AfterRelEventFunc;
/// # use warpgrapher::engine::value::Value;
///
/// fn after_project_owner_create(rel: Vec<Rel>) -> Result<Vec<Rel>, Error> {
///    // Normally work would be done here, resulting in some new value.
///    rels
/// }
///
/// let f: Box<AfterRelEventFunc> = Box::new(after_project_owner_create);
/// ```
pub type AfterRelEventFunc<RequestCtx> =
    fn(Vec<Rel<RequestCtx>>) -> Result<Vec<Rel<RequestCtx>>, Error>;

/// Collects event handlers for application during query processing.
///
/// Examples
///
/// ```rust
/// # use warpgrapher::engine::events::{BeforeEventFunc, EventHandlerBag};
/// # use warpgrapher::engine::value::Value;
/// # use warpgrapher::Error;
///
/// fn before_user_create(value: Value) -> Result<Value, Error> {
///    // Normally work would be done here, resulting in some new value.
///    value
/// }
///
/// let mut handlers = EventHandlerBag::new();
/// handlers.register_before_node_create("User".to_string(), before_user_create);
/// ```
#[derive(Clone)]
pub struct EventHandlerBag<RequestCtx: RequestContext> {
    before_create_handlers: HashMap<String, Vec<BeforeMutationEventFunc>>,
    after_node_create_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_create_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_read_handlers: HashMap<String, Vec<BeforeQueryEventFunc>>,
    after_node_read_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_read_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_update_handlers: HashMap<String, Vec<BeforeMutationEventFunc>>,
    after_node_update_handlers: HashMap<String, Vec<AfterNodeEventFunc<RequestCtx>>>,
    after_rel_update_handlers: HashMap<String, Vec<AfterRelEventFunc<RequestCtx>>>,
    before_delete_handlers: HashMap<String, Vec<BeforeMutationEventFunc>>,
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
    /// let handlers = EventHandlerBag::new();
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
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_user_create(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_node_create("User", before_user_create);
    /// ```
    pub fn register_before_node_create(&mut self, type_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_create_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.before_create_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_project_owner_create(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_rel_create("ProjectOwner", before_project_owner_create);
    /// ```
    pub fn register_before_rel_create(&mut self, rel_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_create_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.before_create_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterNodeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_user_create(nodes: Vec<Node<RequestCtx>>) -> Result<Vec<Node<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_node_create("User", after_user_create);
    /// ```
    pub fn register_after_node_create(&mut self, name: String, f: AfterNodeEventFunc<RequestCtx>) {
        if let Some(handlers) = self.after_node_create_handlers.get_mut(&name) {
            handlers.push(f);
        } else {
            self.after_node_create_handlers.insert(name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_project_owner_create(rels: Vec<Rel<RequestCtx>>) ->
    ///   Result<Vec<Rel<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    rels
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_rel_create("ProjectOwner", after_project_owner_create);
    /// ```
    pub fn register_after_rel_create(
        &mut self,
        rel_name: String,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_rel_create_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.after_rel_create_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before nodes of type `type_name` are read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_user_read(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_node_read("User", before_user_read);
    /// ```
    pub fn register_before_node_read(&mut self, type_name: String, f: BeforeQueryEventFunc) {
        if let Some(handlers) = self.before_read_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.before_read_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a rel is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_project_owner_read(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_rel_read("ProjectOwner", before_project_owner_read);
    /// ```
    pub fn register_before_rel_read(&mut self, rel_name: String, f: BeforeQueryEventFunc) {
        if let Some(handlers) = self.before_read_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.before_read_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterNodeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_user_read(nodes: Vec<Node<RequestCtx>>) -> Result<Vec<Node<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_node_read("User", after_user_read);
    /// ```
    pub fn register_after_node_read(
        &mut self,
        type_name: String,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_node_read_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.after_node_read_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a rel is read.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_project_owner_read(rels: Vec<Rel<RequestCtx>>) ->
    ///   Result<Vec<Rel<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    rels
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_rel_read("ProjectOwner", after_project_owner_read);
    /// ```
    pub fn register_after_rel_read(&mut self, rel_name: String, f: AfterRelEventFunc<RequestCtx>) {
        if let Some(handlers) = self.after_rel_read_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.after_rel_read_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a node of type `type_name` is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_user_update(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_node_update("User", before_user_update);
    /// ```
    pub fn register_before_node_update(&mut self, type_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_update_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.before_update_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a rel is created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_project_owner_update(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_rel_update("ProjectOwner", before_project_owner_update);
    /// ```
    pub fn register_before_rel_update(&mut self, rel_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_update_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.before_update_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterNodeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_user_update(nodes: Vec<Node<RequestCtx>>) -> Result<Vec<Node<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_node_update("User", after_user_update);
    /// ```
    pub fn register_after_node_update(
        &mut self,
        type_name: String,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_node_update_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.after_node_update_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a rel is updated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_project_owner_update(rels: Vec<Rel<RequestCtx>>) ->
    ///   Result<Vec<Rel<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    rels
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_rel_update("ProjectOwner", after_project_owner_update);
    /// ```
    pub fn register_after_rel_update(
        &mut self,
        rel_name: String,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_rel_update_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.after_rel_update_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a node of type `type_name` is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_user_delete(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_node_delete("User", before_user_delete);
    /// ```
    pub fn register_before_node_delete(&mut self, type_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_delete_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.before_delete_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called before a rel is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::BeforeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn before_project_owner_delete(value: Value) -> Result<Value, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_before_rel_delete("ProjectOwner", before_project_owner_delete);
    /// ```
    pub fn register_before_rel_delete(&mut self, rel_name: String, f: BeforeMutationEventFunc) {
        if let Some(handlers) = self.before_delete_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.before_delete_handlers.insert(rel_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a node of type `type_name` is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterNodeEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_user_delete(nodes: Vec<Node<RequestCtx>>) -> Result<Vec<Node<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    value
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_node_delete("User", after_user_delete);
    /// ```
    pub fn register_after_node_delete(
        &mut self,
        type_name: String,
        f: AfterNodeEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_node_delete_handlers.get_mut(&type_name) {
            handlers.push(f);
        } else {
            self.after_node_delete_handlers.insert(type_name, vec![f]);
        }
    }

    /// Registers an event handler `f` to be called after a rel is deleted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use warpgrapher::engine::events::EventHandlerBag;
    /// # use warpgrapher::Error;
    /// # use warpgrapher::engine::events::AfterRelEventFunc;
    /// # use warpgrapher::engine::value::Value;
    ///
    /// fn after_project_owner_delete(rels: Vec<Rel<RequestCtx>>) ->
    ///   Result<Vec<Rel<RequestCtx>>, Error> {
    ///    // Normally work would be done here, resulting in some new value.
    ///    rels
    /// }
    ///
    /// let handlers = EventHandlerBag::new();
    /// handlers.register_after_rel_delete("ProjectOwner", after_project_owner_delete);
    /// ```
    pub fn register_after_rel_delete(
        &mut self,
        rel_name: String,
        f: AfterRelEventFunc<RequestCtx>,
    ) {
        if let Some(handlers) = self.after_rel_delete_handlers.get_mut(&rel_name) {
            handlers.push(f);
        } else {
            self.after_rel_delete_handlers.insert(rel_name, vec![f]);
        }
    }

    pub(crate) fn before_node_create(
        &self,
        type_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
        self.before_create_handlers.get(type_name)
    }

    pub(crate) fn before_rel_create(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
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

    pub(crate) fn before_node_read(&self, type_name: &str) -> Option<&Vec<BeforeQueryEventFunc>> {
        self.before_read_handlers.get(type_name)
    }

    pub(crate) fn before_rel_read(&self, rel_name: &str) -> Option<&Vec<BeforeQueryEventFunc>> {
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
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
        self.before_update_handlers.get(type_name)
    }

    pub(crate) fn before_rel_update(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
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
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
        self.before_delete_handlers.get(type_name)
    }

    pub(crate) fn before_rel_delete(
        &self,
        rel_name: &str,
    ) -> Option<&Vec<BeforeMutationEventFunc>> {
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
