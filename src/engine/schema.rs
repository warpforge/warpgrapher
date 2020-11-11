//! This module provides the GraphQL service implementation. It generates
//! the Juniper root node, and its sub-modules provide schema data structures
//! and resolvers for common create, read, update, and delete operations.

use super::config::{
    Configuration, Endpoint, EndpointClass, GraphqlType, Relationship, Type, TypeDef,
};
use super::objects::Node;
use crate::engine::context::RequestContext;
use crate::error::Error;
use inflector::Inflector;
use juniper::RootNode;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Values;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::panic::catch_unwind;
use std::slice::Iter;
use std::sync::Arc;

/// Carries the type information in the GraphQL schema, derived from the [`Configuration`] used to
/// set up the Warpgrapher [`Engine`]. Used by Warpgrapher in auto-generated resolvers for CRUD
/// operations.
///
/// [`Configuration`]: ../config/struct.Configuration.html
/// [`Engine`]: ../struct.Engine.html
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    name: String,
    type_defs: Arc<HashMap<String, NodeType>>,
}

impl Info {
    pub(crate) fn new(name: String, type_defs: Arc<HashMap<String, NodeType>>) -> Info {
        Info { name, type_defs }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn type_def(&self) -> Result<&NodeType, Error> {
        self.type_def_by_name(&self.name)
    }

    pub(crate) fn type_def_by_name(&self, name: &str) -> Result<&NodeType, Error> {
        self.type_defs
            .get(name)
            .ok_or_else(|| Error::SchemaItemNotFound {
                name: self.name.to_string(),
            })
    }

    pub(crate) fn type_defs(&self) -> Arc<HashMap<String, NodeType>> {
        self.type_defs.clone()
    }
}

pub(super) type RootRef<RequestCtx> = Arc<RootNode<'static, Node<RequestCtx>, Node<RequestCtx>>>;

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum ArgumentKind {
    Required,
    Optional,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum PropertyKind {
    CustomResolver,
    DynamicScalar,
    DynamicRel { rel_name: String },
    Input,
    NodeCreateMutation,
    NodeUpdateMutation,
    NodeDeleteMutation { label: String },
    Object,
    Rel { rel_name: String },
    RelCreateMutation { src_label: String, rel_name: String },
    RelUpdateMutation { src_label: String, rel_name: String },
    RelDeleteMutation { src_label: String, rel_name: String },
    Scalar,
    Union,
    VersionQuery,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum TypeKind {
    Input,
    Object,
    Rel,
    Union,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct NodeType {
    props: HashMap<String, Property>,
    type_kind: TypeKind,
    type_name: String,
    union_types: Option<Vec<String>>,
}

impl NodeType {
    fn new(type_name: String, type_kind: TypeKind, props: HashMap<String, Property>) -> NodeType {
        NodeType {
            props,
            type_kind,
            type_name,
            union_types: None,
        }
    }

    pub(crate) fn property(&self, property_name: &str) -> Result<&Property, Error> {
        self.props
            .get(property_name)
            .ok_or_else(|| Error::SchemaItemNotFound {
                name: self.type_name.to_string() + "::" + property_name,
            })
    }

    pub(crate) fn props(&self) -> Values<String, Property> {
        self.props.values()
    }

    pub(crate) fn type_kind(&self) -> &TypeKind {
        &self.type_kind
    }

    pub(crate) fn type_name(&self) -> &str {
        &self.type_name
    }

    pub(crate) fn union_types(&self) -> Option<Iter<String>> {
        self.union_types.as_ref().map(|uts| uts.iter())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Property {
    name: String,
    kind: PropertyKind,
    type_name: String,
    required: bool,
    list: bool,
    arguments: HashMap<String, Argument>,
    resolver: Option<String>,
    validator: Option<String>,
}

impl Property {
    fn new(name: String, kind: PropertyKind, type_name: String) -> Property {
        Property {
            name,
            kind,
            type_name,
            required: false,
            list: false,
            arguments: HashMap::new(),
            resolver: None,
            validator: None,
        }
    }

    pub(crate) fn arguments(&self) -> Values<String, Argument> {
        self.arguments.values()
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn input_type_definition<'i>(&self, info: &'i Info) -> Result<&'i NodeType, Error> {
        self.arguments
            .get("input")
            .ok_or_else(|| Error::SchemaItemNotFound {
                name: "input".to_string(),
            })
            .and_then(|input_arg| {
                info.type_defs
                    .get(&input_arg.type_name)
                    .ok_or_else(|| Error::SchemaItemNotFound {
                        name: input_arg.type_name.to_string(),
                    })
            })
    }

    pub(crate) fn kind(&self) -> &PropertyKind {
        &self.kind
    }

    pub(crate) fn list(&self) -> bool {
        self.list
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn resolver(&self) -> Option<&String> {
        self.resolver.as_ref()
    }

    pub(crate) fn required(&self) -> bool {
        self.required
    }

    pub(crate) fn type_name(&self) -> &str {
        &self.type_name
    }

    #[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
    pub(crate) fn validator(&self) -> Option<&String> {
        self.validator.as_ref()
    }

    fn with_arguments(mut self, arguments: HashMap<String, Argument>) -> Self {
        self.arguments = arguments;
        self
    }

    fn with_list(mut self, list: bool) -> Self {
        self.list = list;
        self
    }

    fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    fn with_resolver(mut self, resolver: &str) -> Self {
        self.resolver = Some(resolver.to_string());
        self
    }

    fn with_validator(mut self, validator: Option<String>) -> Self {
        self.validator = validator;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Argument {
    name: String,
    kind: ArgumentKind,
    type_name: String,
}

impl Argument {
    fn new(name: String, kind: ArgumentKind, type_name: String) -> Argument {
        Argument {
            name,
            kind,
            type_name,
        }
    }

    pub(crate) fn kind(&self) -> &ArgumentKind {
        &self.kind
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn type_name(&self) -> &str {
        &self.type_name
    }
}

/// Takes a vector of WG Properties and returns a map of Property structs that
/// represent the property fields in a graphql schema component
fn generate_props(
    props: &[crate::engine::config::Property],
    id: bool,
    object: bool,
) -> HashMap<String, Property> {
    let mut hm = HashMap::new();

    // if the ID field was specified, add it
    if id {
        hm.insert(
            "id".to_string(),
            Property::new("id".to_string(), PropertyKind::Scalar, "ID".to_string())
                .with_required(object),
        );
    }

    // insert properties into hashmap
    props.iter().for_each(|p| {
        match &p.resolver() {
            None => {
                hm.insert(
                    p.name().to_string(),
                    Property::new(
                        p.name().to_string(),
                        PropertyKind::Scalar,
                        p.type_name().to_string(),
                    )
                    .with_required(p.required() && object)
                    .with_list(p.list())
                    .with_validator(p.validator().cloned()),
                );
            }
            Some(r) => {
                hm.insert(
                    p.name().to_string(),
                    Property::new(
                        p.name().to_string(),
                        PropertyKind::DynamicScalar,
                        p.type_name().to_string(),
                    )
                    .with_required(p.required() && object)
                    .with_list(p.list())
                    .with_resolver(r)
                    .with_validator(p.validator().cloned()),
                );
            }
        };
    });

    hm
}

/// Takes a WG type and returns the name of the corresponding GqlNodeObject.
/// In reality all this is doing is returning the name, but it add value by
/// maintaining consistency with using functions that returned formatted names
/// instead of doing inline string concat
fn fmt_node_object_name(t: &Type) -> String {
    t.name().to_string()
}

/// Takes a WG type and returns a NodeType representing a GqlNodeObject
///
/// Format:
/// type GqlNodeObject {
///    id: ID
///    prop[n]: <Scalar>
///    rel[n]: <GqlRelNodesUnion>
/// }
///
/// Ex:
/// type Project {
///     id: ID
///     name: String
///     owner: ProjectOwnerRel
/// }
fn generate_node_object(t: &Type) -> NodeType {
    let mut props = generate_props(&t.props_as_slice(), true, true);

    t.rels().for_each(|r| {
        let mut arguments = HashMap::new();
        arguments.insert(
            "input".to_string(),
            Argument::new(
                "input".to_string(),
                ArgumentKind::Optional,
                fmt_rel_query_input_name(t, r),
            ),
        );

        let mut p = Property::new(
            r.name().to_string(),
            match r.resolver() {
                None => PropertyKind::Rel {
                    rel_name: r.name().to_string(),
                },
                Some(_) => PropertyKind::DynamicRel {
                    rel_name: r.name().to_string(),
                },
            },
            fmt_rel_object_name(t, &r),
        )
        .with_list(r.list())
        .with_arguments(arguments);

        if let Some(resolver) = r.resolver() {
            p = p.with_resolver(resolver);
        }

        props.insert(r.name().to_string(), p);
    });

    NodeType::new(t.name().to_string(), TypeKind::Object, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeQueryInput
fn fmt_node_query_input_name(t: &Type) -> String {
    t.name().to_string() + "QueryInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeQueryInput
///
/// Format:
/// input GqlNodeQueryInput {
///     id: <ID>
///     prop[n]: <Scalar>
///     rel[n]:  <GqlRelQueryInput>
/// }
///
/// Ex:
/// input ProjectQueryInput {
///     id: ID
///     name: String
///     owner: ProjectOwnerQueryInput
/// }
fn generate_node_query_input(t: &Type) -> NodeType {
    let mut props = generate_props(&t.props_as_slice(), true, false);

    t.rels().for_each(|r| {
        props.insert(
            r.name().to_string(),
            Property::new(
                r.name().to_string(),
                PropertyKind::Input,
                fmt_rel_query_input_name(t, &r),
            )
            .with_list(r.list()),
        );
    });

    NodeType::new(fmt_node_query_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateMutationInput
fn fmt_node_create_mutation_input_name(t: &Type) -> String {
    t.name().to_string() + "CreateMutationInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeCreateMutationInput
///
/// Format:
/// input GqlNodeCreateMutationInput {
///     prop[n]: <Scalar>
///     rel[n]:  <GqlRelCreateMutationInput>
/// }
///
/// Ex:
/// input ProjectMutationInput {
///     name: String
///     owner: ProjectOwnerMutationInput
/// }
fn generate_node_create_mutation_input(t: &Type) -> NodeType {
    let mut props = generate_props(t.props_as_slice(), false, false);

    t.rels().for_each(|r| {
        props.insert(
            r.name().to_string(),
            Property::new(
                r.name().to_string(),
                PropertyKind::Input,
                fmt_rel_create_mutation_input_name(t, &r),
            )
            .with_list(r.list()),
        );
    });

    NodeType::new(
        fmt_node_create_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateMutationInput
fn fmt_node_update_mutation_input_name(t: &Type) -> String {
    t.name().to_string() + "UpdateMutationInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeUpdateMutationInput
///
/// Format:
/// input GqlNodeUpdateMutationInput {
///    field[n]: Scalar
///    rel[n]: GqlRelChangeInput
/// }
///
/// Ex:
/// input ProjectUpdateMutationInput {
///     since: String
///     owner: ProjectOwnerChangeInput
///     issues: ProjectIssuesChangeInput
/// }
fn generate_node_update_mutation_input(t: &Type) -> NodeType {
    let mut props = generate_props(t.props_as_slice(), false, false);

    t.rels().for_each(|r| {
        props.insert(
            r.name().to_string(),
            Property::new(
                r.name().to_string(),
                PropertyKind::Input,
                fmt_rel_change_input_name(t, &r),
            )
            .with_list(r.list()),
        );
    });

    NodeType::new(
        fmt_node_update_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeInput
fn fmt_node_input_name(t: &Type) -> String {
    t.name().to_string() + "Input"
}

/// Takes a WG type and returns the name of the corresponding GqlNodeInput
///
/// Format:
/// input GqlNodeInput {
///    $EXISTING: GqlNodeQueryInput
///    $NEW: GqlNodeCreateMutationInput
/// }
///
/// Ex:
/// input ProjectInput {
///     $EXISTING: ProjectQueryInput
///     $NEW: ProjectMutationInput
/// }
fn generate_node_input(t: &Type) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$EXISTING".to_string(),
        Property::new(
            "$EXISTING".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
        ),
    );
    props.insert(
        "$NEW".to_string(),
        Property::new(
            "$NEW".to_string(),
            PropertyKind::Input,
            fmt_node_create_mutation_input_name(t),
        ),
    );
    NodeType::new(fmt_node_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeUpdateInput
fn fmt_node_update_input_name(t: &Type) -> String {
    t.name().to_string() + "UpdateInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeUpdateInput
///
/// Format:
/// input GqlNodeUpdateInput {
///     $MATCH: GqlNodeQueryInput
///     $SET: GqlNodeCreateMutationInput
/// }
///
/// Ex:
/// input ProjectUpdateInput {
///     $MATCH: ProjectQueryInput
///     $SET: ProjectMutationInput
/// }
fn generate_node_update_input(t: &Type) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$MATCH".to_string(),
        Property::new(
            "$MATCH".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
        ),
    );
    props.insert(
        "$SET".to_string(),
        Property::new(
            "$SET".to_string(),
            PropertyKind::Input,
            fmt_node_update_mutation_input_name(t),
        ),
    );
    NodeType::new(fmt_node_update_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_node_delete_input_name(t: &Type) -> String {
    t.name().to_string() + "DeleteInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteInput
///
/// Format:
/// input GqlNodeDeleteInput {
///     $MATCH: GqlNodeQueryInput
///     delete: GqlNodeDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectDeleteInput {
///     $MATCH: ProjectQueryInput
///     delete: ProjectDeleteMutationInput
/// }
fn generate_node_delete_input(t: &Type) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$MATCH".to_string(),
        Property::new(
            "$MATCH".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
        ),
    );
    props.insert(
        "$DELETE".to_string(),
        Property::new(
            "$DELETE".to_string(),
            PropertyKind::Input,
            fmt_node_delete_mutation_input_name(t),
        ),
    );
    NodeType::new(fmt_node_delete_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteMutationInput
fn fmt_node_delete_mutation_input_name(t: &Type) -> String {
    t.name().to_string() + "DeleteMutationInput"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteMutationInput
///
/// Format:
/// input GqlNodeDeleteMutationInput {
///     rel[n]: GqlRelDeleteInput
/// }
///
/// Ex:
/// input ProjectDeleteMutationInput {
///     owner: ProjectOwnerDeleteInput
///     issues: ProjectIssuesDeleteInput
/// }
fn generate_node_delete_mutation_input(t: &Type) -> NodeType {
    let mut props = HashMap::new();
    t.rels().for_each(|r| {
        props.insert(
            r.name().to_string(),
            Property::new(
                r.name().to_string(),
                PropertyKind::Input,
                fmt_rel_delete_input_name(t, &r),
            )
            .with_list(r.list()),
        );
    });
    NodeType::new(
        fmt_node_delete_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeReadEndpoint
fn fmt_node_read_endpoint_name(t: &Type) -> String {
    t.name().to_string()
}

/// Takes a WG type and returns a NodeType representing a GqlNodeReadEndpoint
///
/// Format:
/// GqlNodeReadEndpoint(input: <GqlNodeQueryInput>): [<Node>]
///
/// Ex:
/// Project(input: ProjectQueryInput): [Project]
fn generate_node_read_endpoint(t: &Type) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Optional,
            fmt_node_query_input_name(t),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_node_read_endpoint_name(t),
        PropertyKind::Object,
        t.name().to_string(),
    )
    .with_list(true)
    .with_arguments(arguments)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateEndpoint
fn fmt_node_create_endpoint_name(t: &Type) -> String {
    t.name().to_string() + "Create"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeCreateEndpoint
///
/// Format:
/// GqlNodeCreateEndpoint (input: <GqlNodeCreateMutationInput>): <Node>
///
/// Ex:
/// ProjectCreate (input: ProjectCreateMutationInput): Project
fn generate_node_create_endpoint(t: &Type) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_node_create_mutation_input_name(t),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_node_create_endpoint_name(t),
        PropertyKind::NodeCreateMutation,
        t.name().to_string(),
    )
    .with_arguments(arguments)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateEndpoint
fn fmt_node_update_endpoint_name(t: &Type) -> String {
    t.name().to_string() + "Update"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeUpdateEndpoint:
///
/// Format:
/// GqlNodeUpdateEndpoint (input: <GqlNodeUpdateInput>): [<Node>]
///
/// Ex:
/// ProjectUpdate (input: ProjectUpdateInput): [Project]
fn generate_node_update_endpoint(t: &Type) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_node_update_input_name(t),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_node_update_endpoint_name(t),
        PropertyKind::NodeUpdateMutation,
        t.name().to_string(),
    )
    .with_list(true)
    .with_arguments(arguments)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteEndpoint
fn fmt_node_delete_endpoint_name(t: &Type) -> String {
    t.name().to_string() + "Delete"
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteEndpoint
///
/// Format:
/// GqlNodeDeleteEndpoint (input: <GqlNodeQueryInput>): Int
///
/// Ex:
/// ProjectDelete (input: <ProjectQueryInput>): Int
fn generate_node_delete_endpoint(t: &Type) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_node_delete_input_name(t),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_node_delete_endpoint_name(t),
        PropertyKind::NodeDeleteMutation {
            label: fmt_node_object_name(t),
        },
        "Int".to_string(),
    )
    .with_arguments(arguments)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelObject
fn fmt_rel_object_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "Rel"
}

/// Takes a WG rel an returns the name of the rel. In reality, this just makes
/// a copy of the name
fn fmt_rel_name(r: &Relationship) -> String {
    r.name().to_string()
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelObject
///
/// Format:
/// type GqlRelObject {
///     id: ID
///     props: <GqlRelPropsObject>
///     dst: <GqlRelNodesUnion>
///     src: <GqlNodeObject>
/// }
///
/// Ex:
/// type ProjectOwnerRel {
///     id: ID
///     props: ProjectOwnerProps
///     dst: ProjectOwnerNodesUnion
///     src: Project
/// }
fn generate_rel_object(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "id".to_string(),
        Property::new("id".to_string(), PropertyKind::Scalar, "ID".to_string()).with_required(true),
    );

    if !r.props_as_slice().is_empty() {
        props.insert(
            "props".to_string(),
            Property::new(
                "props".to_string(),
                PropertyKind::Object,
                fmt_rel_props_object_name(t, r),
            ),
        );
    }
    props.insert(
        "src".to_string(),
        Property::new(
            "src".to_string(),
            PropertyKind::Object,
            t.name().to_string(),
        )
        .with_required(true),
    );
    props.insert(
        "dst".to_string(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Union,
            fmt_rel_nodes_union_name(t, r),
        )
        .with_required(true),
    );
    NodeType::new(fmt_rel_object_name(t, r), TypeKind::Rel, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelPropsObject
fn fmt_rel_props_object_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "Props"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelPropsObject
///
/// Format:
/// type GqlRelPropsObject {
///     prop[n]: <Scalar>
/// }
///
/// Ex:
/// type ProjectOwnerProps {
///     since: String
/// }
fn generate_rel_props_object(t: &Type, r: &Relationship) -> NodeType {
    NodeType::new(
        fmt_rel_props_object_name(t, r),
        TypeKind::Object,
        generate_props(r.props_as_slice(), false, true),
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelNodesUnion
fn fmt_rel_nodes_union_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "NodesUnion"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelNodesUnion
///
/// Format:
/// union GqlRelNodesUnion = <Node[0]> | <Node[1]>
///
/// Ex:
/// union ProjectIssuesNodesUnion = Feature | Bug
fn generate_rel_nodes_union(t: &Type, r: &Relationship) -> NodeType {
    let mut nt = NodeType::new(
        fmt_rel_nodes_union_name(t, r),
        TypeKind::Union,
        HashMap::new(),
    );
    nt.union_types = Some(r.nodes().cloned().collect());
    nt
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelQueryInput
fn fmt_rel_query_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "QueryInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelQueryInput
///
/// Format:
/// input GqlRelQueryInput {
///      id: ID
///      props: <GqlRelPropsInput>
///      src: <GqlNodeQueryInput>
///      dst: <GqlRelDstQueryInput>
/// }
///
/// Ex:
/// input ProjectOwnerQueryInput {
///     id: ID
///     props: ProjectOwnerPropsInput
///     src: ProjectQueryInput
///     dst: ProjectOwnerNodesQueryInputUnion  
/// }
fn generate_rel_query_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "id".to_string(),
        Property::new("id".to_string(), PropertyKind::Scalar, "ID".to_string()),
    );
    if !r.props_as_slice().is_empty() {
        props.insert(
            "props".to_string(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
            ),
        );
    }
    props.insert(
        "src".to_string(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_query_input_name(t, r),
        ),
    );
    props.insert(
        "dst".to_string(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_query_input_name(t, r),
        ),
    );
    NodeType::new(fmt_rel_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateMutationInput
fn fmt_rel_create_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "CreateMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelCreateMutationInput
///
/// Format:
/// input GqlRelCreateMutationInput {
///     props: <GqlRelPropsInput>
///     dst: <GqlRelNodesMutationInputUnion>
/// }
///
/// Ex:
/// input ProjectOwnerCreateMutationInput  {
///     id: ID
///     props: ProjectOwnerPropsInput
///     dst: ProjectOwnerNodesMutationInputUnion  
/// }
fn generate_rel_create_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    if !r.props_as_slice().is_empty() {
        props.insert(
            "props".to_string(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
            ),
        );
    }
    props.insert(
        "dst".to_string(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_nodes_mutation_input_union_name(t, r),
        )
        .with_required(true),
    );
    NodeType::new(
        fmt_rel_create_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateMutationInput
fn fmt_rel_change_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "ChangeInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelChangeInput
///
/// Format:
/// input GqlRelChangeInput {
///     $ADD: GqlRelCreateMutationInput
///     $UPDATE: GqlRelUpdateMutationInput
///     $DELETE: GqlRelDeleteInput
/// }
///
/// Ex:
/// input ProjectIssuesChangeInput {
///     $ADD: ProjectIssuesCreateMutationInput
///     $UPDATE: ProjectIssuesUpdateInput
///     $DELETE: ProjectIssuesDeleteInput
/// }
fn generate_rel_change_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$ADD".to_string(),
        Property::new(
            "$ADD".to_string(),
            PropertyKind::Input,
            fmt_rel_create_mutation_input_name(t, r),
        ),
    );
    props.insert(
        "$UPDATE".to_string(),
        Property::new(
            "$UPDATE".to_string(),
            PropertyKind::Input,
            fmt_rel_update_input_name(t, r),
        ),
    );
    props.insert(
        "$DELETE".to_string(),
        Property::new(
            "$DELETE".to_string(),
            PropertyKind::Input,
            fmt_rel_delete_input_name(t, r),
        ),
    );
    NodeType::new(fmt_rel_change_input_name(t, r), TypeKind::Input, props)
}
/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateMutationInput
fn fmt_rel_update_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "UpdateMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelUpdateMutationInput
///
/// Format:
/// input GqlRelUpdateMutationInput {
///     props: GqlRelPropsInput
///     src: GqlRelSrcMutationInput
///     dst: GqlRelDstMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerUpdateMutationInput {
///     props: ProjectOwnerPropsInput
///     src: ProjectOwnerSrcUpdateMutationInput
///     dst: ProjectOwnerDstUpdateMutationInput
/// }
fn generate_rel_update_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    if !r.props_as_slice().is_empty() {
        props.insert(
            "props".to_string(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
            ),
        );
    }
    props.insert(
        "src".to_string(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_update_mutation_input_name(t, r),
        ),
    );
    props.insert(
        "dst".to_string(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_update_mutation_input_name(t, r),
        ),
    );
    NodeType::new(
        fmt_rel_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelSrcUpdateMutationInput
fn fmt_rel_src_update_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "SrcUpdateMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelSrcUpdateMutationInput
///
/// Format:
/// input GqlRelSrcUpdateMutationInput {
///     node[n]: GqlNodeUpdateMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerSrcUpdateMutationInput {
///     Project: ProjectUpdateMutationInput
/// }
fn generate_rel_src_update_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name().to_string(),
        Property::new(
            t.name().to_string(),
            PropertyKind::Input,
            fmt_node_update_mutation_input_name(t),
        ),
    );
    NodeType::new(
        fmt_rel_src_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDstUpdateMutationInput
fn fmt_rel_dst_update_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "DstUpdateMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDstUpdateMutationInput
///
/// Format:
/// input GqlRelDstUpdateMutationInput {
///     node[n]: GqlNodeUpdateMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerDstUpdateMutationInput {
///     User: UserUpdateMutationInput
/// }
fn generate_rel_dst_update_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    r.nodes().for_each(|node| {
        props.insert(
            node.to_string(),
            Property::new(
                node.to_string(),
                PropertyKind::Input,
                node.to_string() + "UpdateMutationInput",
            ),
        );
    });
    NodeType::new(
        fmt_rel_dst_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}
/// Takes a WG type and rel and returns the name of the corresponding GqlRelPropsInput
fn fmt_rel_props_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "PropsInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelPropsInput
///
/// Format:
/// input GqlRelPropsInput {
///     prop[n]: <Scalar>
/// }
///
/// Ex:
/// input ProjectOwnerPropsInput   {
///     since: String
/// }
fn generate_rel_props_input(t: &Type, r: &Relationship) -> NodeType {
    NodeType::new(
        fmt_rel_props_input_name(t, r),
        TypeKind::Input,
        generate_props(r.props_as_slice(), false, false),
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelSrcQueryInput
fn fmt_rel_src_query_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "SrcQueryInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelSrcQueryInput
///
/// Format:
/// input GqlRelSrcQueryInput {
///     Node[n]: GqlNodeQueryInput
/// }
///  
/// Ex:
/// input ProjectOwnerSrcQueryInput  {
///     Project: ProjectQueryInput
/// }
fn generate_rel_src_query_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name().to_string(),
        Property::new(
            t.name().to_string(),
            PropertyKind::Input,
            t.name().to_string() + "QueryInput",
        ),
    );
    NodeType::new(fmt_rel_src_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDstQueryInput
fn fmt_rel_dst_query_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "DstQueryInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDstQueryInput
///
/// Format:
/// input GqlRelDstQueryInput {
///     Node[n]: GqlNodeQueryInput
/// }
///  
/// Ex:
/// input ProjectOwnerDstQueryInput  {
///     User: UserQueryInput
/// }
fn generate_rel_dst_query_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    r.nodes().for_each(|node| {
        props.insert(
            node.to_string(),
            Property::new(
                node.to_string(),
                PropertyKind::Input,
                //fmt_node_query_input_name(t, r),
                node.to_string() + "QueryInput",
            ),
        );
    });
    NodeType::new(fmt_rel_dst_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelNodesMutationInputUnion
fn fmt_rel_nodes_mutation_input_union_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "NodesMutationInputUnion"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelNodesMutationInput
///
/// Format:
/// input GqlRelNodesMutationInputUnion {
///     <Node[n]>: <GqlNodeInput>
/// }
///
/// Ex:
/// input ProjectOwnerNodesMutationInputUnion  {
///     User: UserInput
/// }
fn generate_rel_nodes_mutation_input_union(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    r.nodes().for_each(|node| {
        props.insert(
            node.to_string(),
            Property::new(
                node.to_string(),
                PropertyKind::Input,
                node.to_string() + "Input",
            ),
        );
    });
    NodeType::new(
        fmt_rel_nodes_mutation_input_union_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateInput
fn fmt_rel_create_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "CreateInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelCreateInput
///
/// Format:
/// input GqlRelCreateInput {
///     $MATCH: <GqlNodeQueryInput>
///     create: <GqlRelCreateMutationInput>
/// }
///
/// Ex:
/// input ProjectOwnerCreateInput   {
///     $MATCH: ProjectQueryInput
///     $CREATE: ProjectOwnerCreateMutationInput
/// }
fn generate_rel_create_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$MATCH".to_string(),
        Property::new(
            "$MATCH".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
        ),
    );
    props.insert(
        "$CREATE".to_string(),
        Property::new(
            "$CREATE".to_string(),
            PropertyKind::Input,
            fmt_rel_create_mutation_input_name(t, &r),
        )
        .with_list(r.list()),
    );
    NodeType::new(fmt_rel_create_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateInput
fn fmt_rel_update_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "UpdateInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelUpdateInput
///
/// Format:
/// input GqlRelUpdateInput {
///     $MATCH: GqlRelQueryInput
///     $SET: GqlRelUpdateMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerUpdateInput   {
///     $MATCH: ProjectOwnerQueryInput
///     $SET: ProjectOwnerUpdateMutationInput
/// }
fn generate_rel_update_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$MATCH".to_string(),
        Property::new(
            "$MATCH".to_string(),
            PropertyKind::Input,
            fmt_rel_query_input_name(t, r),
        ),
    );
    props.insert(
        "$SET".to_string(),
        Property::new(
            "$SET".to_string(),
            PropertyKind::Input,
            fmt_rel_update_mutation_input_name(t, &r),
        )
        .with_required(true),
    );
    NodeType::new(fmt_rel_update_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_rel_delete_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "DeleteInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDeleteInput
///
/// Format:
/// input GqlRelDeleteInput {
///    $MATCH: GqlRelQueryInput
///    src: GqlRelSrcDeleteMutationInput
///    dst: GqlRelDstDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerDeleteInput {
///    $MATCH: ProjectOwnerQueryInput
///    src: ProjectOwnerSrcDeleteMutationInput
///    dst: ProjectOwnerDstDeleteMutationInput
/// }
fn generate_rel_delete_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "$MATCH".to_string(),
        Property::new(
            "$MATCH".to_string(),
            PropertyKind::Input,
            fmt_rel_query_input_name(t, r),
        ),
    );
    props.insert(
        "src".to_string(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_delete_mutation_input_name(t, r),
        ),
    );
    props.insert(
        "dst".to_string(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_delete_mutation_input_name(t, r),
        ),
    );
    NodeType::new(fmt_rel_delete_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlRelSrcDeleteMutationInput
fn fmt_rel_src_delete_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "SrcDeleteMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelSrcDeleteMutationInput
///
/// Format:
/// input GqlRelSrcDeleteMutationInput {
///    GqlNodeObject[n]: GqlNodeDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerSrcDeleteMutationInput {
///    Project: ProjectDeleteMutationInput
/// }
fn generate_rel_src_delete_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name().to_string(),
        Property::new(
            t.name().to_string(),
            PropertyKind::Input,
            fmt_node_delete_mutation_input_name(t),
        ),
    );
    NodeType::new(
        fmt_rel_src_delete_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_rel_dst_delete_mutation_input_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "DstDeleteMutationInput"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDstDeleteMutationInput
///
/// Format:
/// input GqlRelDstDeleteMutationInput {
///     node[n]: GqlNodeDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerDstDeleteMutationInput {
///     User: UserDeleteMutationInput
/// }
fn generate_rel_dst_delete_mutation_input(t: &Type, r: &Relationship) -> NodeType {
    let mut props = HashMap::new();
    r.nodes().for_each(|node| {
        props.insert(
            node.to_string(),
            Property::new(
                node.to_string(),
                PropertyKind::Input,
                node.to_string() + "DeleteMutationInput",
            ),
        );
    });
    NodeType::new(
        fmt_rel_dst_delete_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelReadEndpoint
fn fmt_rel_read_endpoint_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelReadEndpoint
///
/// Format:
/// GqlRelReadEndpoint (input: <GqlRelQueryInput>): [<GqlRelObject>]
///
/// Ex:
/// ProjectOwner(input: ProjectOwnerQueryInput): [ProjectOwnerRel]
fn generate_rel_read_endpoint(t: &Type, r: &Relationship) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Optional,
            fmt_rel_query_input_name(t, r),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_rel_read_endpoint_name(t, r),
        PropertyKind::Rel {
            rel_name: r.name().to_string(),
        },
        fmt_rel_object_name(t, r),
    )
    .with_list(true)
    .with_arguments(arguments)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateEndpoint
fn fmt_rel_create_endpoint_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "Create"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelCreateEndpoint
///
/// Format:
/// GqlRelCreateEndpoint (input: <GqlRelCreateInput>): <GqlRelObject>
///
/// Ex:
/// ProjectOwnerCreate(input: ProjectOwnerCreateInput): ProjectOwnerRel
fn generate_rel_create_endpoint(t: &Type, r: &Relationship) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_rel_create_input_name(t, r),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_rel_create_endpoint_name(t, r),
        PropertyKind::RelCreateMutation {
            src_label: fmt_node_object_name(t),
            rel_name: fmt_rel_name(r),
        },
        fmt_rel_object_name(t, r),
    )
    .with_list(true)
    .with_arguments(arguments)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateEndpoint
fn fmt_rel_update_endpoint_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "Update"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelUpdateEndpoint
///
/// Format:
/// GqlRelUpdateEndpoint (input: <GqlRelUpdateInput>): [<GqlRelObject>]
///
/// Ex:
/// ProjectOwnerUpdate(input: ProjectOwnerUpdateInput): ProjectOwnerRel
fn generate_rel_update_endpoint(t: &Type, r: &Relationship) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_rel_update_input_name(t, r),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_rel_update_endpoint_name(t, r),
        PropertyKind::RelUpdateMutation {
            src_label: fmt_node_object_name(t),
            rel_name: fmt_rel_name(r),
        },
        fmt_rel_object_name(t, r),
    )
    .with_list(true)
    .with_arguments(arguments)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDeleteEndpoint
fn fmt_rel_delete_endpoint_name(t: &Type, r: &Relationship) -> String {
    t.name().to_string()
        + &((&r.name().to_string().to_title_case())
            .split_whitespace()
            .collect::<String>())
        + "Delete"
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDeleteEndpoint
///
/// Format:
/// GqlRelDeleteEndpoint (input: <GqlRelQueryInput>): [<Node>]
///
/// Ex:
/// ProjectOwnerDelete(input: ProjectOwnerQueryInput): [Project]
fn generate_rel_delete_endpoint(t: &Type, r: &Relationship) -> Property {
    let mut arguments = HashMap::new();
    arguments.insert(
        "input".to_string(),
        Argument::new(
            "input".to_string(),
            ArgumentKind::Required,
            fmt_rel_delete_input_name(t, r),
        ),
    );
    arguments.insert(
        "partitionKey".to_string(),
        Argument::new(
            "partitionKey".to_string(),
            ArgumentKind::Optional,
            "String".to_string(),
        ),
    );

    Property::new(
        fmt_rel_delete_endpoint_name(t, r),
        PropertyKind::RelDeleteMutation {
            src_label: fmt_node_object_name(t),
            rel_name: fmt_rel_name(r),
        },
        "Int".to_string(),
    )
    .with_arguments(arguments)
}

/// Takes a WG Endpoint and returns a NodeType representing a root endpoint
fn generate_custom_endpoint(e: &Endpoint) -> Property {
    let mut arguments = HashMap::new();
    if let Some(input) = e.input() {
        let is_required = if input.required() {
            ArgumentKind::Required
        } else {
            ArgumentKind::Optional
        };

        arguments.insert(
            "partitionKey".to_string(),
            Argument::new(
                "partitionKey".to_string(),
                ArgumentKind::Optional,
                "String".to_string(),
            ),
        );

        match input.type_def() {
            TypeDef::Scalar(s) => match s {
                GraphqlType::Boolean => arguments.insert(
                    "input".to_string(),
                    Argument::new("input".to_string(), is_required, "Boolean".to_string()),
                ),
                GraphqlType::Float => arguments.insert(
                    "input".to_string(),
                    Argument::new("input".to_string(), is_required, "Float".to_string()),
                ),
                GraphqlType::Int => arguments.insert(
                    "input".to_string(),
                    Argument::new("input".to_string(), is_required, "Int".to_string()),
                ),
                GraphqlType::String => arguments.insert(
                    "input".to_string(),
                    Argument::new("input".to_string(), is_required, "String".to_string()),
                ),
            },
            TypeDef::Existing(e) => arguments.insert(
                "input".to_string(),
                Argument::new("input".to_string(), is_required, e.clone()),
            ),
            TypeDef::Custom(c) => arguments.insert(
                "input".to_string(),
                Argument::new("input".to_string(), is_required, c.name().to_string()),
            ),
        };
    }

    Property::new(
        e.name().to_string(),
        PropertyKind::CustomResolver,
        match &e.output().type_def() {
            TypeDef::Scalar(t) => match &t {
                GraphqlType::Int => "Int".to_string(),
                GraphqlType::Float => "Float".to_string(),
                GraphqlType::String => "String".to_string(),
                GraphqlType::Boolean => "Boolean".to_string(),
            },
            TypeDef::Existing(s) => s.clone(),
            TypeDef::Custom(t) => t.name().to_string(),
        },
    )
    .with_required(e.output().required())
    .with_list(e.output().list())
    .with_arguments(arguments)
}

fn generate_custom_endpoint_input(t: &Type) -> NodeType {
    let mut props = generate_props(t.props_as_slice(), false, false);
    t.rels().for_each(|r| {
        props.insert(
            r.name().to_string(),
            Property::new(
                r.name().to_string(),
                PropertyKind::Input,
                fmt_rel_query_input_name(t, &r),
            )
            .with_list(r.list()),
        );
    });
    NodeType::new(t.name().to_string(), TypeKind::Input, props)
}

fn generate_static_version_query() -> Property {
    Property::new(
        "_version".to_string(),
        PropertyKind::VersionQuery,
        "String".to_string(),
    )
}

/// Takes a WG config and returns a map of graphql schema components for model
/// types, custom endpoints, and associated endpoint types
fn generate_schema(c: &Configuration) -> HashMap<String, NodeType> {
    let mut nthm = HashMap::new();
    let mut mutation_props = HashMap::new();
    let mut query_props = HashMap::new();

    // generate graphql schema components for warpgrapher types
    c.types().for_each(|t| {
        // GqlNodeType
        let node_type = generate_node_object(t);
        nthm.insert(node_type.type_name.to_string(), node_type);

        // GqlNodeQueryInput
        let node_query_input = generate_node_query_input(t);
        nthm.insert(node_query_input.type_name.to_string(), node_query_input);

        // GqlNodeCreateMutationInput
        let node_create_mutation_input = generate_node_create_mutation_input(t);
        nthm.insert(
            node_create_mutation_input.type_name.to_string(),
            node_create_mutation_input,
        );

        // GqlNodeUpdateMutationInput
        let node_update_mutation_input = generate_node_update_mutation_input(t);
        nthm.insert(
            node_update_mutation_input.type_name.to_string(),
            node_update_mutation_input,
        );

        // GqlNodeInput
        let node_input = generate_node_input(t);
        nthm.insert(node_input.type_name.to_string(), node_input);

        // GqlNodeUpdateInput
        let node_update_input = generate_node_update_input(t);
        nthm.insert(node_update_input.type_name.to_string(), node_update_input);

        // GqlNodeDeleteInput
        let node_delete_input = generate_node_delete_input(t);
        nthm.insert(node_delete_input.type_name.to_string(), node_delete_input);

        // GqlNodeDeleteMutationInput
        let node_delete_mutation_input = generate_node_delete_mutation_input(t);
        nthm.insert(
            node_delete_mutation_input.type_name.to_string(),
            node_delete_mutation_input,
        );

        // GqlNodeReadEndpoint
        if t.endpoints().read() {
            let read_endpoint = generate_node_read_endpoint(t);
            query_props.insert(read_endpoint.name().to_string(), read_endpoint);
        }

        // GqlNodeCreateEndpoint
        if t.endpoints().create() {
            let create_endpoint = generate_node_create_endpoint(t);
            mutation_props.insert(create_endpoint.name().to_string(), create_endpoint);
        }

        // GqlNodeUpdateEndpoint
        if t.endpoints().update() {
            let update_endpoint = generate_node_update_endpoint(t);
            mutation_props.insert(update_endpoint.name().to_string(), update_endpoint);
        }

        // GqlNodeDeleteEndpoint
        if t.endpoints().delete() {
            let delete_endpoint = generate_node_delete_endpoint(t);
            mutation_props.insert(delete_endpoint.name().to_string(), delete_endpoint);
        }

        t.rels().for_each(|r| {
            // GqlRelObject
            let rel_object = generate_rel_object(t, r);
            nthm.insert(rel_object.type_name.to_string(), rel_object);

            // GqlRelPropsObject
            let rel_props_object = generate_rel_props_object(t, r);
            nthm.insert(rel_props_object.type_name.to_string(), rel_props_object);

            // GqlRelNodesUnion
            let rel_nodes_union = generate_rel_nodes_union(t, r);
            nthm.insert(rel_nodes_union.type_name.to_string(), rel_nodes_union);

            // GqlRelQueryInput
            let rel_query_input = generate_rel_query_input(t, r);
            nthm.insert(rel_query_input.type_name.to_string(), rel_query_input);

            // GqlRelCreateMutationInput
            let rel_create_mutation_input = generate_rel_create_mutation_input(t, r);
            nthm.insert(
                rel_create_mutation_input.type_name.to_string(),
                rel_create_mutation_input,
            );

            // GqlRelChangeInput
            let rel_change_input = generate_rel_change_input(t, r);
            nthm.insert(rel_change_input.type_name.to_string(), rel_change_input);

            // GqlRelUpdateMutationInput
            let rel_update_mutation_input = generate_rel_update_mutation_input(t, r);
            nthm.insert(
                rel_update_mutation_input.type_name.to_string(),
                rel_update_mutation_input,
            );

            // GqlRelSrcUpdateMutationInput
            let rel_src_update_mutation_input = generate_rel_src_update_mutation_input(t, r);
            nthm.insert(
                rel_src_update_mutation_input.type_name.to_string(),
                rel_src_update_mutation_input,
            );

            // GqlRelDstUpdateMutationInput
            let rel_dst_update_mutation_input = generate_rel_dst_update_mutation_input(t, r);
            nthm.insert(
                rel_dst_update_mutation_input.type_name.to_string(),
                rel_dst_update_mutation_input,
            );

            // GqlRelPropsInput
            let rel_props_input = generate_rel_props_input(t, r);
            nthm.insert(rel_props_input.type_name.to_string(), rel_props_input);

            // GqlRelSrcQueryInput
            let rel_src_query_input = generate_rel_src_query_input(t, r);
            nthm.insert(
                rel_src_query_input.type_name.to_string(),
                rel_src_query_input,
            );

            // GqlRelDstQueryInput
            let rel_dst_query_input = generate_rel_dst_query_input(t, r);
            nthm.insert(
                rel_dst_query_input.type_name.to_string(),
                rel_dst_query_input,
            );

            // GqlRelNodesMutationInputUnion
            let rel_nodes_mutation_input_union = generate_rel_nodes_mutation_input_union(t, r);
            nthm.insert(
                rel_nodes_mutation_input_union.type_name.to_string(),
                rel_nodes_mutation_input_union,
            );

            // GqlRelCreateInput
            let rel_create_input = generate_rel_create_input(t, r);
            nthm.insert(rel_create_input.type_name.to_string(), rel_create_input);

            // GqlRelUpdateInput
            let rel_update_input = generate_rel_update_input(t, r);
            nthm.insert(rel_update_input.type_name.to_string(), rel_update_input);

            // GqlRelDeleteInput
            let rel_delete_input = generate_rel_delete_input(t, r);
            nthm.insert(rel_delete_input.type_name.to_string(), rel_delete_input);

            // GqlRelSrcDeleteMutationInput
            let rel_src_delete_mutation_input = generate_rel_src_delete_mutation_input(t, r);
            nthm.insert(
                rel_src_delete_mutation_input.type_name.to_string(),
                rel_src_delete_mutation_input,
            );

            // GqlRelDstDeleteMutationInput
            let rel_dst_delete_mutation_input = generate_rel_dst_delete_mutation_input(t, r);
            nthm.insert(
                rel_dst_delete_mutation_input.type_name.to_string(),
                rel_dst_delete_mutation_input,
            );

            // GqlRelReadEndpoint
            if r.endpoints().read() {
                let rel_read_endpoint = generate_rel_read_endpoint(t, r);
                query_props.insert(rel_read_endpoint.name().to_string(), rel_read_endpoint);
            }

            // GqlRelCreateEndpoint
            if r.endpoints().create() {
                let rel_create_endpoint = generate_rel_create_endpoint(t, r);
                mutation_props.insert(rel_create_endpoint.name().to_string(), rel_create_endpoint);
            }

            // GqlRelUpdateEndpoint
            if r.endpoints().update() {
                let rel_update_endpoint = generate_rel_update_endpoint(t, r);
                mutation_props.insert(rel_update_endpoint.name().to_string(), rel_update_endpoint);
            }

            // GqlRelDelete Endpoint
            if r.endpoints().delete() {
                let rel_delete_endpoint = generate_rel_delete_endpoint(t, r);
                mutation_props.insert(rel_delete_endpoint.name().to_string(), rel_delete_endpoint);
            }
        });
    });

    // generate graphql schema components for custom endpoints and associated types
    c.endpoints().for_each(|e| {
        // add custom endpoint
        let endpoint = generate_custom_endpoint(e);
        match e.class() {
            EndpointClass::Mutation => {
                mutation_props.insert(e.name().to_string(), endpoint);
            }
            EndpointClass::Query => {
                query_props.insert(e.name().to_string(), endpoint);
            }
        }

        // add custom input type if provided
        if let Some(input) = e.input() {
            if let TypeDef::Custom(t) = input.type_def() {
                let input = generate_custom_endpoint_input(&t);
                nthm.insert(t.name().to_string(), input);
            }
        }

        // add custom output type if provided
        if let TypeDef::Custom(t) = &e.output().type_def() {
            let node_type = generate_node_object(&t);
            nthm.insert(node_type.type_name.to_string(), node_type);
        }
    });

    // static endpoints
    query_props.insert("_version".to_string(), generate_static_version_query());

    // insert
    nthm.insert(
        "Mutation".to_string(),
        NodeType::new("Mutation".to_string(), TypeKind::Object, mutation_props),
    );

    nthm.insert(
        "Query".to_string(),
        NodeType::new("Query".to_string(), TypeKind::Object, query_props),
    );

    nthm
}

/// Takes a Warpgrapher configuration and returns the Juniper RootNode for a
/// GraphQL schema that matches the Warpgrapher configuration.
///
/// # Errors
/// Returns an [`Error`] of kind [`CouldNotResolveType`] if
/// there is an error in the configuration, specifically if the
/// configuration of type A references type B, but type B cannot be found.
///
/// [`Error`]: ../error/struct.Error.html
/// [`CouldNotResolveType`]: ../error/enum.ErrorKind.html#variant.CouldNotResolveType
///
pub(super) fn create_root_node<RequestCtx>(c: &Configuration) -> Result<RootRef<RequestCtx>, Error>
where
    RequestCtx: RequestContext,
{
    // Runtime performance could be optimized by generating the entirety of the
    // schema in one loop iteration over the configuration. In fact, that's how
    // the first iteration of the code worked. However, doing so adds code
    // complexity, as all the schema objects built from any given
    // Type are built at once. This implementation opts for clarity
    // over runtime efficiency, given that the number of configuration items
    // is lkely to be small.

    let nthm = generate_schema(c);
    let nts = Arc::new(nthm);
    let root_mutation_info = Info::new("Mutation".to_string(), nts.clone());
    let root_query_info = Info::new("Query".to_string(), nts);
    catch_unwind(|| {
        Arc::new(RootNode::new_with_info(
            Node::new("Query".to_string(), HashMap::new()),
            Node::new("Mutation".to_string(), HashMap::new()),
            root_query_info,
            root_mutation_info,
        ))
    })
    .map_err(|e| {
        e.downcast::<Error>()
            .map(|e| *e)
            .unwrap_or_else(|e| Error::SchemaItemNotFound {
                name: format!("{:#?}", e),
            })
    })
}

pub(crate) fn type_name_variants(t: &Type) -> HashSet<String> {
    let mut hs = HashSet::new();

    hs.insert(fmt_node_query_input_name(t));
    hs.insert(fmt_node_create_mutation_input_name(t));
    hs.insert(fmt_node_update_mutation_input_name(t));
    hs.insert(fmt_node_input_name(t));
    hs.insert(fmt_node_update_input_name(t));
    hs.insert(fmt_node_delete_input_name(t));
    hs.insert(fmt_node_delete_mutation_input_name(t));
    hs.insert(fmt_node_create_endpoint_name(t));
    hs.insert(fmt_node_update_endpoint_name(t));
    hs.insert(fmt_node_delete_endpoint_name(t));

    hs
}

pub(crate) fn rel_name_variants(t: &Type, r: &Relationship) -> HashSet<String> {
    let mut hs = HashSet::new();

    hs.insert(fmt_rel_object_name(t, r));
    hs.insert(fmt_rel_name(r));
    hs.insert(fmt_rel_props_object_name(t, r));
    hs.insert(fmt_rel_nodes_union_name(t, r));
    hs.insert(fmt_rel_query_input_name(t, r));
    hs.insert(fmt_rel_create_mutation_input_name(t, r));
    hs.insert(fmt_rel_change_input_name(t, r));
    hs.insert(fmt_rel_update_mutation_input_name(t, r));
    hs.insert(fmt_rel_src_update_mutation_input_name(t, r));
    hs.insert(fmt_rel_dst_update_mutation_input_name(t, r));
    hs.insert(fmt_rel_props_input_name(t, r));
    hs.insert(fmt_rel_src_query_input_name(t, r));
    hs.insert(fmt_rel_dst_query_input_name(t, r));
    hs.insert(fmt_rel_nodes_mutation_input_union_name(t, r));
    hs.insert(fmt_rel_create_input_name(t, r));
    hs.insert(fmt_rel_update_input_name(t, r));
    hs.insert(fmt_rel_delete_input_name(t, r));
    hs.insert(fmt_rel_src_delete_mutation_input_name(t, r));
    hs.insert(fmt_rel_dst_delete_mutation_input_name(t, r));
    hs.insert(fmt_rel_read_endpoint_name(t, r));
    hs.insert(fmt_rel_create_endpoint_name(t, r));
    hs.insert(fmt_rel_update_endpoint_name(t, r));
    hs.insert(fmt_rel_delete_endpoint_name(t, r));

    hs
}

#[cfg(test)]
mod tests {
    use super::{
        create_root_node, fmt_node_create_endpoint_name, fmt_node_create_mutation_input_name,
        fmt_node_delete_endpoint_name, fmt_node_delete_input_name,
        fmt_node_delete_mutation_input_name, fmt_node_input_name, fmt_node_object_name,
        fmt_node_query_input_name, fmt_node_read_endpoint_name, fmt_node_update_endpoint_name,
        fmt_node_update_input_name, fmt_node_update_mutation_input_name, fmt_rel_change_input_name,
        fmt_rel_create_endpoint_name, fmt_rel_create_input_name,
        fmt_rel_create_mutation_input_name, fmt_rel_delete_endpoint_name,
        fmt_rel_delete_input_name, fmt_rel_dst_delete_mutation_input_name,
        fmt_rel_dst_query_input_name, fmt_rel_dst_update_mutation_input_name,
        fmt_rel_nodes_mutation_input_union_name, fmt_rel_nodes_union_name, fmt_rel_object_name,
        fmt_rel_props_input_name, fmt_rel_props_object_name, fmt_rel_query_input_name,
        fmt_rel_read_endpoint_name, fmt_rel_src_delete_mutation_input_name,
        fmt_rel_src_query_input_name, fmt_rel_src_update_mutation_input_name,
        fmt_rel_update_endpoint_name, fmt_rel_update_input_name,
        fmt_rel_update_mutation_input_name, generate_custom_endpoint,
        generate_node_create_endpoint, generate_node_create_mutation_input,
        generate_node_delete_endpoint, generate_node_delete_input,
        generate_node_delete_mutation_input, generate_node_input, generate_node_object,
        generate_node_query_input, generate_node_read_endpoint, generate_node_update_endpoint,
        generate_node_update_input, generate_node_update_mutation_input, generate_rel_change_input,
        generate_rel_create_endpoint, generate_rel_create_input,
        generate_rel_create_mutation_input, generate_rel_delete_endpoint,
        generate_rel_delete_input, generate_rel_dst_delete_mutation_input,
        generate_rel_dst_query_input, generate_rel_dst_update_mutation_input,
        generate_rel_nodes_mutation_input_union, generate_rel_nodes_union, generate_rel_object,
        generate_rel_props_input, generate_rel_props_object, generate_rel_query_input,
        generate_rel_read_endpoint, generate_rel_src_delete_mutation_input,
        generate_rel_src_update_mutation_input, generate_rel_update_endpoint,
        generate_rel_update_input, generate_rel_update_mutation_input, generate_schema,
        ArgumentKind, Info, NodeType, Property, PropertyKind, TypeKind,
    };
    use crate::engine::config::{
        mock_config, mock_endpoint_one, mock_endpoint_three, mock_endpoint_two,
        mock_endpoints_filter, mock_project_config, mock_project_type,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Passes if a new Info struct is created
    #[test]
    fn info_new() {
        let i = Info::new("typename".to_string(), Arc::new(HashMap::new()));

        assert!(i.name == "typename");
    }

    /// Passes if a new NodeType is created
    #[test]
    fn node_type_new() {
        let nt = NodeType::new("typename".to_string(), TypeKind::Object, HashMap::new());

        assert!(nt.type_name == "typename");
        assert!(nt.type_kind == TypeKind::Object);
    }

    /// Passes if a new Property is created
    #[test]
    fn property_new() {
        let p = Property::new(
            "propname".to_string(),
            PropertyKind::Scalar,
            "String".to_string(),
        )
        .with_required(true);

        assert!(p.name == "propname");
        assert!(p.kind == PropertyKind::Scalar);
        assert!(p.type_name == "String");
        assert!(p.required);
        assert!(!p.list());
        assert!(p.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_object_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_object_name(&project_type) == "Project");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_node_object() {
        /*
            type Project {
                id: ID!
                name: String!
                tags: [String]
                public: Boolean!
                owner(input: ProjectOwnerQueryInput): ProjectOwnerRel
                commits(input: ProjectCommitsQueryInput): ProjectCommitsRel
                issues(input: ProjectIssuesQueryInput): ProjectIssuesRel
                board(input: ProjectBoardQueryInput): ProjectBoardRel
            }
        */
        let project_type = mock_project_type();
        let project_node_object = generate_node_object(&project_type);
        assert!(project_node_object.type_name == "Project");
        assert!(project_node_object.props.len() == 8);
        assert!(project_node_object.type_kind == TypeKind::Object);
        let project_id = project_node_object.props.get("id").unwrap();
        assert!(project_id.name == "id");
        assert!(project_id.kind == PropertyKind::Scalar);
        assert!(project_id.type_name == "ID");
        assert!(project_id.required);
        assert!(!project_id.list);
        assert!(project_id.arguments.is_empty());
        let project_name = project_node_object.props.get("name").unwrap();
        assert!(project_name.name == "name");
        assert!(project_name.kind == PropertyKind::Scalar);
        assert!(project_name.type_name == "String");
        assert!(project_name.required);
        assert!(!project_name.list);
        assert!(project_name.arguments.is_empty());
        let project_tags = project_node_object.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.arguments.is_empty());
        let project_public = project_node_object.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(project_public.required);
        assert!(!project_public.list);
        assert!(project_public.arguments.is_empty());
        let project_owner = project_node_object.props.get("owner").unwrap();
        assert!(project_owner.name() == "owner");
        assert!(match &project_owner.kind {
            PropertyKind::Rel { rel_name } => rel_name == "owner",
            _ => false,
        });
        assert!(project_owner.type_name == "ProjectOwnerRel");
        assert!(!project_owner.required);
        assert!(!project_owner.list());
        assert!(project_owner.arguments.contains_key("input"));
        if let Some(input) = project_owner.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectOwnerQueryInput");
        }
        let project_board = project_node_object.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(match &project_board.kind {
            PropertyKind::Rel { rel_name } => rel_name == "board",
            _ => false,
        });
        assert!(project_board.type_name == "ProjectBoardRel");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(project_board.arguments.contains_key("input"));
        if let Some(input) = project_board.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectBoardQueryInput");
        }
        let project_commits = project_node_object.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(match &project_commits.kind {
            PropertyKind::Rel { rel_name } => rel_name == "commits",
            _ => false,
        });
        assert!(project_commits.type_name == "ProjectCommitsRel");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(project_commits.arguments.contains_key("input"));
        if let Some(input) = project_commits.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectCommitsQueryInput");
        }
        let project_issues = project_node_object.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(match &project_issues.kind {
            PropertyKind::Rel { rel_name } => rel_name == "issues",
            _ => false,
        });
        assert!(project_issues.type_name == "ProjectIssuesRel");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(project_issues.arguments.contains_key("input"));
        if let Some(input) = project_issues.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectIssuesQueryInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_query_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_query_input_name(&project_type) == "ProjectQueryInput");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_node_query_input() {
        /*
            input ProjectQueryInput {
                id: ID
                name: String
                tags: [String]
                public: Boolean
                owner: ProjectOwnerQueryInput
                board: ProjectBoardQueryInput
                commits: [ProjectCommitsQueryInput]
                issues: [ProjectIssuesQueryInput]
            }
        */
        let project_type = mock_project_type();
        let project_query_input = generate_node_query_input(&project_type);
        assert!(project_query_input.type_name == "ProjectQueryInput");
        assert!(project_query_input.type_kind == TypeKind::Input);
        assert!(project_query_input.props.len() == 8);
        let project_id = project_query_input.props.get("id").unwrap();
        assert!(project_id.name == "id");
        assert!(project_id.kind == PropertyKind::Scalar);
        assert!(project_id.type_name == "ID");
        assert!(!project_id.required);
        assert!(!project_id.list);
        assert!(project_id.arguments.is_empty());
        let project_name = project_query_input.props.get("name").unwrap();
        assert!(project_name.name == "name");
        assert!(project_name.kind == PropertyKind::Scalar);
        assert!(project_name.type_name == "String");
        assert!(!project_name.required);
        assert!(!project_name.list);
        assert!(project_name.arguments.is_empty());
        let project_tags = project_query_input.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.arguments.is_empty());
        let project_public = project_query_input.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(!project_public.required);
        assert!(!project_public.list);
        assert!(project_public.arguments.is_empty());
        let project_owner = project_query_input.props.get("owner").unwrap();
        assert!(project_owner.name() == "owner");
        assert!(project_owner.kind == PropertyKind::Input);
        assert!(project_owner.type_name == "ProjectOwnerQueryInput");
        assert!(!project_owner.required);
        assert!(!project_owner.list());
        assert!(project_owner.arguments.is_empty());
        let project_board = project_query_input.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(project_board.kind == PropertyKind::Input);
        assert!(project_board.type_name == "ProjectBoardQueryInput");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(project_board.arguments.is_empty());
        let project_commits = project_query_input.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(project_commits.kind == PropertyKind::Input);
        assert!(project_commits.type_name == "ProjectCommitsQueryInput");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(project_commits.arguments.is_empty());
        let project_issues = project_query_input.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(project_issues.kind == PropertyKind::Input);
        assert!(project_issues.type_name == "ProjectIssuesQueryInput");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(project_issues.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_create_mutation_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_create_mutation_input_name(&project_type) == "ProjectCreateMutationInput");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_node_create_mutation_input() {
        /*
            input ProjectCreateMutationInput {
                name: String
                tags: [String]
                public: Boolean
                owner: ProjectOwnerCreateMutationInput
                board: ProjectBoardCreateMutationInput
                commits: [ProjectCommitsMutationInput]
                issues: [ProjectIssuesMutationInput]
            }
        */
        let project_type = mock_project_type();
        let project_mutation_input = generate_node_create_mutation_input(&project_type);
        assert!(project_mutation_input.type_name == "ProjectCreateMutationInput");
        assert!(project_mutation_input.type_kind == TypeKind::Input);
        assert!(project_mutation_input.props.len() == 7);
        let project_name = project_mutation_input.props.get("name").unwrap();
        assert!(project_name.name == "name");
        assert!(project_name.kind == PropertyKind::Scalar);
        assert!(project_name.type_name == "String");
        assert!(!project_name.required);
        assert!(!project_name.list);
        assert!(project_name.arguments.is_empty());
        let project_tags = project_mutation_input.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.arguments.is_empty());
        let project_public = project_mutation_input.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(!project_public.required);
        assert!(!project_public.list);
        assert!(project_public.arguments.is_empty());
        let project_owner = project_mutation_input.props.get("owner").unwrap();
        assert!(project_owner.name() == "owner");
        assert!(project_owner.kind == PropertyKind::Input);
        assert!(project_owner.type_name == "ProjectOwnerCreateMutationInput");
        assert!(!project_owner.required);
        assert!(!project_owner.list());
        assert!(project_owner.arguments.is_empty());
        let project_board = project_mutation_input.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(project_board.kind == PropertyKind::Input);
        assert!(project_board.type_name == "ProjectBoardCreateMutationInput");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(project_board.arguments.is_empty());
        let project_commits = project_mutation_input.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(project_commits.kind == PropertyKind::Input);
        assert!(project_commits.type_name == "ProjectCommitsCreateMutationInput");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(project_commits.arguments.is_empty());
        let project_issues = project_mutation_input.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(project_issues.kind == PropertyKind::Input);
        assert!(project_issues.type_name == "ProjectIssuesCreateMutationInput");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(project_issues.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_update_mutation_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_update_mutation_input_name(&project_type) == "ProjectUpdateMutationInput");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_node_update_mutation_input() {
        /*
            input ProjectUpdateMutationInput {
                name: String
                tags: [String]
                public: Boolean
                owner: ProjectOwnerChangeInput
                board: ProjectBoardChangeInput
                commits: [ProjectCommitsChangeInput]
                issues: [ProjectIssuesChangeInput]
            }
        */
        let project_type = mock_project_type();
        let project_update_mutation_input = generate_node_update_mutation_input(&project_type);
        assert!(project_update_mutation_input.type_name == "ProjectUpdateMutationInput");
        assert!(project_update_mutation_input.type_kind == TypeKind::Input);
        assert!(project_update_mutation_input.props.len() == 7);
        let name = project_update_mutation_input.props.get("name").unwrap();
        assert!(name.name == "name");
        assert!(name.kind == PropertyKind::Scalar);
        assert!(name.type_name == "String");
        assert!(!name.required);
        assert!(!name.list);
        assert!(name.arguments.is_empty());
        let tags = project_update_mutation_input.props.get("tags").unwrap();
        assert!(tags.name == "tags");
        assert!(tags.kind == PropertyKind::Scalar);
        assert!(tags.type_name == "String");
        assert!(!tags.required);
        assert!(tags.list);
        assert!(tags.arguments.is_empty());
        let public = project_update_mutation_input.props.get("public").unwrap();
        assert!(public.name == "public");
        assert!(public.kind == PropertyKind::Scalar);
        assert!(public.type_name == "Boolean");
        assert!(!public.required);
        assert!(!public.list);
        assert!(public.arguments.is_empty());
        let owner = project_update_mutation_input.props.get("owner").unwrap();
        assert!(owner.name() == "owner");
        assert!(owner.kind == PropertyKind::Input);
        assert!(owner.type_name == "ProjectOwnerChangeInput");
        assert!(!owner.required);
        assert!(!owner.list());
        assert!(owner.arguments.is_empty());
        let board = project_update_mutation_input.props.get("board").unwrap();
        assert!(board.name == "board");
        assert!(board.kind == PropertyKind::Input);
        assert!(board.type_name == "ProjectBoardChangeInput");
        assert!(!board.required);
        assert!(!board.list);
        assert!(board.arguments.is_empty());
        let commits = project_update_mutation_input.props.get("commits").unwrap();
        assert!(commits.name == "commits");
        assert!(commits.kind == PropertyKind::Input);
        assert!(commits.type_name == "ProjectCommitsChangeInput");
        assert!(!commits.required);
        assert!(commits.list);
        assert!(commits.arguments.is_empty());
        let issues = project_update_mutation_input.props.get("issues").unwrap();
        assert!(issues.name == "issues");
        assert!(issues.kind == PropertyKind::Input);
        assert!(issues.type_name == "ProjectIssuesChangeInput");
        assert!(!issues.required);
        assert!(issues.list);
        assert!(issues.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_input_name(&project_type) == "ProjectInput");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_input() {
        /*
            input ProjectInput {
                $EXISTING: ProjectQueryInput
                $NEW: ProjectCreateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_input = generate_node_input(&project_type);
        let project_match = project_input.props.get("$EXISTING").unwrap();
        assert!(project_match.name == "$EXISTING");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.arguments.is_empty());
        let project_create = project_input.props.get("$NEW").unwrap();
        assert!(project_create.name == "$NEW");
        assert!(project_create.kind == PropertyKind::Input);
        assert!(project_create.type_name == "ProjectCreateMutationInput");
        assert!(!project_create.required);
        assert!(!project_create.list);
        assert!(project_create.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_update_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_update_input_name(&project_type) == "ProjectUpdateInput");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_update_input() {
        /*
            input ProjectUpdateInput {
                $MATCH: ProjectQueryInput
                $SET: ProjectUpdateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_update_input = generate_node_update_input(&project_type);
        let project_match = project_update_input.props.get("$MATCH").unwrap();
        assert!(project_match.name == "$MATCH");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.arguments.is_empty());
        let project_update = project_update_input.props.get("$SET").unwrap();
        assert!(project_update.name == "$SET");
        assert!(project_update.kind == PropertyKind::Input);
        assert!(project_update.type_name == "ProjectUpdateMutationInput");
        assert!(!project_update.required);
        assert!(!project_update.list);
        assert!(project_update.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_delete_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_delete_input_name(&project_type) == "ProjectDeleteInput");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_delete_input() {
        /*
            input ProjectDeleteInput {
                $MATCH: ProjectQueryInput
                delete: ProjectDeleteMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_delete_input = generate_node_delete_input(&project_type);
        assert!(project_delete_input.type_name == "ProjectDeleteInput");
        assert!(project_delete_input.props.len() == 2);
        let project_match = project_delete_input.props.get("$MATCH").unwrap();
        assert!(project_match.name == "$MATCH");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.arguments.is_empty());
        let project_delete = project_delete_input.props.get("$DELETE").unwrap();
        assert!(project_delete.name == "$DELETE");
        assert!(project_delete.kind == PropertyKind::Input);
        assert!(project_delete.type_name == "ProjectDeleteMutationInput");
        assert!(!project_delete.required);
        assert!(!project_delete.list);
        assert!(project_delete.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_delete_mutation_input_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_delete_mutation_input_name(&project_type) == "ProjectDeleteMutationInput");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_node_delete_mutation_input() {
        /*
        input ProjectDeleteMutationInput {
            owner: ProjectOwnerDeleteInput
            board: ProjectBoardDeleteInput
            commits: ProjectCommitsDeleteInput
            issues: ProjectIssuesDeleteInput
        }
        */
        let project_type = mock_project_type();
        let project_delete_mutation_input = generate_node_delete_mutation_input(&project_type);
        assert!(project_delete_mutation_input.type_name == "ProjectDeleteMutationInput");
        assert!(project_delete_mutation_input.props.len() == 4);
        let owner = project_delete_mutation_input.props.get("owner").unwrap();
        assert!(owner.name() == "owner");
        assert!(owner.kind == PropertyKind::Input);
        assert!(owner.type_name == "ProjectOwnerDeleteInput");
        assert!(!owner.required);
        assert!(!owner.list());
        assert!(owner.arguments.is_empty());
        let board = project_delete_mutation_input.props.get("board").unwrap();
        assert!(board.name == "board");
        assert!(board.kind == PropertyKind::Input);
        assert!(board.type_name == "ProjectBoardDeleteInput");
        assert!(!board.required);
        assert!(!board.list);
        assert!(board.arguments.is_empty());
        let commits = project_delete_mutation_input.props.get("commits").unwrap();
        assert!(commits.name == "commits");
        assert!(commits.kind == PropertyKind::Input);
        assert!(commits.type_name == "ProjectCommitsDeleteInput");
        assert!(!commits.required);
        assert!(commits.list);
        assert!(commits.arguments.is_empty());
        let issues = project_delete_mutation_input.props.get("issues").unwrap();
        assert!(issues.name == "issues");
        assert!(issues.kind == PropertyKind::Input);
        assert!(issues.type_name == "ProjectIssuesDeleteInput");
        assert!(!issues.required);
        assert!(issues.list);
        assert!(issues.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_read_endpoint_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_read_endpoint_name(&project_type) == "Project");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_read_endpoint() {
        /*
            Project(input: ProjectQueryInput): [Project]
        */
        let project_type = mock_project_type();
        let project_read_endpoint = generate_node_read_endpoint(&project_type);
        assert!(project_read_endpoint.name == "Project");
        assert!(project_read_endpoint.kind == PropertyKind::Object);
        assert!(project_read_endpoint.type_name == "Project");
        assert!(!project_read_endpoint.required);
        assert!(project_read_endpoint.list);
        assert!(project_read_endpoint.arguments.contains_key("input"));
        if let Some(input) = project_read_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectQueryInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_create_endpoint_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_create_endpoint_name(&project_type) == "ProjectCreate");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_create_endpoint() {
        /*
            ProjectCreate(input: ProjectMutationInput): Project
        */
        let project_type = mock_project_type();
        let project_create_endpoint = generate_node_create_endpoint(&project_type);
        assert!(project_create_endpoint.name == "ProjectCreate");
        assert!(project_create_endpoint.kind == PropertyKind::NodeCreateMutation);
        assert!(project_create_endpoint.type_name == "Project");
        assert!(!project_create_endpoint.required);
        assert!(!project_create_endpoint.list);
        assert!(project_create_endpoint.arguments.contains_key("input"));
        if let Some(input) = project_create_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectCreateMutationInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_update_endpoint_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_update_endpoint_name(&project_type) == "ProjectUpdate");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_update_endpoint() {
        /*
            ProjectUpdate(input: ProjectUpdateInput): [Project]
        */
        let project_type = mock_project_type();
        let project_update_endpoint = generate_node_update_endpoint(&project_type);
        assert!(project_update_endpoint.name == "ProjectUpdate");
        assert!(project_update_endpoint.kind == PropertyKind::NodeUpdateMutation);
        assert!(project_update_endpoint.type_name == "Project");
        assert!(!project_update_endpoint.required);
        assert!(project_update_endpoint.list);
        assert!(project_update_endpoint.arguments.contains_key("input"));
        if let Some(input) = project_update_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectUpdateInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_node_delete_endpoint_name() {
        let project_type = mock_project_type();
        assert!(fmt_node_delete_endpoint_name(&project_type) == "ProjectDelete");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_node_delete_endpoint() {
        /*
            ProjectDelete (input: ProjectDeleteInput): Int
        */
        let project_type = mock_project_type();
        let project_delete_endpoint = generate_node_delete_endpoint(&project_type);
        assert!(project_delete_endpoint.name == "ProjectDelete");
        assert!(match &project_delete_endpoint.kind {
            PropertyKind::NodeDeleteMutation { label } => label == "Project",
            _ => false,
        });
        assert!(project_delete_endpoint.type_name == "Int");
        assert!(!project_delete_endpoint.required);
        assert!(!project_delete_endpoint.list);
        assert!(project_delete_endpoint.arguments.contains_key("input"));
        if let Some(input) = project_delete_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectDeleteInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_object_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(fmt_rel_object_name(&project_type, &project_owner_rel) == "ProjectOwnerRel");
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_rel_object() {
        /*
            type ProjectOwnerRel {
                id: ID!
                props: ProjectOwnerProps
                dst: ProjectOwnerNodesUnion!
                src: Project!
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_object = generate_rel_object(&project_type, &project_owner_rel);
        let project_owner_id = project_owner_object.props.get("id").unwrap();
        assert!(project_owner_id.name == "id");
        assert!(project_owner_id.kind == PropertyKind::Scalar);
        assert!(project_owner_id.type_name == "ID");
        assert!(project_owner_id.required);
        assert!(!project_owner_id.list);
        assert!(project_owner_id.arguments.is_empty());
        let project_owner_props = project_owner_object.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Object);
        assert!(project_owner_props.type_name == "ProjectOwnerProps");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.arguments.is_empty());
        let project_owner_dst = project_owner_object.props.get("dst").unwrap();
        assert!(project_owner_dst.name == "dst");
        assert!(project_owner_dst.kind == PropertyKind::Union);
        assert!(project_owner_dst.type_name == "ProjectOwnerNodesUnion");
        assert!(project_owner_dst.required);
        assert!(!project_owner_dst.list);
        assert!(project_owner_dst.arguments.is_empty());
        let project_owner_src = project_owner_object.props.get("src").unwrap();
        assert!(project_owner_src.name == "src");
        assert!(project_owner_src.kind == PropertyKind::Object);
        assert!(project_owner_src.type_name == "Project");
        assert!(project_owner_src.required);
        assert!(!project_owner_src.list);
        assert!(project_owner_src.arguments.is_empty());
        /*
            type ProjectBoardRel {
                id: ID!
                props: ProjectBoardProps
                dst: ProjectBoardNodesUnion!
                src: Project!
            }
        */
        let project_type = mock_project_type();
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_object = generate_rel_object(&project_type, &project_board_rel);
        let project_board_id = project_board_object.props.get("id").unwrap();
        assert!(project_board_id.name == "id");
        assert!(project_board_id.kind == PropertyKind::Scalar);
        assert!(project_board_id.type_name == "ID");
        assert!(project_board_id.required);
        assert!(!project_board_id.list);
        assert!(project_board_id.arguments.is_empty());
        let project_board_props = project_board_object.props.get("props");
        assert!(project_board_props.is_none());
        let project_board_dst = project_board_object.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Union);
        assert!(project_board_dst.type_name == "ProjectBoardNodesUnion");
        assert!(project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.arguments.is_empty());
        let project_board_src = project_board_object.props.get("src").unwrap();
        assert!(project_board_src.name == "src");
        assert!(project_board_src.kind == PropertyKind::Object);
        assert!(project_board_src.type_name == "Project");
        assert!(project_board_src.required);
        assert!(!project_board_src.list);
        assert!(project_board_src.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_props_object_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_props_object_name(&project_type, &project_owner_rel) == "ProjectOwnerProps"
        );
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_rel_props_object() {
        /*
            type ProjectOwnerProps {
                since: String
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_props_object =
            generate_rel_props_object(&project_type, &project_owner_rel);
        assert!(project_owner_props_object.props.len() == 1);
        let project_owner_props_name = project_owner_props_object.props.get("since").unwrap();
        assert!(project_owner_props_name.name == "since");
        assert!(project_owner_props_name.kind == PropertyKind::Scalar);
        assert!(project_owner_props_name.type_name == "String");
        assert!(!project_owner_props_name.required);
        assert!(!project_owner_props_name.list);
        assert!(project_owner_props_name.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_nodes_union_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_nodes_union_name(&project_type, &project_owner_rel) == "ProjectOwnerNodesUnion"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_nodes_union() {
        /*
            union ProjectOwnerNodesUnion = User
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_nodes_union = generate_rel_nodes_union(&project_type, &project_owner_rel);
        assert!(project_owner_nodes_union.type_name == "ProjectOwnerNodesUnion");
        assert!(project_owner_nodes_union.type_kind == TypeKind::Union);
        assert!(project_owner_nodes_union.props.is_empty());
        let project_owner_nodes = project_owner_nodes_union.union_types.unwrap();
        assert!(project_owner_nodes.len() == 1);
        assert!(project_owner_nodes[0] == "User");
        /*
            union ProjectBoardNodesUnion = ScrumBoard | KanbanBoard
        */
        let project_type = mock_project_type();
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_nodes_union = generate_rel_nodes_union(&project_type, &project_board_rel);
        assert!(project_board_nodes_union.type_name == "ProjectBoardNodesUnion");
        assert!(project_board_nodes_union.type_kind == TypeKind::Union);
        assert!(project_board_nodes_union.props.is_empty());
        let project_board_nodes = project_board_nodes_union.union_types.unwrap();
        assert!(project_board_nodes.len() == 2);
        assert!(project_board_nodes[0] == "ScrumBoard");
        assert!(project_board_nodes[1] == "KanbanBoard");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_query_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_query_input_name(&project_type, &project_owner_rel) == "ProjectOwnerQueryInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_rel_query_input() {
        /*
            input ProjectOwnerQueryInput {
                id: ID
                props: ProjectOwnerPropsInput
                src: ProjectOwnerSrcQueryInput
                dst: ProjectOwnerDstQueryInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_query_input = generate_rel_query_input(&project_type, &project_owner_rel);
        // id
        let project_owner_id = project_owner_query_input.props.get("id").unwrap();
        assert!(project_owner_id.name == "id");
        assert!(project_owner_id.kind == PropertyKind::Scalar);
        assert!(project_owner_id.type_name == "ID");
        assert!(!project_owner_id.required);
        assert!(!project_owner_id.list);
        assert!(project_owner_id.arguments.is_empty());
        // properties
        let project_owner_props = project_owner_query_input.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerPropsInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.arguments.is_empty());
        // src
        let project_owner_props = project_owner_query_input.props.get("src").unwrap();
        assert!(project_owner_props.name == "src");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerSrcQueryInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.arguments.is_empty());
        // dst
        let project_owner_props = project_owner_query_input.props.get("dst").unwrap();
        assert!(project_owner_props.name == "dst");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerDstQueryInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.arguments.is_empty());
        /*
            input ProjectBoardQueryInput {
                id: ID
                props: ProjectBoardPropsInput
                src: ProjectBoardSrcQueryInput
                dst: ProjectBoardDstQueryInput
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_query_input = generate_rel_query_input(&project_type, &project_board_rel);
        // id
        let project_board_id = project_board_query_input.props.get("id").unwrap();
        assert!(project_board_id.name == "id");
        assert!(project_board_id.kind == PropertyKind::Scalar);
        assert!(project_board_id.type_name == "ID");
        assert!(!project_board_id.required);
        assert!(!project_board_id.list);
        assert!(project_board_id.arguments.is_empty());
        // properties
        assert!(project_board_query_input.props.get("props").is_none());
        // src
        let project_board_src = project_board_query_input.props.get("src").unwrap();
        assert!(project_board_src.name == "src");
        assert!(project_board_src.kind == PropertyKind::Input);
        assert!(project_board_src.type_name == "ProjectBoardSrcQueryInput");
        assert!(!project_board_src.required);
        assert!(!project_board_src.list);
        assert!(project_board_src.arguments.is_empty());
        // dst
        let project_board_dst = project_board_query_input.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Input);
        assert!(project_board_dst.type_name == "ProjectBoardDstQueryInput");
        assert!(!project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_create_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerCreateMutationInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_create_mutation_input() {
        /*
            input ProjectOwnerCreateMutationInput {
                props: ProjectOwnerPropsInput
                dst: ProjectOwnerNodesMutationInputUnion
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_mutation_input =
            generate_rel_create_mutation_input(&project_type, &project_owner_rel);
        assert!(project_owner_mutation_input.type_name == "ProjectOwnerCreateMutationInput");
        // properties
        let project_owner_props = project_owner_mutation_input.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerPropsInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.arguments.is_empty());
        // dst
        let project_owner_dst = project_owner_mutation_input.props.get("dst").unwrap();
        assert!(project_owner_dst.name == "dst");
        assert!(project_owner_dst.kind == PropertyKind::Input);
        assert!(project_owner_dst.type_name == "ProjectOwnerNodesMutationInputUnion");
        assert!(project_owner_dst.required);
        assert!(!project_owner_dst.list);
        assert!(project_owner_dst.arguments.is_empty());
        /*
            input ProjectBoardCreateMutationInput {
                dst: ProjectBoardNodesMutationInputUnion
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_mutation_input =
            generate_rel_create_mutation_input(&project_type, &project_board_rel);
        assert!(project_board_mutation_input.type_name == "ProjectBoardCreateMutationInput");
        // properties
        let project_board_props = project_board_mutation_input.props.get("props");
        assert!(project_board_props.is_none());
        // dst
        let project_board_dst = project_board_mutation_input.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Input);
        assert!(project_board_dst.type_name == "ProjectBoardNodesMutationInputUnion");
        assert!(project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_change_input_name() {
        let project_type = mock_project_type();
        let project_issues_rel = project_type.rels().find(|&r| r.name() == "issues").unwrap();
        assert!(
            fmt_rel_change_input_name(&project_type, &project_issues_rel)
                == "ProjectIssuesChangeInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_change_input() {
        /*
            input ProjectIssuesChangeInput {
                $ADD: ProjectIssuesCreateMutationInput
                $UPDATE: ProjectIssuesUpdateInput
                $DELETE: ProjectIssuesDeleteInput
            }
        */
        let project_type = mock_project_type();
        let project_issues_rel = project_type.rels().find(|&r| r.name() == "issues").unwrap();
        let project_issues_change_input =
            generate_rel_change_input(&project_type, &project_issues_rel);
        assert!(project_issues_change_input.type_name == "ProjectIssuesChangeInput");
        // $ADD
        let project_issues_add = project_issues_change_input.props.get("$ADD").unwrap();
        assert!(project_issues_add.name == "$ADD");
        assert!(project_issues_add.kind == PropertyKind::Input);
        assert!(project_issues_add.type_name == "ProjectIssuesCreateMutationInput");
        assert!(!project_issues_add.required);
        assert!(!project_issues_add.list);
        assert!(project_issues_add.arguments.is_empty());
        // $UPDATE
        let project_issues_update = project_issues_change_input.props.get("$UPDATE").unwrap();
        assert!(project_issues_update.name == "$UPDATE");
        assert!(project_issues_update.kind == PropertyKind::Input);
        assert!(project_issues_update.type_name == "ProjectIssuesUpdateInput");
        assert!(!project_issues_update.required);
        assert!(!project_issues_update.list);
        assert!(project_issues_update.arguments.is_empty());
        // $DELETE
        let project_issues_delete = project_issues_change_input.props.get("$DELETE").unwrap();
        assert!(project_issues_delete.name == "$DELETE");
        assert!(project_issues_delete.kind == PropertyKind::Input);
        assert!(project_issues_delete.type_name == "ProjectIssuesDeleteInput");
        assert!(!project_issues_delete.required);
        assert!(!project_issues_delete.list);
        assert!(project_issues_delete.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_update_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerUpdateMutationInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_update_mutation_input() {
        /*
            input ProjectOwnerUpdateMutationInput {
                props: ProjectOwnerPropsInput
                src: ProjectOwnerSrcUpdateMutationInput
                dst: ProjectOwnerDstUpdateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_update_mutation_input =
            generate_rel_update_mutation_input(&project_type, &project_owner_rel);
        assert!(project_owner_update_mutation_input.type_name == "ProjectOwnerUpdateMutationInput");
        // properties
        let props = project_owner_update_mutation_input
            .props
            .get("props")
            .unwrap();
        assert!(props.name == "props");
        assert!(props.kind == PropertyKind::Input);
        assert!(props.type_name == "ProjectOwnerPropsInput");
        assert!(!props.required);
        assert!(!props.list);
        assert!(props.arguments.is_empty());
        // src
        let src = project_owner_update_mutation_input
            .props
            .get("src")
            .unwrap();
        assert!(src.name == "src");
        assert!(src.kind == PropertyKind::Input);
        assert!(src.type_name == "ProjectOwnerSrcUpdateMutationInput");
        assert!(!src.required);
        assert!(!src.list);
        assert!(src.arguments.is_empty());
        // dst
        let dst = project_owner_update_mutation_input
            .props
            .get("dst")
            .unwrap();
        assert!(dst.name == "dst");
        assert!(dst.kind == PropertyKind::Input);
        assert!(dst.type_name == "ProjectOwnerDstUpdateMutationInput");
        assert!(!dst.required);
        assert!(!dst.list);
        assert!(dst.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_src_update_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_src_update_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerSrcUpdateMutationInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_src_update_mutation_input() {
        let project_type = mock_project_type();
        /*
            input ProjectOwnerSrcUpdateMutationInput {
                Project: ProjectUpdateMutationInput
            }
        */
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_src_update_mutation_input =
            generate_rel_src_update_mutation_input(&project_type, &project_owner_rel);
        assert!(
            project_owner_src_update_mutation_input.type_name
                == "ProjectOwnerSrcUpdateMutationInput"
        );
        let project = project_owner_src_update_mutation_input
            .props
            .get("Project")
            .unwrap();
        assert!(project.name == "Project");
        assert!(project.kind == PropertyKind::Input);
        assert!(project.type_name == "ProjectUpdateMutationInput");
        assert!(!project.required);
        assert!(!project.list);
        assert!(project.arguments.is_empty());
        /*
            input ProjectIssuesSrcUpdateMutationInput {
                Project: ProjectUpdateMutationInput
            }
        */
        let project_issues_rel = project_type.rels().find(|&r| r.name() == "issues").unwrap();
        let project_issues_src_update_mutation_input =
            generate_rel_src_update_mutation_input(&project_type, &project_issues_rel);
        assert!(
            project_issues_src_update_mutation_input.type_name
                == "ProjectIssuesSrcUpdateMutationInput"
        );
        let project2 = project_issues_src_update_mutation_input
            .props
            .get("Project")
            .unwrap();
        assert!(project2.name == "Project");
        assert!(project2.kind == PropertyKind::Input);
        assert!(project2.type_name == "ProjectUpdateMutationInput");
        assert!(!project2.required);
        assert!(!project2.list);
        assert!(project2.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_dst_update_mutation_input_name() {
        let project_type = mock_project_type();
        /*
            input ProjectOwnerDstUpdateMutationInput {
                User: UserUpdateMutationInput
            }
        */
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_dst_update_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDstUpdateMutationInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_dst_update_mutation_input() {
        let project_type = mock_project_type();
        /*
            input ProjectOwnerDstUpdateMutationInput {
                User: UserUpdateMutationInput
            }
        */
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_dst_update_mutation_input =
            generate_rel_dst_update_mutation_input(&project_type, &project_owner_rel);
        assert!(
            project_owner_dst_update_mutation_input.type_name
                == "ProjectOwnerDstUpdateMutationInput"
        );
        let user = project_owner_dst_update_mutation_input
            .props
            .get("User")
            .unwrap();
        assert!(user.name() == "User");
        assert!(user.kind == PropertyKind::Input);
        assert!(user.type_name == "UserUpdateMutationInput");
        assert!(!user.required);
        assert!(!user.list());
        assert!(user.arguments.is_empty());
        /*
            input ProjectIssuesDstUpdateMutationInput {
                Bug: BugUpdateMutationInput
                Feature: FeatureUpdateMutationInput
            }
        */
        let project_issues_rel = project_type.rels().find(|&r| r.name() == "issues").unwrap();
        let project_issues_dst_update_mutation_input =
            generate_rel_dst_update_mutation_input(&project_type, &project_issues_rel);
        assert!(
            project_issues_dst_update_mutation_input.type_name
                == "ProjectIssuesDstUpdateMutationInput"
        );
        let bug = project_issues_dst_update_mutation_input
            .props
            .get("Bug")
            .unwrap();
        assert!(bug.name == "Bug");
        assert!(bug.kind == PropertyKind::Input);
        assert!(bug.type_name == "BugUpdateMutationInput");
        assert!(!bug.required);
        assert!(!bug.list);
        assert!(bug.arguments.is_empty());
        let feature = project_issues_dst_update_mutation_input
            .props
            .get("Feature")
            .unwrap();
        assert!(feature.name == "Feature");
        assert!(feature.kind == PropertyKind::Input);
        assert!(feature.type_name == "FeatureUpdateMutationInput");
        assert!(!feature.required);
        assert!(!feature.list);
        assert!(feature.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_props_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_props_input_name(&project_type, &project_owner_rel) == "ProjectOwnerPropsInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_props_input() {
        /*
            input ProjectOwnerPropsInput {
                since: String
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_props_input = generate_rel_props_input(&project_type, &project_owner_rel);
        assert!(project_owner_props_input.type_name == "ProjectOwnerPropsInput");
        assert!(project_owner_props_input.type_kind == TypeKind::Input);
        assert!(project_owner_props_input.props.len() == 1);
        let project_owner_since = project_owner_props_input.props.get("since").unwrap();
        assert!(project_owner_since.name == "since");
        assert!(project_owner_since.kind == PropertyKind::Scalar);
        assert!(project_owner_since.type_name == "String");
        assert!(!project_owner_since.required);
        assert!(!project_owner_since.list);
        assert!(project_owner_since.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_src_query_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_src_query_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerSrcQueryInput"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_src_query_input_name(&project_type, &project_board_rel)
                == "ProjectBoardSrcQueryInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_dst_query_input_name_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_dst_query_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDstQueryInput"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_dst_query_input_name(&project_type, &project_board_rel)
                == "ProjectBoardDstQueryInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_dst_query_input() {
        /*
            input ProjectOwnerNodesQueryInputUnion {
                User: UserQueryInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_nodes_query_input_union =
            generate_rel_dst_query_input(&project_type, &project_owner_rel);
        assert!(project_owner_nodes_query_input_union.type_name == "ProjectOwnerDstQueryInput");
        assert!(project_owner_nodes_query_input_union.type_kind == TypeKind::Input);
        assert!(project_owner_nodes_query_input_union.props.len() == 1);
        let user_input = project_owner_nodes_query_input_union
            .props
            .get("User")
            .unwrap();
        assert!(user_input.name == "User");
        assert!(user_input.kind == PropertyKind::Input);
        assert!(user_input.type_name == "UserQueryInput");
        assert!(!user_input.required());
        assert!(!user_input.list);
        assert!(user_input.arguments.is_empty());
        /*
            input ProjectBoardNodesQueryInputUnion {
                KanbanBoard: KanbanBoardQueryInput
                ScrumBoard: ScrumBoardQueryInput
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_nodes_query_input_union =
            generate_rel_dst_query_input(&project_type, &project_board_rel);
        assert!(project_board_nodes_query_input_union.type_name == "ProjectBoardDstQueryInput");
        assert!(project_board_nodes_query_input_union.type_kind == TypeKind::Input);
        assert!(project_board_nodes_query_input_union.props.len() == 2);
        let kanbanboard_input = project_board_nodes_query_input_union
            .props
            .get("KanbanBoard")
            .unwrap();
        assert!(kanbanboard_input.name == "KanbanBoard");
        assert!(kanbanboard_input.kind == PropertyKind::Input);
        assert!(kanbanboard_input.type_name == "KanbanBoardQueryInput");
        assert!(!kanbanboard_input.required());
        assert!(!kanbanboard_input.list);
        assert!(kanbanboard_input.arguments.is_empty());
        let scrumboard_input = project_board_nodes_query_input_union
            .props
            .get("ScrumBoard")
            .unwrap();
        assert!(scrumboard_input.name == "ScrumBoard");
        assert!(scrumboard_input.kind == PropertyKind::Input);
        assert!(scrumboard_input.type_name == "ScrumBoardQueryInput");
        assert!(!scrumboard_input.required());
        assert!(!scrumboard_input.list);
        assert!(scrumboard_input.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_nodes_mutation_input_union_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_nodes_mutation_input_union_name(&project_type, &project_owner_rel)
                == "ProjectOwnerNodesMutationInputUnion"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_nodes_mutation_input_union_name(&project_type, &project_board_rel)
                == "ProjectBoardNodesMutationInputUnion"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_nodes_mutation_input_union() {
        /*
            input ProjectOwnerNodesMutationInputUnion {
                User: UserInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_nodes_mutation_input_union =
            generate_rel_nodes_mutation_input_union(&project_type, &project_owner_rel);
        assert!(
            project_owner_nodes_mutation_input_union.type_name
                == "ProjectOwnerNodesMutationInputUnion"
        );
        assert!(project_owner_nodes_mutation_input_union.type_kind == TypeKind::Input);
        assert!(project_owner_nodes_mutation_input_union.props.len() == 1);
        let user_input = project_owner_nodes_mutation_input_union
            .props
            .get("User")
            .unwrap();
        assert!(user_input.name == "User");
        assert!(user_input.kind == PropertyKind::Input);
        assert!(user_input.type_name == "UserInput");
        assert!(!user_input.required());
        assert!(!user_input.list);
        assert!(user_input.arguments.is_empty());
        /*
            input ProjectBoardNodesQueryInputUnion {
                KanbanBoard: KanbanBoardInput
                ScrumBoard: ScrumBoardInput
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_nodes_mutation_input_union =
            generate_rel_nodes_mutation_input_union(&project_type, &project_board_rel);
        assert!(
            project_board_nodes_mutation_input_union.type_name
                == "ProjectBoardNodesMutationInputUnion"
        );
        assert!(project_board_nodes_mutation_input_union.type_kind == TypeKind::Input);
        assert!(project_board_nodes_mutation_input_union.props.len() == 2);
        let kanbanboard_input = project_board_nodes_mutation_input_union
            .props
            .get("KanbanBoard")
            .unwrap();
        assert!(kanbanboard_input.name == "KanbanBoard");
        assert!(kanbanboard_input.kind == PropertyKind::Input);
        assert!(kanbanboard_input.type_name == "KanbanBoardInput");
        assert!(!kanbanboard_input.required());
        assert!(!kanbanboard_input.list);
        assert!(kanbanboard_input.arguments.is_empty());
        let scrumboard_input = project_board_nodes_mutation_input_union
            .props
            .get("ScrumBoard")
            .unwrap();
        assert!(scrumboard_input.name == "ScrumBoard");
        assert!(scrumboard_input.kind == PropertyKind::Input);
        assert!(scrumboard_input.type_name == "ScrumBoardInput");
        assert!(!scrumboard_input.required());
        assert!(!scrumboard_input.list);
        assert!(scrumboard_input.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_create_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerCreateInput"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_create_input_name(&project_type, &project_board_rel)
                == "ProjectBoardCreateInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_rel_create_input() {
        /*
            input ProjectOwnerCreateInput {
                $MATCH: ProjectQueryInput
                create: ProjectOwnerCreateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_create_input =
            generate_rel_create_input(&project_type, &project_owner_rel);
        assert!(project_owner_create_input.type_name == "ProjectOwnerCreateInput");
        assert!(project_owner_create_input.type_kind == TypeKind::Input);
        assert!(project_owner_create_input.props.len() == 2);
        let project_owner_match = project_owner_create_input.props.get("$MATCH").unwrap();
        assert!(project_owner_match.name == "$MATCH");
        assert!(project_owner_match.kind == PropertyKind::Input);
        assert!(project_owner_match.type_name == "ProjectQueryInput");
        assert!(!project_owner_match.required);
        assert!(!project_owner_match.list);
        assert!(project_owner_match.arguments.is_empty());
        let project_owner_create = project_owner_create_input.props.get("$CREATE").unwrap();
        assert!(project_owner_create.name == "$CREATE");
        assert!(project_owner_create.kind == PropertyKind::Input);
        assert!(project_owner_create.type_name == "ProjectOwnerCreateMutationInput");
        assert!(!project_owner_create.required);
        assert!(!project_owner_create.list);
        assert!(project_owner_create.arguments.is_empty());
        /*
            input ProjectBoardCreateInput {
                $MATCH: ProjectQueryInput
                create: ProjectBoardCreateMutationInput
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_create_input =
            generate_rel_create_input(&project_type, &project_board_rel);
        assert!(project_board_create_input.type_name == "ProjectBoardCreateInput");
        assert!(project_board_create_input.type_kind == TypeKind::Input);
        assert!(project_board_create_input.props.len() == 2);
        let project_board_match = project_board_create_input.props.get("$MATCH").unwrap();
        assert!(project_board_match.name == "$MATCH");
        assert!(project_board_match.kind == PropertyKind::Input);
        assert!(project_board_match.type_name == "ProjectQueryInput");
        assert!(!project_board_match.required);
        assert!(!project_board_match.list);
        assert!(project_board_match.arguments.is_empty());
        let project_board_create = project_board_create_input.props.get("$CREATE").unwrap();
        assert!(project_board_create.name == "$CREATE");
        assert!(project_board_create.kind == PropertyKind::Input);
        assert!(project_board_create.type_name == "ProjectBoardCreateMutationInput");
        assert!(!project_board_create.required);
        assert!(!project_board_create.list);
        assert!(project_board_create.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_update_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerUpdateInput"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_update_input_name(&project_type, &project_board_rel)
                == "ProjectBoardUpdateInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_rel_update_input() {
        /*
            input ProjectOwnerUpdateInput {
                $MATCH: ProjectOwnerQueryInput
                update: ProjectOwnerUpdateMutationInput!
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_update_input =
            generate_rel_update_input(&project_type, &project_owner_rel);
        assert!(project_owner_update_input.type_name == "ProjectOwnerUpdateInput");
        assert!(project_owner_update_input.type_kind == TypeKind::Input);
        assert!(project_owner_update_input.props.len() == 2);
        let project_owner_match = project_owner_update_input.props.get("$MATCH").unwrap();
        assert!(project_owner_match.name == "$MATCH");
        assert!(project_owner_match.kind == PropertyKind::Input);
        assert!(project_owner_match.type_name == "ProjectOwnerQueryInput");
        assert!(!project_owner_match.required);
        assert!(!project_owner_match.list);
        assert!(project_owner_match.arguments.is_empty());
        let project_owner_update = project_owner_update_input.props.get("$SET").unwrap();
        assert!(project_owner_update.name == "$SET");
        assert!(project_owner_update.kind == PropertyKind::Input);
        assert!(project_owner_update.type_name == "ProjectOwnerUpdateMutationInput");
        assert!(project_owner_update.required);
        assert!(!project_owner_update.list);
        assert!(project_owner_update.arguments.is_empty());
        /*
            input ProjectBoardUpdateInput {
                $MATCH: ProjectBoardQueryInput
                update: ProjectBoardUpdateMutationInput!
            }
        */
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        let project_board_update_input =
            generate_rel_update_input(&project_type, &project_board_rel);
        assert!(project_board_update_input.type_name == "ProjectBoardUpdateInput");
        assert!(project_board_update_input.type_kind == TypeKind::Input);
        assert!(project_board_update_input.props.len() == 2);
        let project_board_match = project_board_update_input.props.get("$MATCH").unwrap();
        assert!(project_board_match.name == "$MATCH");
        assert!(project_board_match.kind == PropertyKind::Input);
        assert!(project_board_match.type_name == "ProjectBoardQueryInput");
        assert!(!project_board_match.required);
        assert!(!project_board_match.list);
        assert!(project_board_match.arguments.is_empty());
        let project_board_update = project_board_update_input.props.get("$SET").unwrap();
        assert!(project_board_update.name == "$SET");
        assert!(project_board_update.kind == PropertyKind::Input);
        assert!(project_board_update.type_name == "ProjectBoardUpdateMutationInput");
        assert!(project_board_update.required);
        assert!(!project_board_update.list);
        assert!(project_board_update.arguments.is_empty());
    }

    #[test]
    fn test_fmt_rel_delete_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_delete_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDeleteInput"
        );
    }

    #[test]
    fn test_generate_rel_delete_input() {
        /*
        input ProjectOwnerDeleteInput {
            $MATCH: ProjectOwnerQueryInput
            src: ProjectOwnerSrcMutationInput
            dst: ProjectOwnerDstDeleteMutationInput
        }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_delete_input =
            generate_rel_delete_input(&project_type, &project_owner_rel);
        assert!(project_owner_delete_input.type_name == "ProjectOwnerDeleteInput");
        let pmatch = project_owner_delete_input.props.get("$MATCH").unwrap();
        assert!(pmatch.name == "$MATCH");
        assert!(pmatch.kind == PropertyKind::Input);
        assert!(pmatch.type_name == "ProjectOwnerQueryInput");
        assert!(!pmatch.required);
        assert!(!pmatch.list);
        assert!(pmatch.arguments.is_empty());
        let src = project_owner_delete_input.props.get("src").unwrap();
        assert!(src.name == "src");
        assert!(src.kind == PropertyKind::Input);
        assert!(src.type_name == "ProjectOwnerSrcDeleteMutationInput");
        assert!(!src.required);
        assert!(!src.list);
        assert!(src.arguments.is_empty());
        let dst = project_owner_delete_input.props.get("dst").unwrap();
        assert!(dst.name == "dst");
        assert!(dst.kind == PropertyKind::Input);
        assert!(dst.type_name == "ProjectOwnerDstDeleteMutationInput");
        assert!(!dst.required);
        assert!(!dst.list);
        assert!(dst.arguments.is_empty());
    }

    #[test]
    fn test_fmt_rel_src_delete_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_src_delete_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerSrcDeleteMutationInput"
        );
    }

    #[test]
    fn test_generate_rel_src_delete_mutation_input() {
        /*
        input ProjectOwnerSrcDeleteMutationInput {
            Project: ProjectDeleteMutationInput
        }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_src_delete_mutation_input =
            generate_rel_src_delete_mutation_input(&project_type, &project_owner_rel);
        assert!(
            project_owner_src_delete_mutation_input.type_name
                == "ProjectOwnerSrcDeleteMutationInput"
        );
        assert!(project_owner_src_delete_mutation_input.props.len() == 1);
        let project = project_owner_src_delete_mutation_input
            .props
            .get("Project")
            .unwrap();
        assert!(project.name == "Project");
        assert!(project.kind == PropertyKind::Input);
        assert!(project.type_name == "ProjectDeleteMutationInput");
        assert!(!project.required);
        assert!(!project.list);
        assert!(project.arguments.is_empty());
    }

    #[test]
    fn test_fmt_rel_dst_delete_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_dst_delete_mutation_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDstDeleteMutationInput"
        );
    }

    #[test]
    fn test_generate_rel_dst_delete_mutation_input() {
        /*
        input ProjectOwnerDstDeleteMutationInput {
            User: UserDeleteMutationInput
        }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_dst_delete_mutation_input =
            generate_rel_dst_delete_mutation_input(&project_type, &project_owner_rel);
        assert!(
            project_owner_dst_delete_mutation_input.type_name
                == "ProjectOwnerDstDeleteMutationInput"
        );
        assert!(project_owner_dst_delete_mutation_input.props.len() == 1);
        let user = project_owner_dst_delete_mutation_input
            .props
            .get("User")
            .unwrap();
        assert!(user.name() == "User");
        assert!(user.kind == PropertyKind::Input);
        assert!(user.type_name == "UserDeleteMutationInput");
        assert!(!user.required);
        assert!(!user.list());
        assert!(user.arguments.is_empty());

        /*
        input ProjectIssuesDstDeleteMutationInput {
            Bug: BugDeleteMutationInput
            Feature: FeatureDeleteMutationInput
        }
        */
        let project_issues_rel = project_type.rels().find(|&r| r.name() == "issues").unwrap();
        let project_issues_dst_delete_mutation_input =
            generate_rel_dst_delete_mutation_input(&project_type, &project_issues_rel);
        assert!(
            project_issues_dst_delete_mutation_input.type_name
                == "ProjectIssuesDstDeleteMutationInput"
        );
        assert!(project_issues_dst_delete_mutation_input.props.len() == 2);
        let bug = project_issues_dst_delete_mutation_input
            .props
            .get("Bug")
            .unwrap();
        assert!(bug.name == "Bug");
        assert!(bug.kind == PropertyKind::Input);
        assert!(bug.type_name == "BugDeleteMutationInput");
        assert!(!bug.required);
        assert!(!bug.list);
        assert!(bug.arguments.is_empty());
        let feature = project_issues_dst_delete_mutation_input
            .props
            .get("Feature")
            .unwrap();
        assert!(feature.name == "Feature");
        assert!(feature.kind == PropertyKind::Input);
        assert!(feature.type_name == "FeatureDeleteMutationInput");
        assert!(!feature.required);
        assert!(!feature.list);
        assert!(feature.arguments.is_empty());
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_read_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(fmt_rel_read_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwner");
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(fmt_rel_read_endpoint_name(&project_type, &project_board_rel) == "ProjectBoard");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_read_endpoint() {
        /*
            ProjectOwner(input: ProjectOwnerQueryInput): [ProjectOwnerRel]
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_read_endpoint =
            generate_rel_read_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_read_endpoint.name == "ProjectOwner");
        assert!(match &project_owner_read_endpoint.kind {
            PropertyKind::Rel { rel_name } => rel_name == "owner",
            _ => false,
        });
        assert!(project_owner_read_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_read_endpoint.required);
        assert!(project_owner_read_endpoint.list);
        assert!(project_owner_read_endpoint.arguments.contains_key("input"));
        if let Some(input) = project_owner_read_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "ProjectOwnerQueryInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_create_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerCreate"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_create_endpoint_name(&project_type, &project_board_rel) == "ProjectBoardCreate"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_create_endpoint() {
        /*
            ProjectOwnerCreate(input: ProjectOwnerCreateInput): ProjectOwnerRel
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_create_endpoint =
            generate_rel_create_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_create_endpoint.name == "ProjectOwnerCreate");
        assert!(match &project_owner_create_endpoint.kind {
            PropertyKind::RelCreateMutation {
                src_label,
                rel_name,
            } => src_label == "Project" && rel_name == "owner",
            _ => false,
        });
        assert!(project_owner_create_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_create_endpoint.required);
        assert!(project_owner_create_endpoint.list);
        assert!(project_owner_create_endpoint
            .arguments
            .contains_key("input"));
        if let Some(input) = project_owner_create_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectOwnerCreateInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_update_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerUpdate"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_update_endpoint_name(&project_type, &project_board_rel) == "ProjectBoardUpdate"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_update_endpoint() {
        /*
            ProjectOwnerUpdate(input: ProjectOwnerUpdateInput): [ProjectOwnerRel]
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_update_endpoint =
            generate_rel_update_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_update_endpoint.name == "ProjectOwnerUpdate");
        assert!(match &project_owner_update_endpoint.kind {
            PropertyKind::RelUpdateMutation {
                src_label,
                rel_name,
            } => src_label == "Project" && rel_name == "owner",
            _ => false,
        });
        assert!(project_owner_update_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_update_endpoint.required);
        assert!(project_owner_update_endpoint.list);
        assert!(project_owner_update_endpoint
            .arguments
            .contains_key("input"));
        if let Some(input) = project_owner_update_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectOwnerUpdateInput");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_delete_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        assert!(
            fmt_rel_delete_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerDelete"
        );
        let project_board_rel = project_type.rels().find(|&r| r.name() == "board").unwrap();
        assert!(
            fmt_rel_delete_endpoint_name(&project_type, &project_board_rel) == "ProjectBoardDelete"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_delete_endpoint() {
        /*
            ProjectOwnerDelete (input: ProjectOwnerDeleteInput): [Project]
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type.rels().find(|&r| r.name() == "owner").unwrap();
        let project_owner_delete_endpoint =
            generate_rel_delete_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_delete_endpoint.name == "ProjectOwnerDelete");
        assert!(match &project_owner_delete_endpoint.kind {
            PropertyKind::RelDeleteMutation {
                src_label,
                rel_name,
            } => src_label == "Project" && rel_name == "owner",
            _ => false,
        });
        assert!(project_owner_delete_endpoint.type_name == "Int");
        assert!(!project_owner_delete_endpoint.required);
        assert!(!project_owner_delete_endpoint.list);
        assert!(project_owner_delete_endpoint
            .arguments
            .contains_key("input"));
        if let Some(input) = project_owner_delete_endpoint.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "ProjectOwnerDeleteInput");
        }
    }

    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_generate_custom_endpoint() {
        /*
            RegisterUsers(input: [UserCreateMutationInput]): [User]
        */
        let e1 = mock_endpoint_one();
        let e1_object = generate_custom_endpoint(&e1);
        assert!(e1_object.name == "RegisterUsers");
        assert!(e1_object.kind == PropertyKind::CustomResolver);
        assert!(e1_object.type_name == "User");
        assert!(e1_object.required);
        assert!(e1_object.list);
        assert!(e1_object.arguments.contains_key("input"));
        if let Some(input) = e1_object.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "UserCreateMutationInput");
        }
        /*
            DisableUser(input: UserQueryInput): User
        */
        let e2 = mock_endpoint_two();
        let e2_object = generate_custom_endpoint(&e2);
        assert!(e2_object.name == "DisableUser");
        assert!(e2_object.kind == PropertyKind::CustomResolver);
        assert!(e2_object.type_name == "User");
        assert!(e2_object.required);
        assert!(!e2_object.list);
        assert!(e2_object.arguments.contains_key("input"));
        if let Some(input) = e2_object.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Required);
            assert!(input.type_name == "UserQueryInput");
        }
        /*
            ComputeBurndown(input: BurndownFilter): BurndownMetrics
        */
        let e3 = mock_endpoint_three();
        let e3_object = generate_custom_endpoint(&e3);
        assert!(e3_object.name == "ComputeBurndown");
        assert!(e3_object.kind == PropertyKind::CustomResolver);
        assert!(e3_object.type_name == "BurndownMetrics");
        assert!(e3_object.required);
        assert!(!e3_object.list);
        assert!(e3_object.arguments.contains_key("input"));
        if let Some(input) = e3_object.arguments.get("input") {
            assert!(input.name == "input");
            assert!(input.kind == ArgumentKind::Optional);
            assert!(input.type_name == "BurndownFilter");
        }
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_schema() {
        let config = mock_config();
        let schema = generate_schema(&config);
        //assert!(schema.len() == 79);
        assert!(schema.contains_key("Project"));
        assert!(schema.contains_key("ProjectQueryInput"));
        assert!(schema.contains_key("ProjectCreateMutationInput"));
        assert!(schema.contains_key("ProjectUpdateMutationInput"));
        assert!(schema.contains_key("ProjectInput"));
        assert!(schema.contains_key("ProjectUpdateInput"));
        assert!(schema.contains_key("ProjectOwnerRel"));
        assert!(schema.contains_key("ProjectOwnerProps"));
        assert!(schema.contains_key("Query"));
        assert!(schema.contains_key("Mutation"));
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_wg_type_endpoints_filter() {
        let config = mock_endpoints_filter();
        let schema = generate_schema(&config);
        let query = schema.get("Query").unwrap();
        let mutation = schema.get("Mutation").unwrap();
        assert!(query.props.len() == 1);
        assert!(mutation.props.len() == 1);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_wg_rels_endpoints_filter() {}

    /// Passes if the root node is created
    #[test]
    fn test_create_root_node() {
        let config = mock_config();
        let root_node = create_root_node::<()>(&config);
        assert!(root_node.is_ok());
    }

    /// Passes if a broken reference creates an error
    #[test]
    fn type_lookup_error() {
        let config = mock_project_config();
        let root_node = create_root_node::<()>(&config);
        assert!(root_node.is_err());
    }

    /// Passes if Info implements the Send trait
    #[test]
    fn test_info_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Info>();
    }

    /// Passes if Info implements the Sync trait
    #[test]
    fn test_info_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Info>();
    }
}
