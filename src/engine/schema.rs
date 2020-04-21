//! This module provides the GraphQL service implementation. It generates
//! the Juniper root node, and its sub-modules provide schema data structures
//! and resolvers for common create, read, update, and delete operations.

use super::config::{
    EndpointClass, GraphqlType, Config, WarpgrapherEndpoint, WarpgrapherProp,
    WarpgrapherRel, WarpgrapherType, WarpgrapherTypeDef,
};
use super::objects::Node;
use crate::engine::context::WarpgrapherRequestContext;
use crate::error::{Error, ErrorKind};
use inflector::Inflector;
use juniper::RootNode;
use serde_json::Map;
use std::collections::HashMap;
use std::fmt::Debug;
use std::panic::catch_unwind;
use std::sync::Arc;

pub type RootRef<GlobalCtx, ReqCtx> =
    Arc<RootNode<'static, Node<GlobalCtx, ReqCtx>, Node<GlobalCtx, ReqCtx>>>;

//#[derive(Debug, PartialEq)]
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputKind {
    Required,
    Optional,
}

//#[derive(Debug, PartialEq)]
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PropertyKind {
    CustomResolver,
    DynamicScalar,
    Input,
    NodeCreateMutation,
    NodeUpdateMutation,
    NodeDeleteMutation(String), // String is node label of the node to be deleted
    Object,
    Rel(String), // String is the name of the rel, which is trivially the field name
    // for rels on objects, but not obvious for root relationship query
    // endpoints
    RelCreateMutation(String, String), // (src_node_label, rel_name)
    RelUpdateMutation(String, String), // (src_node_label, rel_name)
    RelDeleteMutation(String, String), // (src_node_label, rel_name)
    Scalar,
    Union,
    VersionQuery,
}

#[derive(Debug, PartialEq)]
pub enum TypeKind {
    Input,
    Object,
    Rel,
    Union,
}

#[derive(Debug)]
pub struct Info {
    pub name: String,
    pub type_defs: Arc<HashMap<String, NodeType>>,
}

impl Info {
    pub fn new(name: String, type_defs: Arc<HashMap<String, NodeType>>) -> Info {
        Info { name, type_defs }
    }

    pub fn get_type_def(&self) -> Result<&NodeType, Error> {
        self.get_type_def_by_name(&self.name)
    }

    pub fn get_type_def_by_name(&self, name: &str) -> Result<&NodeType, Error> {
        self.type_defs
            .get(name)
            .ok_or_else(|| Error::new(ErrorKind::MissingSchemaElement(self.name.to_owned()), None))
    }
}

#[derive(Debug, PartialEq)]
pub struct NodeType {
    pub props: HashMap<String, Property>,
    pub type_kind: TypeKind,
    pub type_name: String,
    pub union_types: Option<Vec<String>>,
}

impl NodeType {
    pub fn new(
        type_name: String,
        type_kind: TypeKind,
        props: HashMap<String, Property>,
    ) -> NodeType {
        NodeType {
            props,
            type_kind,
            type_name,
            union_types: None,
        }
    }

    pub fn get_prop(&self, field_name: &str) -> Result<&Property, Error> {
        self.props.get(field_name).ok_or_else(|| {
            Error::new(
                ErrorKind::MissingSchemaElement(String::from(&self.type_name) + "::" + field_name),
                None,
            )
        })
    }
    pub fn get_prop_by_type(&self, type_name: &str) -> Result<&Property, Error> {
        self.props
            .iter()
            .find(|(_k, v)| v.type_name == type_name)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::MissingSchemaElement(
                        self.type_name.to_owned() + " property with type of " + type_name,
                    ),
                    None,
                )
            })
            .and_then(|(_k, v)| Ok(v))
    }
}

//#[derive(Debug, PartialEq)]
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Property {
    pub name: String,
    pub kind: PropertyKind,
    pub type_name: String,
    pub required: bool,
    pub list: bool,
    pub input: Option<(InputKind, String)>,
    pub resolver: Option<String>,
    pub validator: Option<String>,
}

impl Property {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        kind: PropertyKind,
        type_name: String,
        required: bool,
        list: bool,
        input: Option<(InputKind, String)>,
        resolver: Option<String>,
        validator: Option<String>,
    ) -> Property {
        Property {
            name,
            kind,
            type_name,
            required,
            list,
            input,
            resolver,
            validator,
        }
    }

    pub fn get_input_type_definition<'i>(&self, info: &'i Info) -> Result<&'i NodeType, Error> {
        if let Some(input_field) = &self.input {
            info.type_defs.get(&input_field.1).ok_or_else(|| {
                Error::new(
                    ErrorKind::MissingSchemaElement(input_field.1.to_owned()),
                    None,
                )
            })
        } else {
            Err(Error::new(
                ErrorKind::MissingSchemaElement(String::from("Input for ") + &self.name),
                None,
            ))
        }
    }
}

/// Takes a vector of WG Props and returns a map of Property structs that
/// represent the property fields in a graphql schema component
fn generate_props(props: &[WarpgrapherProp], id: bool, object: bool) -> HashMap<String, Property> {
    let mut hm = HashMap::new();

    // if the ID field was specified, add it
    if id {
        hm.insert(
            "id".to_owned(),
            Property::new(
                "id".to_owned(),
                PropertyKind::Scalar,
                "ID".to_owned(),
                object,
                false,
                None,
                None,
                None,
            ),
        );
    }

    // insert properties into hashmap
    for p in props {
        match &p.resolver {
            None => {
                hm.insert(
                    p.name.to_owned(),
                    Property::new(
                        p.name.to_owned(),
                        PropertyKind::Scalar,
                        p.type_name.to_owned(),
                        p.required && object,
                        p.list,
                        None,
                        None,
                        p.validator.to_owned(),
                    ),
                );
            }
            Some(r) => {
                hm.insert(
                    p.name.to_owned(),
                    Property::new(
                        p.name.to_owned(),
                        PropertyKind::DynamicScalar,
                        p.type_name.to_owned(),
                        p.required && object,
                        p.list,
                        None,
                        Some(r.to_owned()),
                        p.validator.to_owned(),
                    ),
                );
            }
        };
    }

    hm
}

/// Takes a WG type and returns the name of the corresponding GqlNodeObject.
/// In reality all this is doing is returning the name, but it add value by
/// maintaining consistency with using functions that returned formatted names
/// instead of doing inline string concat
fn fmt_node_object_name(t: &WarpgrapherType) -> String {
    t.name.to_owned()
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
fn generate_node_object(t: &WarpgrapherType) -> NodeType {
    let mut props = generate_props(&t.props, true, true);
    for r in t.rels.clone() {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Rel(r.name.to_owned()),
                fmt_rel_object_name(t, &r),
                false,
                r.list,
                Some((InputKind::Optional, fmt_rel_query_input_name(t, &r))),
                None,
                None,
            ),
        );
    }
    NodeType::new(t.name.to_owned(), TypeKind::Object, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeQueryInput
fn fmt_node_query_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "QueryInput".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeQueryInput
///
/// Format:
/// input GqlNodeQueryInput {
///     id: ID>
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
fn generate_node_query_input(t: &WarpgrapherType) -> NodeType {
    let mut props = generate_props(&t.props, true, false);
    for r in &t.rels {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Input,
                fmt_rel_query_input_name(t, &r),
                false,
                r.list,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(fmt_node_query_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateMutationInput
fn fmt_node_create_mutation_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "CreateMutationInput".to_string())
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
fn generate_node_create_mutation_input(t: &WarpgrapherType) -> NodeType {
    let mut props = generate_props(&t.props, false, false);
    for r in &t.rels {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Input,
                fmt_rel_create_mutation_input_name(t, &r),
                false,
                r.list,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_node_create_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateMutationInput
fn fmt_node_update_mutation_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "UpdateMutationInput".to_string())
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
fn generate_node_update_mutation_input(t: &WarpgrapherType) -> NodeType {
    let mut props = generate_props(&t.props, false, false);
    for r in &t.rels {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Input,
                fmt_rel_change_input_name(t, &r),
                false,
                r.list,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_node_update_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeInput
fn fmt_node_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "Input".to_string())
}

/// Takes a WG type and returns the name of the corresponding GqlNodeInput
///
/// Format:
/// input GqlNodeInput {
///    EXISTING: GqlNodeQueryInput
///    NEW: GqlNodeCreateMutationInput
/// }
///
/// Ex:
/// input ProjectInput {
///     EXISTING: ProjectQueryInput
///     NEW: ProjectMutationInput
/// }
fn generate_node_input(t: &WarpgrapherType) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "EXISTING".to_owned(),
        Property::new(
            "EXISTING".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "NEW".to_owned(),
        Property::new(
            "NEW".to_string(),
            PropertyKind::Input,
            fmt_node_create_mutation_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_node_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeUpdateInput
fn fmt_node_update_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "UpdateInput".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeUpdateInput
///
/// Format:
/// input GqlNodeUpdateInput {
///     match: GqlNodeQueryInput
///     modify: GqlNodeCreateMutationInput
/// }
///
/// Ex:
/// input ProjectUpdateInput {
///     match: ProjectQueryInput
///     modify: ProjectMutationInput
/// }
fn generate_node_update_input(t: &WarpgrapherType) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "match".to_owned(),
        Property::new(
            "match".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "modify".to_owned(),
        Property::new(
            "modify".to_string(),
            PropertyKind::Input,
            fmt_node_update_mutation_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_node_update_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_node_delete_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "DeleteInput".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteInput
///
/// Format:
/// input GqlNodeDeleteInput {
///     match: GqlNodeQueryInput
///     delete: GqlNodeDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectDeleteInput {
///     match: ProjectQueryInput
///     delete: ProjectDeleteMutationInput
/// }
fn generate_node_delete_input(t: &WarpgrapherType) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "match".to_owned(),
        Property::new(
            "match".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "delete".to_owned(),
        Property::new(
            "delete".to_string(),
            PropertyKind::Input,
            fmt_node_delete_mutation_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_node_delete_input_name(t), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteMutationInput
fn fmt_node_delete_mutation_input_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "DeleteMutationInput".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteMutationInput
///
/// Format:
/// input GqlNodeDeleteMutationInput {
///     force: Boolean
///     rel[n]: GqlRelDeleteInput
/// }
///
/// Ex:
/// input ProjectDeleteMutationInput {
///     force: Boolean
///     owner: ProjectOwnerDeleteInput
///     issues: ProjectIssuesDeleteInput
/// }
fn generate_node_delete_mutation_input(t: &WarpgrapherType) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "force".to_owned(),
        Property::new(
            "force".to_string(),
            PropertyKind::Scalar,
            "Boolean".to_string(),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    for r in &t.rels {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Input,
                fmt_rel_delete_input_name(t, &r),
                false,
                r.list,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_node_delete_mutation_input_name(t),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeReadEndpoint
fn fmt_node_read_endpoint_name(t: &WarpgrapherType) -> String {
    t.name.to_owned()
}

/// Takes a WG type and returns a NodeType representing a GqlNodeReadEndpoint
///
/// Format:
/// GqlNodeReadEndpoint(input: <GqlNodeQueryInput>): [<Node>]
///
/// Ex:
/// Project(input: ProjectQueryInput): [Project]
fn generate_node_read_endpoint(t: &WarpgrapherType) -> Property {
    Property::new(
        fmt_node_read_endpoint_name(t),
        PropertyKind::Object,
        t.name.to_owned(),
        false,
        true,
        Some((InputKind::Optional, fmt_node_query_input_name(t))),
        None,
        None,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateEndpoint
fn fmt_node_create_endpoint_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "Create".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeCreateEndpoint
///
/// Format:
/// GqlNodeCreateEndpoint (input: <GqlNodeCreateMutationInput>): <Node>
///
/// Ex:
/// ProjectCreate (input: ProjectCreateMutationInput): Project
fn generate_node_create_endpoint(t: &WarpgrapherType) -> Property {
    Property::new(
        fmt_node_create_endpoint_name(t),
        PropertyKind::NodeCreateMutation,
        t.name.to_owned(),
        false,
        false,
        Some((InputKind::Required, fmt_node_create_mutation_input_name(t))),
        None,
        None,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeCreateEndpoint
fn fmt_node_update_endpoint_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "Update".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeUpdateEndpoint:
///
/// Format:
/// GqlNodeUpdateEndpoint (input: <GqlNodeUpdateInput>): [<Node>]
///
/// Ex:
/// ProjectUpdate (input: ProjectUpdateInput): [Project]
fn generate_node_update_endpoint(t: &WarpgrapherType) -> Property {
    Property::new(
        fmt_node_update_endpoint_name(t),
        PropertyKind::NodeUpdateMutation,
        t.name.to_owned(),
        false,
        true,
        Some((InputKind::Required, fmt_node_update_input_name(t))),
        None,
        None,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteEndpoint
fn fmt_node_delete_endpoint_name(t: &WarpgrapherType) -> String {
    format!("{}{}", t.name.to_owned(), "Delete".to_string())
}

/// Takes a WG type and returns a NodeType representing a GqlNodeDeleteEndpoint
///
/// Format:
/// GqlNodeDeleteEndpoint (input: <GqlNodeQueryInput>): Int
///
/// Ex:
/// ProjectDelete (input: <ProjectQueryInput>): Int
fn generate_node_delete_endpoint(t: &WarpgrapherType) -> Property {
    Property::new(
        fmt_node_delete_endpoint_name(t),
        PropertyKind::NodeDeleteMutation(fmt_node_object_name(t)),
        "Int".to_string(),
        false,
        false,
        Some((InputKind::Required, fmt_node_delete_input_name(t))),
        None,
        None,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelObject
fn fmt_rel_object_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "Rel".to_string()
    )
}

/// Takes a WG rel an returns the name of the rel. In reality, this just makes
/// a copy of the name
fn fmt_rel_name(r: &WarpgrapherRel) -> String {
    r.name.to_owned()
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
fn generate_rel_object(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "id".to_owned(),
        Property::new(
            "id".to_owned(),
            PropertyKind::Scalar,
            "ID".to_owned(),
            true,
            false,
            None,
            None,
            None,
        ),
    );
    if !r.props.is_empty() {
        props.insert(
            "props".to_owned(),
            Property::new(
                "props".to_string(),
                PropertyKind::Object,
                fmt_rel_props_object_name(t, r),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    props.insert(
        "src".to_owned(),
        Property::new(
            "src".to_string(),
            PropertyKind::Object,
            t.name.to_owned(),
            true,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "dst".to_owned(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Union,
            fmt_rel_nodes_union_name(t, r),
            true,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_object_name(t, r), TypeKind::Rel, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelPropsObject
fn fmt_rel_props_object_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "Props".to_string()
    )
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
fn generate_rel_props_object(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    NodeType::new(
        fmt_rel_props_object_name(t, r),
        TypeKind::Object,
        generate_props(&r.props, false, true),
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelNodesUnion
fn fmt_rel_nodes_union_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "NodesUnion".to_string()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelNodesUnion
///
/// Format:
/// union GqlRelNodesUnion = <Node[0]> | <Node[1]>
///
/// Ex:
/// union ProjectIssuesNodesUnion = Feature | Bug
fn generate_rel_nodes_union(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut nt = NodeType::new(
        fmt_rel_nodes_union_name(t, r),
        TypeKind::Union,
        HashMap::new(),
    );
    nt.union_types = Some(r.nodes.clone());
    nt
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelQueryInput
fn fmt_rel_query_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "QueryInput".to_string()
    )
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
fn generate_rel_query_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "id".to_owned(),
        Property::new(
            "id".to_owned(),
            PropertyKind::Scalar,
            "ID".to_owned(),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    if !r.props.is_empty() {
        props.insert(
            "props".to_owned(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    props.insert(
        "src".to_owned(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_query_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "dst".to_owned(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_query_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateMutationInput
fn fmt_rel_create_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "CreateMutationInput".to_string()
    )
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
fn generate_rel_create_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    if !r.props.is_empty() {
        props.insert(
            "props".to_owned(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    props.insert(
        "dst".to_owned(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_nodes_mutation_input_union_name(t, r),
            true,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(
        fmt_rel_create_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateMutationInput
fn fmt_rel_change_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "ChangeInput".to_string()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelChangeInput
///
/// Format:
/// input GqlRelChangeInput {
///     ADD: GqlRelCreateMutationInput
///     UPDATE: GqlRelUpdateMutationInput
///     DELETE: GqlRelDeleteInput
/// }
///
/// Ex:
/// input ProjectIssuesChangeInput {
///     ADD: ProjectIssuesCreateMutationInput
///     UPDATE: ProjectIssuesUpdateInput
///     DELETE: ProjectIssuesDeleteInput
/// }
fn generate_rel_change_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "ADD".to_owned(),
        Property::new(
            "ADD".to_string(),
            PropertyKind::Input,
            fmt_rel_create_mutation_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "UPDATE".to_owned(),
        Property::new(
            "UPDATE".to_string(),
            PropertyKind::Input,
            fmt_rel_update_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "DELETE".to_owned(),
        Property::new(
            "DELETE".to_string(),
            PropertyKind::Input,
            fmt_rel_delete_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_change_input_name(t, r), TypeKind::Input, props)
}
/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateMutationInput
fn fmt_rel_update_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "UpdateMutationInput".to_string()
    )
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
fn generate_rel_update_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    if !r.props.is_empty() {
        props.insert(
            "props".to_owned(),
            Property::new(
                "props".to_string(),
                PropertyKind::Input,
                fmt_rel_props_input_name(t, r),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    props.insert(
        "src".to_owned(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_update_mutation_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "dst".to_owned(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_update_mutation_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(
        fmt_rel_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelSrcUpdateMutationInput
fn fmt_rel_src_update_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "SrcUpdateMutationInput".to_string()
    )
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
fn generate_rel_src_update_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name.clone(),
        Property::new(
            t.name.clone(),
            PropertyKind::Input,
            fmt_node_update_mutation_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(
        fmt_rel_src_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDstUpdateMutationInput
fn fmt_rel_dst_update_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "DstUpdateMutationInput".to_string()
    )
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
fn generate_rel_dst_update_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    for node in r.nodes.clone() {
        props.insert(
            node.clone(),
            Property::new(
                node.clone(),
                PropertyKind::Input,
                format!("{}UpdateMutationInput", node.clone()),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_rel_dst_update_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}
/// Takes a WG type and rel and returns the name of the corresponding GqlRelPropsInput
fn fmt_rel_props_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "PropsInput".to_string()
    )
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
fn generate_rel_props_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    NodeType::new(
        fmt_rel_props_input_name(t, r),
        TypeKind::Input,
        generate_props(&r.props, false, false),
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelSrcQueryInput
fn fmt_rel_src_query_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "SrcQueryInput".to_string()
    )
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
fn generate_rel_src_query_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name.clone(),
        Property::new(
            t.name.clone(),
            PropertyKind::Input,
            format!("{}QueryInput", t.name.clone()),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_src_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDstQueryInput
fn fmt_rel_dst_query_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "DstQueryInput".to_string()
    )
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
fn generate_rel_dst_query_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    for node in r.nodes.clone() {
        props.insert(
            node.clone(),
            Property::new(
                node.clone(),
                PropertyKind::Input,
                //fmt_node_query_input_name(t, r),
                format!("{}QueryInput", node.clone()),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(fmt_rel_dst_query_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelNodesMutationInputUnion
fn fmt_rel_nodes_mutation_input_union_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "NodesMutationInputUnion".to_string()
    )
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
fn generate_rel_nodes_mutation_input_union(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    for node in r.nodes.clone() {
        props.insert(
            node.clone(),
            Property::new(
                node.clone(),
                PropertyKind::Input,
                format!("{}Input", node.clone()),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_rel_nodes_mutation_input_union_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateInput
fn fmt_rel_create_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "CreateInput".to_string()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelCreateInput
///
/// Format:
/// input GqlRelCreateInput {
///     match: <GqlNodeQueryInput>
///     create: <GqlRelCreateMutationInput>
/// }
///
/// Ex:
/// input ProjectOwnerCreateInput   {
///     match: ProjectQueryInput
///     create: ProjectOwnerCreateMutationInput
/// }
fn generate_rel_create_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "match".to_owned(),
        Property::new(
            "match".to_string(),
            PropertyKind::Input,
            fmt_node_query_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "create".to_owned(),
        Property::new(
            "create".to_string(),
            PropertyKind::Input,
            fmt_rel_create_mutation_input_name(t, &r),
            false,
            r.list,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_create_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateInput
fn fmt_rel_update_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}{}",
        t.name.to_owned(),
        r.name.to_owned().to_title_case(),
        "UpdateInput".to_string()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelUpdateInput
///
/// Format:
/// input GqlRelUpdateInput {
///     match: GqlRelQueryInput
///     update: GqlRelUpdateMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerUpdateInput   {
///     match: ProjectOwnerQueryInput
///     update: ProjectOwnerUpdateMutationInput
/// }
fn generate_rel_update_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "match".to_owned(),
        Property::new(
            "match".to_string(),
            PropertyKind::Input,
            fmt_rel_query_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "update".to_owned(),
        Property::new(
            "update".to_string(),
            PropertyKind::Input,
            fmt_rel_update_mutation_input_name(t, &r),
            true,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_update_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_rel_delete_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}DeleteInput",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDeleteInput
///
/// Format:
/// input GqlRelDeleteInput {
///    match: GqlRelQueryInput
///    src: GqlRelSrcDeleteMutationInput
///    dst: GqlRelDstDeleteMutationInput
/// }
///
/// Ex:
/// input ProjectOwnerDeleteInput {
///    match: ProjectOwnerQueryInput
///    src: ProjectOwnerSrcDeleteMutationInput
///    dst: ProjectOwnerDstDeleteMutationInput
/// }
fn generate_rel_delete_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        "match".to_owned(),
        Property::new(
            "match".to_string(),
            PropertyKind::Input,
            fmt_rel_query_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "src".to_owned(),
        Property::new(
            "src".to_string(),
            PropertyKind::Input,
            fmt_rel_src_delete_mutation_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    props.insert(
        "dst".to_owned(),
        Property::new(
            "dst".to_string(),
            PropertyKind::Input,
            fmt_rel_dst_delete_mutation_input_name(t, r),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(fmt_rel_delete_input_name(t, r), TypeKind::Input, props)
}

/// Takes a WG type and returns the name of the corresponding GqlRelSrcDeleteMutationInput
fn fmt_rel_src_delete_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}SrcDeleteMutationInput",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
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
fn generate_rel_src_delete_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    props.insert(
        t.name.clone(),
        Property::new(
            t.name.clone(),
            PropertyKind::Input,
            fmt_node_delete_mutation_input_name(t),
            false,
            false,
            None,
            None,
            None,
        ),
    );
    NodeType::new(
        fmt_rel_src_delete_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and returns the name of the corresponding GqlNodeDeleteInput
fn fmt_rel_dst_delete_mutation_input_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}DstDeleteMutationInput",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
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
fn generate_rel_dst_delete_mutation_input(t: &WarpgrapherType, r: &WarpgrapherRel) -> NodeType {
    let mut props = HashMap::new();
    for node in r.nodes.clone() {
        props.insert(
            node.clone(),
            Property::new(
                node.clone(),
                PropertyKind::Input,
                format!("{}DeleteMutationInput", node.clone()),
                false,
                false,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(
        fmt_rel_dst_delete_mutation_input_name(t, r),
        TypeKind::Input,
        props,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelReadEndpoint
fn fmt_rel_read_endpoint_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!("{}{}", t.name.to_owned(), r.name.to_owned().to_title_case())
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelReadEndpoint
///
/// Format:
/// GqlRelReadEndpoint (input: <GqlRelQueryInput>): [<GqlRelObject>]
///
/// Ex:
/// ProjectOwner(input: ProjectOwnerQueryInput): [ProjectOwnerRel]
fn generate_rel_read_endpoint(t: &WarpgrapherType, r: &WarpgrapherRel) -> Property {
    Property::new(
        fmt_rel_read_endpoint_name(t, r),
        PropertyKind::Rel(r.name.to_owned()),
        fmt_rel_object_name(t, r),
        false,
        true,
        Some((InputKind::Optional, fmt_rel_query_input_name(t, r))),
        None,
        None,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelCreateEndpoint
fn fmt_rel_create_endpoint_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}Create",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelCreateEndpoint
///
/// Format:
/// GqlRelCreateEndpoint (input: <GqlRelCreateInput>): <GqlRelObject>
///
/// Ex:
/// ProjectOwnerCreate(input: ProjectOwnerCreateInput): ProjectOwnerRel
fn generate_rel_create_endpoint(t: &WarpgrapherType, r: &WarpgrapherRel) -> Property {
    Property::new(
        fmt_rel_create_endpoint_name(t, r),
        PropertyKind::RelCreateMutation(fmt_node_object_name(t), fmt_rel_name(r)),
        fmt_rel_object_name(t, r),
        false,
        r.list,
        Some((InputKind::Required, fmt_rel_create_input_name(t, r))),
        None,
        None,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelUpdateEndpoint
fn fmt_rel_update_endpoint_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}Update",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelUpdateEndpoint
///
/// Format:
/// GqlRelUpdateEndpoint (input: <GqlRelUpdateInput>): [<GqlRelObject>]
///
/// Ex:
/// ProjectOwnerUpdate(input: ProjectOwnerUpdateInput): ProjectOwnerRel
fn generate_rel_update_endpoint(t: &WarpgrapherType, r: &WarpgrapherRel) -> Property {
    Property::new(
        fmt_rel_update_endpoint_name(t, r),
        PropertyKind::RelUpdateMutation(fmt_node_object_name(t), fmt_rel_name(r)),
        fmt_rel_object_name(t, r),
        false,
        true,
        Some((InputKind::Required, fmt_rel_update_input_name(t, r))),
        None,
        None,
    )
}

/// Takes a WG type and rel and returns the name of the corresponding GqlRelDeleteEndpoint
fn fmt_rel_delete_endpoint_name(t: &WarpgrapherType, r: &WarpgrapherRel) -> String {
    format!(
        "{}{}Delete",
        t.name.to_owned(),
        r.name.to_owned().to_title_case()
    )
}

/// Takes a WG Type and Rel and returns a NodeType representing a GqlRelDeleteEndpoint
///
/// Format:
/// GqlRelDeleteEndpoint (input: <GqlRelQueryInput>): [<Node>]
///
/// Ex:
/// ProjectOwnerDelete(input: ProjectOwnerQueryInput): [Project]
fn generate_rel_delete_endpoint(t: &WarpgrapherType, r: &WarpgrapherRel) -> Property {
    Property::new(
        fmt_rel_delete_endpoint_name(t, r),
        PropertyKind::RelDeleteMutation(fmt_node_object_name(t), fmt_rel_name(r)),
        "Int".to_string(),
        false,
        false,
        Some((InputKind::Required, fmt_rel_delete_input_name(t, r))),
        None,
        None,
    )
}

/// Takes a WG Endpoint and returns a NodeType representing a root endpoint
fn generate_custom_endpoint(e: &WarpgrapherEndpoint) -> Property {
    Property::new(
        e.name.clone(),
        PropertyKind::CustomResolver,
        match &e.output.type_def {
            WarpgrapherTypeDef::Scalar(t) => match &t {
                GraphqlType::Int => "Int".to_string(),
                GraphqlType::Float => "Float".to_string(),
                GraphqlType::String => "String".to_string(),
                GraphqlType::Boolean => "Boolean".to_string(),
            },
            WarpgrapherTypeDef::Existing(s) => s.clone(),
            WarpgrapherTypeDef::Custom(t) => t.name.clone(),
        },
        e.output.required,
        e.output.list,
        match &e.input {
            None => None,
            Some(input) => Some((
                match &input.required {
                    true => InputKind::Required,
                    false => InputKind::Optional,
                },
                match &input.type_def {
                    WarpgrapherTypeDef::Scalar(s) => match &s {
                        GraphqlType::Int => "Int".to_string(),
                        GraphqlType::Float => "Float".to_string(),
                        GraphqlType::String => "String".to_string(),
                        GraphqlType::Boolean => "Boolean".to_string(),
                    },
                    WarpgrapherTypeDef::Existing(e) => e.clone(),
                    WarpgrapherTypeDef::Custom(c) => c.name.clone(),
                },
            )),
        },
        None,
        None,
    )
}

fn generate_custom_endpoint_input(t: &WarpgrapherType) -> NodeType {
    let mut props = generate_props(&t.props, false, false);
    for r in &t.rels {
        props.insert(
            r.name.to_owned(),
            Property::new(
                r.name.to_owned(),
                PropertyKind::Input,
                fmt_rel_query_input_name(t, &r),
                false,
                r.list,
                None,
                None,
                None,
            ),
        );
    }
    NodeType::new(t.name.clone(), TypeKind::Input, props)
}

fn generate_static_version_query() -> Property {
    Property::new(
        "_version".to_owned(),
        PropertyKind::VersionQuery,
        "String".to_owned(),
        false,
        false,
        None,
        None,
        None,
    )
}

/// Takes a WG config and returns a map of graphql schema components for model
/// types, custom endpoints, and associated endpoint types
fn generate_schema(c: &Config) -> HashMap<String, NodeType> {
    let mut nthm = HashMap::new();
    let mut mutation_props = HashMap::new();
    let mut query_props = HashMap::new();

    // generate graphql schema components for warpgrapher types
    for t in &c.model {
        // GqlNodeType
        let node_type = generate_node_object(t);
        nthm.insert(node_type.type_name.to_owned(), node_type);

        // GqlNodeQueryInput
        let node_query_input = generate_node_query_input(t);
        nthm.insert(node_query_input.type_name.to_owned(), node_query_input);

        // GqlNodeCreateMutationInput
        let node_create_mutation_input = generate_node_create_mutation_input(t);
        nthm.insert(
            node_create_mutation_input.type_name.to_owned(),
            node_create_mutation_input,
        );

        // GqlNodeUpdateMutationInput
        let node_update_mutation_input = generate_node_update_mutation_input(t);
        nthm.insert(
            node_update_mutation_input.type_name.to_owned(),
            node_update_mutation_input,
        );

        // GqlNodeInput
        let node_input = generate_node_input(t);
        nthm.insert(node_input.type_name.to_owned(), node_input);

        // GqlNodeUpdateInput
        let node_update_input = generate_node_update_input(t);
        nthm.insert(node_update_input.type_name.to_owned(), node_update_input);

        // GqlNodeDeleteInput
        let node_delete_input = generate_node_delete_input(t);
        nthm.insert(node_delete_input.type_name.to_owned(), node_delete_input);

        // GqlNodeDeleteMutationInput
        let node_delete_mutation_input = generate_node_delete_mutation_input(t);
        nthm.insert(
            node_delete_mutation_input.type_name.to_owned(),
            node_delete_mutation_input,
        );

        // GqlNodeReadEndpoint
        if t.endpoints.read {
            let read_endpoint = generate_node_read_endpoint(t);
            query_props.insert(read_endpoint.name.to_owned(), read_endpoint);
        }

        // GqlNodeCreateEndpoint
        if t.endpoints.create {
            let create_endpoint = generate_node_create_endpoint(t);
            mutation_props.insert(create_endpoint.name.to_owned(), create_endpoint);
        }

        // GqlNodeUpdateEndpoint
        if t.endpoints.update {
            let update_endpoint = generate_node_update_endpoint(t);
            mutation_props.insert(update_endpoint.name.to_owned(), update_endpoint);
        }

        // GqlNodeDeleteEndpoint
        if t.endpoints.delete {
            let delete_endpoint = generate_node_delete_endpoint(t);
            mutation_props.insert(delete_endpoint.name.to_owned(), delete_endpoint);
        }

        for r in &t.rels {
            // GqlRelObject
            let rel_object = generate_rel_object(t, r);
            nthm.insert(rel_object.type_name.to_owned(), rel_object);

            // GqlRelPropsObject
            let rel_props_object = generate_rel_props_object(t, r);
            nthm.insert(rel_props_object.type_name.to_owned(), rel_props_object);

            // GqlRelNodesUnion
            let rel_nodes_union = generate_rel_nodes_union(t, r);
            nthm.insert(rel_nodes_union.type_name.to_owned(), rel_nodes_union);

            // GqlRelQueryInput
            let rel_query_input = generate_rel_query_input(t, r);
            nthm.insert(rel_query_input.type_name.to_owned(), rel_query_input);

            // GqlRelCreateMutationInput
            let rel_create_mutation_input = generate_rel_create_mutation_input(t, r);
            nthm.insert(
                rel_create_mutation_input.type_name.to_owned(),
                rel_create_mutation_input,
            );

            // GqlRelChangeInput
            let rel_change_input = generate_rel_change_input(t, r);
            nthm.insert(rel_change_input.type_name.to_owned(), rel_change_input);

            // GqlRelUpdateMutationInput
            let rel_update_mutation_input = generate_rel_update_mutation_input(t, r);
            nthm.insert(
                rel_update_mutation_input.type_name.to_owned(),
                rel_update_mutation_input,
            );

            // GqlRelSrcUpdateMutationInput
            let rel_src_update_mutation_input = generate_rel_src_update_mutation_input(t, r);
            nthm.insert(
                rel_src_update_mutation_input.type_name.to_owned(),
                rel_src_update_mutation_input,
            );

            // GqlRelDstUpdateMutationInput
            let rel_dst_update_mutation_input = generate_rel_dst_update_mutation_input(t, r);
            nthm.insert(
                rel_dst_update_mutation_input.type_name.to_owned(),
                rel_dst_update_mutation_input,
            );

            // GqlRelPropsInput
            let rel_props_input = generate_rel_props_input(t, r);
            nthm.insert(rel_props_input.type_name.to_owned(), rel_props_input);

            // GqlRelSrcQueryInput
            let rel_src_query_input = generate_rel_src_query_input(t, r);
            nthm.insert(
                rel_src_query_input.type_name.to_owned(),
                rel_src_query_input,
            );

            // GqlRelDstQueryInput
            let rel_dst_query_input = generate_rel_dst_query_input(t, r);
            nthm.insert(
                rel_dst_query_input.type_name.to_owned(),
                rel_dst_query_input,
            );

            // GqlRelNodesMutationInputUnion
            let rel_nodes_mutation_input_union = generate_rel_nodes_mutation_input_union(t, r);
            nthm.insert(
                rel_nodes_mutation_input_union.type_name.to_owned(),
                rel_nodes_mutation_input_union,
            );

            // GqlRelCreateInput
            let rel_create_input = generate_rel_create_input(t, r);
            nthm.insert(rel_create_input.type_name.to_owned(), rel_create_input);

            // GqlRelUpdateInput
            let rel_update_input = generate_rel_update_input(t, r);
            nthm.insert(rel_update_input.type_name.to_owned(), rel_update_input);

            // GqlRelDeleteInput
            let rel_delete_input = generate_rel_delete_input(t, r);
            nthm.insert(rel_delete_input.type_name.to_owned(), rel_delete_input);

            // GqlRelSrcDeleteMutationInput
            let rel_src_delete_mutation_input = generate_rel_src_delete_mutation_input(t, r);
            nthm.insert(
                rel_src_delete_mutation_input.type_name.to_owned(),
                rel_src_delete_mutation_input,
            );

            // GqlRelDstDeleteMutationInput
            let rel_dst_delete_mutation_input = generate_rel_dst_delete_mutation_input(t, r);
            nthm.insert(
                rel_dst_delete_mutation_input.type_name.to_owned(),
                rel_dst_delete_mutation_input,
            );

            // GqlRelReadEndpoint
            if r.endpoints.read {
                let rel_read_endpoint = generate_rel_read_endpoint(t, r);
                query_props.insert(rel_read_endpoint.name.to_owned(), rel_read_endpoint);
            }

            // GqlRelCreateEndpoint
            if r.endpoints.create {
                let rel_create_endpoint = generate_rel_create_endpoint(t, r);
                mutation_props.insert(rel_create_endpoint.name.to_owned(), rel_create_endpoint);
            }

            // GqlRelUpdateEndpoint
            if r.endpoints.update {
                let rel_update_endpoint = generate_rel_update_endpoint(t, r);
                mutation_props.insert(rel_update_endpoint.name.to_owned(), rel_update_endpoint);
            }

            // GqlRelDelete Endpoint
            if r.endpoints.delete {
                let rel_delete_endpoint = generate_rel_delete_endpoint(t, r);
                mutation_props.insert(rel_delete_endpoint.name.to_owned(), rel_delete_endpoint);
            }
        }
    }

    // generate graphql schema components for custom endpoints and associated types
    for e in &c.endpoints {
        // add custom endpoint
        let endpoint = generate_custom_endpoint(e);
        match e.class {
            EndpointClass::Mutation => {
                mutation_props.insert(e.name.clone(), endpoint);
            }
            EndpointClass::Query => {
                query_props.insert(e.name.clone(), endpoint);
            }
        }

        // add custom input type if provided
        if let Some(input) = &e.input {
            if let WarpgrapherTypeDef::Custom(t) = &input.type_def {
                let input = generate_custom_endpoint_input(&t);
                nthm.insert(t.name.clone(), input);
            }
        }

        // add custom output type if provided
        if let WarpgrapherTypeDef::Custom(t) = &e.output.type_def {
            let node_type = generate_node_object(&t);
            nthm.insert(node_type.type_name.to_owned(), node_type);
        }
    }

    // static endpoints
    query_props.insert("_version".to_string(), generate_static_version_query());

    // insert
    nthm.insert(
        "Mutation".to_owned(),
        NodeType::new("Mutation".to_owned(), TypeKind::Object, mutation_props),
    );

    nthm.insert(
        "Query".to_owned(),
        NodeType::new("Query".to_owned(), TypeKind::Object, query_props),
    );

    nthm
}

/// Takes a Warpgrapher configuration and returns the Juniper RootNode for a
/// GraphQL schema that matches the Warpgrapher configuration.
///
/// # Errors
/// Returns an [`Error`] of kind [`CouldNotResolveWarpgrapherType`] if
/// there is an error in the configuration, specifically if the
/// configuration of type A references type B, but type B cannot be found.
///
/// [`Error`]: ../error/struct.Error.html
/// [`CouldNotResolveWarpgrapherType`]: ../error/enum.ErrorKind.html#variant.CouldNotResolveWarpgrapherType
///
pub fn create_root_node<GlobalCtx: Debug, ReqCtx: Debug + WarpgrapherRequestContext>(
    c: &Config,
) -> Result<RootRef<GlobalCtx, ReqCtx>, Error> {
    // Runtime performance could be optimized by generating the entirety of the
    // schema in one loop iteration over the configuration. In fact, that's how
    // the first iteration of the code worked. However, doing so adds code
    // complexity, as all the schema objects built from any given
    // WarpgrapherType are built at once. This implementation opts for clarity
    // over runtime efficiency, given that the number of configuration items
    // is lkely to be small.

    let nthm = generate_schema(c);
    let nts = Arc::new(nthm);
    let root_mutation_info = Info::new("Mutation".to_owned(), nts.clone());
    let root_query_info = Info::new("Query".to_owned(), nts);
    catch_unwind(|| {
        Arc::new(RootNode::new_with_info(
            Node::new("Query".to_owned(), Map::new()),
            Node::new("Mutation".to_owned(), Map::new()),
            root_query_info,
            root_mutation_info,
        ))
    })
    .map_err(|e| {
        e.downcast::<Error>()
            .and_then(|e| Ok(*e))
            .unwrap_or_else(|e| {
                Error::new(ErrorKind::MissingSchemaElement(format!("{:#?}", e)), None)
            })
    })
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
        generate_rel_update_input, generate_rel_update_mutation_input, generate_schema, Info,
        InputKind, NodeType, Property, PropertyKind, TypeKind,
    };
    use crate::engine::config::{
        EndpointClass, Config, WarpgrapherEndpoint, WarpgrapherEndpointType,
        WarpgrapherEndpointsFilter, WarpgrapherProp, WarpgrapherRel, WarpgrapherType,
        WarpgrapherTypeDef,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn mock_project_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "Project".to_string(),
            vec![
                WarpgrapherProp::new(
                    "name".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
                WarpgrapherProp::new(
                    "tags".to_string(),
                    "String".to_string(),
                    false,
                    true,
                    None,
                    None,
                ),
                WarpgrapherProp::new(
                    "public".to_string(),
                    "Boolean".to_string(),
                    true,
                    false,
                    None,
                    None,
                ),
            ],
            vec![
                WarpgrapherRel::new(
                    "owner".to_string(),
                    false,
                    vec!["User".to_string()],
                    vec![WarpgrapherProp::new(
                        "since".to_string(),
                        "String".to_string(),
                        false,
                        false,
                        None,
                        None,
                    )],
                    WarpgrapherEndpointsFilter::all(),
                ),
                WarpgrapherRel::new(
                    "board".to_string(),
                    false,
                    vec!["ScrumBoard".to_string(), "KanbanBoard".to_string()],
                    vec![],
                    WarpgrapherEndpointsFilter::all(),
                ),
                WarpgrapherRel::new(
                    "commits".to_string(),
                    true,
                    vec!["Commit".to_string()],
                    vec![],
                    WarpgrapherEndpointsFilter::all(),
                ),
                WarpgrapherRel::new(
                    "issues".to_string(),
                    true,
                    vec!["Feature".to_string(), "Bug".to_string()],
                    vec![],
                    WarpgrapherEndpointsFilter::all(),
                ),
            ],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_user_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "User".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_kanbanboard_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "KanbanBoard".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_scrumboard_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "ScrumBoard".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_feature_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "Feature".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_bug_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "Bug".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_commit_type() -> WarpgrapherType {
        WarpgrapherType::new(
            "Commit".to_string(),
            vec![WarpgrapherProp::new(
                "name".to_string(),
                "String".to_string(),
                true,
                false,
                None,
                None,
            )],
            vec![],
            WarpgrapherEndpointsFilter::all(),
        )
    }

    fn mock_endpoint_one() -> WarpgrapherEndpoint {
        // RegisterUsers(input: [UserCreateMutationInput]): [User]
        WarpgrapherEndpoint::new(
            "RegisterUsers".to_string(),
            EndpointClass::Mutation,
            Some(WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Existing("UserCreateMutationInput".to_string()),
                true,
                true,
            )),
            WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Existing("User".to_string()),
                true,
                true,
            ),
        )
    }

    fn mock_endpoint_two() -> WarpgrapherEndpoint {
        // DisableUser(input: UserQueryInput): User
        WarpgrapherEndpoint::new(
            "DisableUser".to_string(),
            EndpointClass::Mutation,
            Some(WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Existing("UserQueryInput".to_string()),
                false,
                true,
            )),
            WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Existing("User".to_string()),
                false,
                true,
            ),
        )
    }

    fn mock_endpoint_three() -> WarpgrapherEndpoint {
        // ComputeBurndown(input: BurndownFilter): BurndownMetrics
        WarpgrapherEndpoint::new(
            "ComputeBurndown".to_string(),
            EndpointClass::Query,
            Some(WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Custom(WarpgrapherType::new(
                    "BurndownFilter".to_string(),
                    vec![WarpgrapherProp::new(
                        "ticket_types".to_string(),
                        "String".to_string(),
                        true,
                        false,
                        None,
                        None,
                    )],
                    vec![],
                    WarpgrapherEndpointsFilter::all(),
                )),
                false,
                false,
            )),
            WarpgrapherEndpointType::new(
                WarpgrapherTypeDef::Custom(WarpgrapherType::new(
                    "BurndownMetrics".to_string(),
                    vec![WarpgrapherProp::new(
                        "points".to_string(),
                        "Int".to_string(),
                        false,
                        false,
                        None,
                        None,
                    )],
                    vec![],
                    WarpgrapherEndpointsFilter::all(),
                )),
                false,
                true,
            ),
        )
    }

    fn mock_config() -> Config {
        Config::new(
            1,
            vec![
                mock_project_type(),
                mock_user_type(),
                mock_kanbanboard_type(),
                mock_scrumboard_type(),
                mock_feature_type(),
                mock_bug_type(),
                mock_commit_type(),
            ],
            vec![
                mock_endpoint_one(),
                mock_endpoint_two(),
                mock_endpoint_three(),
            ],
        )
    }

    /// Passes if a new Info struct is created
    #[test]
    fn info_new() {
        init();

        let i = Info::new("typename".to_string(), Arc::new(HashMap::new()));

        assert!(i.name == "typename");
    }

    /// Passes if a new NodeType is created
    #[test]
    fn node_type_new() {
        init();

        let nt = NodeType::new("typename".to_string(), TypeKind::Object, HashMap::new());

        assert!(nt.type_name == "typename");
        assert!(nt.type_kind == TypeKind::Object);
    }

    /// Passes if a new Property is created
    #[test]
    fn property_new() {
        init();

        let p = Property::new(
            "propname".to_string(),
            PropertyKind::Scalar,
            "String".to_string(),
            true,
            false,
            None,
            None,
            None,
        );

        assert!(p.name == "propname");
        assert!(p.kind == PropertyKind::Scalar);
        assert!(p.type_name == "String");
        assert!(p.required);
        assert!(!p.list);
        assert!(p.input.is_none());
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
        assert!(project_id.input == None);
        let project_name = project_node_object.props.get("name").unwrap();
        assert!(project_name.name == "name");
        assert!(project_name.kind == PropertyKind::Scalar);
        assert!(project_name.type_name == "String");
        assert!(project_name.required);
        assert!(!project_name.list);
        assert!(project_name.input == None);
        let project_tags = project_node_object.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.input == None);
        let project_public = project_node_object.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(project_public.required);
        assert!(!project_public.list);
        assert!(project_public.input == None);
        let project_owner = project_node_object.props.get("owner").unwrap();
        assert!(project_owner.name == "owner");
        assert!(match &project_owner.kind {
            PropertyKind::Rel(r) => r == "owner",
            _ => false,
        });
        assert!(project_owner.type_name == "ProjectOwnerRel");
        assert!(!project_owner.required);
        assert!(!project_owner.list);
        assert!(
            project_owner.input == Some((InputKind::Optional, "ProjectOwnerQueryInput".to_owned()))
        );
        let project_board = project_node_object.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(match &project_board.kind {
            PropertyKind::Rel(r) => r == "board",
            _ => false,
        });
        assert!(project_board.type_name == "ProjectBoardRel");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(
            project_board.input == Some((InputKind::Optional, "ProjectBoardQueryInput".to_owned()))
        );
        let project_commits = project_node_object.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(match &project_commits.kind {
            PropertyKind::Rel(r) => r == "commits",
            _ => false,
        });
        assert!(project_commits.type_name == "ProjectCommitsRel");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(
            project_commits.input
                == Some((InputKind::Optional, "ProjectCommitsQueryInput".to_owned()))
        );
        let project_issues = project_node_object.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(match &project_issues.kind {
            PropertyKind::Rel(r) => r == "issues",
            _ => false,
        });
        assert!(project_issues.type_name == "ProjectIssuesRel");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(
            project_issues.input
                == Some((InputKind::Optional, "ProjectIssuesQueryInput".to_owned()))
        );
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
        assert!(project_id.input == None);
        let project_name = project_query_input.props.get("name").unwrap();
        assert!(project_name.name == "name");
        assert!(project_name.kind == PropertyKind::Scalar);
        assert!(project_name.type_name == "String");
        assert!(!project_name.required);
        assert!(!project_name.list);
        assert!(project_name.input == None);
        let project_tags = project_query_input.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.input == None);
        let project_public = project_query_input.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(!project_public.required);
        assert!(!project_public.list);
        assert!(project_public.input == None);
        let project_owner = project_query_input.props.get("owner").unwrap();
        assert!(project_owner.name == "owner");
        assert!(project_owner.kind == PropertyKind::Input);
        assert!(project_owner.type_name == "ProjectOwnerQueryInput");
        assert!(!project_owner.required);
        assert!(!project_owner.list);
        assert!(project_owner.input == None);
        let project_board = project_query_input.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(project_board.kind == PropertyKind::Input);
        assert!(project_board.type_name == "ProjectBoardQueryInput");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(project_board.input == None);
        let project_commits = project_query_input.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(project_commits.kind == PropertyKind::Input);
        assert!(project_commits.type_name == "ProjectCommitsQueryInput");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(project_commits.input == None);
        let project_issues = project_query_input.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(project_issues.kind == PropertyKind::Input);
        assert!(project_issues.type_name == "ProjectIssuesQueryInput");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(project_issues.input == None);
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
        assert!(project_name.input == None);
        let project_tags = project_mutation_input.props.get("tags").unwrap();
        assert!(project_tags.name == "tags");
        assert!(project_tags.kind == PropertyKind::Scalar);
        assert!(project_tags.type_name == "String");
        assert!(!project_tags.required);
        assert!(project_tags.list);
        assert!(project_tags.input == None);
        let project_public = project_mutation_input.props.get("public").unwrap();
        assert!(project_public.name == "public");
        assert!(project_public.kind == PropertyKind::Scalar);
        assert!(project_public.type_name == "Boolean");
        assert!(!project_public.required);
        assert!(!project_public.list);
        assert!(project_public.input == None);
        let project_owner = project_mutation_input.props.get("owner").unwrap();
        assert!(project_owner.name == "owner");
        assert!(project_owner.kind == PropertyKind::Input);
        assert!(project_owner.type_name == "ProjectOwnerCreateMutationInput");
        assert!(!project_owner.required);
        assert!(!project_owner.list);
        assert!(project_owner.input == None);
        let project_board = project_mutation_input.props.get("board").unwrap();
        assert!(project_board.name == "board");
        assert!(project_board.kind == PropertyKind::Input);
        assert!(project_board.type_name == "ProjectBoardCreateMutationInput");
        assert!(!project_board.required);
        assert!(!project_board.list);
        assert!(project_board.input == None);
        let project_commits = project_mutation_input.props.get("commits").unwrap();
        assert!(project_commits.name == "commits");
        assert!(project_commits.kind == PropertyKind::Input);
        assert!(project_commits.type_name == "ProjectCommitsCreateMutationInput");
        assert!(!project_commits.required);
        assert!(project_commits.list);
        assert!(project_commits.input == None);
        let project_issues = project_mutation_input.props.get("issues").unwrap();
        assert!(project_issues.name == "issues");
        assert!(project_issues.kind == PropertyKind::Input);
        assert!(project_issues.type_name == "ProjectIssuesCreateMutationInput");
        assert!(!project_issues.required);
        assert!(project_issues.list);
        assert!(project_issues.input == None);
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
        assert!(name.input == None);
        let tags = project_update_mutation_input.props.get("tags").unwrap();
        assert!(tags.name == "tags");
        assert!(tags.kind == PropertyKind::Scalar);
        assert!(tags.type_name == "String");
        assert!(!tags.required);
        assert!(tags.list);
        assert!(tags.input == None);
        let public = project_update_mutation_input.props.get("public").unwrap();
        assert!(public.name == "public");
        assert!(public.kind == PropertyKind::Scalar);
        assert!(public.type_name == "Boolean");
        assert!(!public.required);
        assert!(!public.list);
        assert!(public.input == None);
        let owner = project_update_mutation_input.props.get("owner").unwrap();
        assert!(owner.name == "owner");
        assert!(owner.kind == PropertyKind::Input);
        assert!(owner.type_name == "ProjectOwnerChangeInput");
        assert!(!owner.required);
        assert!(!owner.list);
        assert!(owner.input == None);
        let board = project_update_mutation_input.props.get("board").unwrap();
        assert!(board.name == "board");
        assert!(board.kind == PropertyKind::Input);
        assert!(board.type_name == "ProjectBoardChangeInput");
        assert!(!board.required);
        assert!(!board.list);
        assert!(board.input == None);
        let commits = project_update_mutation_input.props.get("commits").unwrap();
        assert!(commits.name == "commits");
        assert!(commits.kind == PropertyKind::Input);
        assert!(commits.type_name == "ProjectCommitsChangeInput");
        assert!(!commits.required);
        assert!(commits.list);
        assert!(commits.input == None);
        let issues = project_update_mutation_input.props.get("issues").unwrap();
        assert!(issues.name == "issues");
        assert!(issues.kind == PropertyKind::Input);
        assert!(issues.type_name == "ProjectIssuesChangeInput");
        assert!(!issues.required);
        assert!(issues.list);
        assert!(issues.input == None);
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
                EXISTING: ProjectQueryInput
                NEW: ProjectCreateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_input = generate_node_input(&project_type);
        let project_match = project_input.props.get("EXISTING").unwrap();
        assert!(project_match.name == "EXISTING");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.input == None);
        let project_create = project_input.props.get("NEW").unwrap();
        assert!(project_create.name == "NEW");
        assert!(project_create.kind == PropertyKind::Input);
        assert!(project_create.type_name == "ProjectCreateMutationInput");
        assert!(!project_create.required);
        assert!(!project_create.list);
        assert!(project_create.input == None);
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
                match: ProjectQueryInput
                modify: ProjectUpdateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_update_input = generate_node_update_input(&project_type);
        let project_match = project_update_input.props.get("match").unwrap();
        assert!(project_match.name == "match");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.input == None);
        let project_update = project_update_input.props.get("modify").unwrap();
        assert!(project_update.name == "modify");
        assert!(project_update.kind == PropertyKind::Input);
        assert!(project_update.type_name == "ProjectUpdateMutationInput");
        assert!(!project_update.required);
        assert!(!project_update.list);
        assert!(project_update.input == None);
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
                match: ProjectQueryInput
                delete: ProjectDeleteMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_delete_input = generate_node_delete_input(&project_type);
        assert!(project_delete_input.type_name == "ProjectDeleteInput");
        assert!(project_delete_input.props.len() == 2);
        let project_match = project_delete_input.props.get("match").unwrap();
        assert!(project_match.name == "match");
        assert!(project_match.kind == PropertyKind::Input);
        assert!(project_match.type_name == "ProjectQueryInput");
        assert!(!project_match.required);
        assert!(!project_match.list);
        assert!(project_match.input == None);
        let project_delete = project_delete_input.props.get("delete").unwrap();
        assert!(project_delete.name == "delete");
        assert!(project_delete.kind == PropertyKind::Input);
        assert!(project_delete.type_name == "ProjectDeleteMutationInput");
        assert!(!project_delete.required);
        assert!(!project_delete.list);
        assert!(project_delete.input == None);
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
            force: Boolean
            owner: ProjectOwnerDeleteInput
            board: ProjectBoardDeleteInput
            commits: ProjectCommitsDeleteInput
            issues: ProjectIssuesDeleteInput
        }
        */
        let project_type = mock_project_type();
        let project_delete_mutation_input = generate_node_delete_mutation_input(&project_type);
        assert!(project_delete_mutation_input.type_name == "ProjectDeleteMutationInput");
        assert!(project_delete_mutation_input.props.len() == 5);
        let force = project_delete_mutation_input.props.get("force").unwrap();
        assert!(force.name == "force");
        assert!(force.kind == PropertyKind::Scalar);
        assert!(force.type_name == "Boolean");
        assert!(!force.required);
        assert!(!force.list);
        assert!(force.input == None);
        let owner = project_delete_mutation_input.props.get("owner").unwrap();
        assert!(owner.name == "owner");
        assert!(owner.kind == PropertyKind::Input);
        assert!(owner.type_name == "ProjectOwnerDeleteInput");
        assert!(!owner.required);
        assert!(!owner.list);
        assert!(owner.input == None);
        let board = project_delete_mutation_input.props.get("board").unwrap();
        assert!(board.name == "board");
        assert!(board.kind == PropertyKind::Input);
        assert!(board.type_name == "ProjectBoardDeleteInput");
        assert!(!board.required);
        assert!(!board.list);
        assert!(board.input == None);
        let commits = project_delete_mutation_input.props.get("commits").unwrap();
        assert!(commits.name == "commits");
        assert!(commits.kind == PropertyKind::Input);
        assert!(commits.type_name == "ProjectCommitsDeleteInput");
        assert!(!commits.required);
        assert!(commits.list);
        assert!(commits.input == None);
        let issues = project_delete_mutation_input.props.get("issues").unwrap();
        assert!(issues.name == "issues");
        assert!(issues.kind == PropertyKind::Input);
        assert!(issues.type_name == "ProjectIssuesDeleteInput");
        assert!(!issues.required);
        assert!(issues.list);
        assert!(issues.input == None);
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
        assert!(
            project_read_endpoint.input
                == Some((InputKind::Optional, "ProjectQueryInput".to_string()))
        );
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
        assert!(
            project_create_endpoint.input
                == Some((
                    InputKind::Required,
                    "ProjectCreateMutationInput".to_string()
                ))
        );
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
        assert!(
            project_update_endpoint.input
                == Some((InputKind::Required, "ProjectUpdateInput".to_string()))
        );
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
            PropertyKind::NodeDeleteMutation(s) => s == "Project",
            _ => false,
        });
        assert!(project_delete_endpoint.type_name == "Int");
        assert!(!project_delete_endpoint.required);
        assert!(!project_delete_endpoint.list);
        assert!(
            project_delete_endpoint.input
                == Some((InputKind::Required, "ProjectDeleteInput".to_string()))
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_object_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_object = generate_rel_object(&project_type, &project_owner_rel);
        let project_owner_id = project_owner_object.props.get("id").unwrap();
        assert!(project_owner_id.name == "id");
        assert!(project_owner_id.kind == PropertyKind::Scalar);
        assert!(project_owner_id.type_name == "ID");
        assert!(project_owner_id.required);
        assert!(!project_owner_id.list);
        assert!(project_owner_id.input == None);
        let project_owner_props = project_owner_object.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Object);
        assert!(project_owner_props.type_name == "ProjectOwnerProps");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.input == None);
        let project_owner_dst = project_owner_object.props.get("dst").unwrap();
        assert!(project_owner_dst.name == "dst");
        assert!(project_owner_dst.kind == PropertyKind::Union);
        assert!(project_owner_dst.type_name == "ProjectOwnerNodesUnion");
        assert!(project_owner_dst.required);
        assert!(!project_owner_dst.list);
        assert!(project_owner_dst.input == None);
        let project_owner_src = project_owner_object.props.get("src").unwrap();
        assert!(project_owner_src.name == "src");
        assert!(project_owner_src.kind == PropertyKind::Object);
        assert!(project_owner_src.type_name == "Project");
        assert!(project_owner_src.required);
        assert!(!project_owner_src.list);
        assert!(project_owner_src.input == None);
        /*
            type ProjectBoardRel {
                id: ID!
                props: ProjectBoardProps
                dst: ProjectBoardNodesUnion!
                src: Project!
            }
        */
        let project_type = mock_project_type();
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        let project_board_object = generate_rel_object(&project_type, &project_board_rel);
        let project_board_id = project_board_object.props.get("id").unwrap();
        assert!(project_board_id.name == "id");
        assert!(project_board_id.kind == PropertyKind::Scalar);
        assert!(project_board_id.type_name == "ID");
        assert!(project_board_id.required);
        assert!(!project_board_id.list);
        assert!(project_board_id.input == None);
        let project_board_props = project_board_object.props.get("props");
        assert!(project_board_props.is_none());
        let project_board_dst = project_board_object.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Union);
        assert!(project_board_dst.type_name == "ProjectBoardNodesUnion");
        assert!(project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.input == None);
        let project_board_src = project_board_object.props.get("src").unwrap();
        assert!(project_board_src.name == "src");
        assert!(project_board_src.kind == PropertyKind::Object);
        assert!(project_board_src.type_name == "Project");
        assert!(project_board_src.required);
        assert!(!project_board_src.list);
        assert!(project_board_src.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_props_object_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_props_object =
            generate_rel_props_object(&project_type, &project_owner_rel);
        assert!(project_owner_props_object.props.len() == 1);
        let project_owner_props_name = project_owner_props_object.props.get("since").unwrap();
        assert!(project_owner_props_name.name == "since");
        assert!(project_owner_props_name.kind == PropertyKind::Scalar);
        assert!(project_owner_props_name.type_name == "String");
        assert!(!project_owner_props_name.required);
        assert!(!project_owner_props_name.list);
        assert!(project_owner_props_name.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_nodes_union_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_query_input = generate_rel_query_input(&project_type, &project_owner_rel);
        // id
        let project_owner_id = project_owner_query_input.props.get("id").unwrap();
        assert!(project_owner_id.name == "id");
        assert!(project_owner_id.kind == PropertyKind::Scalar);
        assert!(project_owner_id.type_name == "ID");
        assert!(!project_owner_id.required);
        assert!(!project_owner_id.list);
        assert!(project_owner_id.input == None);
        // props
        let project_owner_props = project_owner_query_input.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerPropsInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.input == None);
        // src
        let project_owner_props = project_owner_query_input.props.get("src").unwrap();
        assert!(project_owner_props.name == "src");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerSrcQueryInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.input == None);
        // dst
        let project_owner_props = project_owner_query_input.props.get("dst").unwrap();
        assert!(project_owner_props.name == "dst");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerDstQueryInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.input == None);
        /*
            input ProjectBoardQueryInput {
                id: ID
                props: ProjectBoardPropsInput
                src: ProjectBoardSrcQueryInput
                dst: ProjectBoardDstQueryInput
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        let project_board_query_input = generate_rel_query_input(&project_type, &project_board_rel);
        // id
        let project_board_id = project_board_query_input.props.get("id").unwrap();
        assert!(project_board_id.name == "id");
        assert!(project_board_id.kind == PropertyKind::Scalar);
        assert!(project_board_id.type_name == "ID");
        assert!(!project_board_id.required);
        assert!(!project_board_id.list);
        assert!(project_board_id.input == None);
        // props
        assert!(project_board_query_input.props.get("props").is_none());
        // src
        let project_board_src = project_board_query_input.props.get("src").unwrap();
        assert!(project_board_src.name == "src");
        assert!(project_board_src.kind == PropertyKind::Input);
        assert!(project_board_src.type_name == "ProjectBoardSrcQueryInput");
        assert!(!project_board_src.required);
        assert!(!project_board_src.list);
        assert!(project_board_src.input == None);
        // dst
        let project_board_dst = project_board_query_input.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Input);
        assert!(project_board_dst.type_name == "ProjectBoardDstQueryInput");
        assert!(!project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_mutation_input =
            generate_rel_create_mutation_input(&project_type, &project_owner_rel);
        assert!(project_owner_mutation_input.type_name == "ProjectOwnerCreateMutationInput");
        // props
        let project_owner_props = project_owner_mutation_input.props.get("props").unwrap();
        assert!(project_owner_props.name == "props");
        assert!(project_owner_props.kind == PropertyKind::Input);
        assert!(project_owner_props.type_name == "ProjectOwnerPropsInput");
        assert!(!project_owner_props.required);
        assert!(!project_owner_props.list);
        assert!(project_owner_props.input == None);
        // dst
        let project_owner_dst = project_owner_mutation_input.props.get("dst").unwrap();
        assert!(project_owner_dst.name == "dst");
        assert!(project_owner_dst.kind == PropertyKind::Input);
        assert!(project_owner_dst.type_name == "ProjectOwnerNodesMutationInputUnion");
        assert!(project_owner_dst.required);
        assert!(!project_owner_dst.list);
        assert!(project_owner_dst.input == None);
        /*
            input ProjectBoardCreateMutationInput {
                dst: ProjectBoardNodesMutationInputUnion
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        let project_board_mutation_input =
            generate_rel_create_mutation_input(&project_type, &project_board_rel);
        assert!(project_board_mutation_input.type_name == "ProjectBoardCreateMutationInput");
        // props
        let project_board_props = project_board_mutation_input.props.get("props");
        assert!(project_board_props.is_none());
        // dst
        let project_board_dst = project_board_mutation_input.props.get("dst").unwrap();
        assert!(project_board_dst.name == "dst");
        assert!(project_board_dst.kind == PropertyKind::Input);
        assert!(project_board_dst.type_name == "ProjectBoardNodesMutationInputUnion");
        assert!(project_board_dst.required);
        assert!(!project_board_dst.list);
        assert!(project_board_dst.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_change_input_name() {
        let project_type = mock_project_type();
        let project_issues_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "issues")
            .unwrap();
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
                ADD: ProjectIssuesCreateMutationInput
                UPDATE: ProjectIssuesUpdateInput
                DELETE: ProjectIssuesDeleteInput
            }
        */
        let project_type = mock_project_type();
        let project_issues_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "issues")
            .unwrap();
        let project_issues_change_input =
            generate_rel_change_input(&project_type, &project_issues_rel);
        assert!(project_issues_change_input.type_name == "ProjectIssuesChangeInput");
        // ADD
        let project_issues_add = project_issues_change_input.props.get("ADD").unwrap();
        assert!(project_issues_add.name == "ADD");
        assert!(project_issues_add.kind == PropertyKind::Input);
        assert!(project_issues_add.type_name == "ProjectIssuesCreateMutationInput");
        assert!(!project_issues_add.required);
        assert!(!project_issues_add.list);
        assert!(project_issues_add.input == None);
        // UPDATE
        let project_issues_update = project_issues_change_input.props.get("UPDATE").unwrap();
        assert!(project_issues_update.name == "UPDATE");
        assert!(project_issues_update.kind == PropertyKind::Input);
        assert!(project_issues_update.type_name == "ProjectIssuesUpdateInput");
        assert!(!project_issues_update.required);
        assert!(!project_issues_update.list);
        assert!(project_issues_update.input == None);
        // DELETE
        let project_issues_delete = project_issues_change_input.props.get("DELETE").unwrap();
        assert!(project_issues_delete.name == "DELETE");
        assert!(project_issues_delete.kind == PropertyKind::Input);
        assert!(project_issues_delete.type_name == "ProjectIssuesDeleteInput");
        assert!(!project_issues_delete.required);
        assert!(!project_issues_delete.list);
        assert!(project_issues_delete.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_update_mutation_input =
            generate_rel_update_mutation_input(&project_type, &project_owner_rel);
        assert!(project_owner_update_mutation_input.type_name == "ProjectOwnerUpdateMutationInput");
        // props
        let props = project_owner_update_mutation_input
            .props
            .get("props")
            .unwrap();
        assert!(props.name == "props");
        assert!(props.kind == PropertyKind::Input);
        assert!(props.type_name == "ProjectOwnerPropsInput");
        assert!(!props.required);
        assert!(!props.list);
        assert!(props.input == None);
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
        assert!(src.input == None);
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
        assert!(dst.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_src_update_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(project.input == None);
        /*
            input ProjectIssuesSrcUpdateMutationInput {
                Project: ProjectUpdateMutationInput
            }
        */
        let project_issues_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "issues")
            .unwrap();
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
        assert!(project2.input == None);
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(user.name == "User");
        assert!(user.kind == PropertyKind::Input);
        assert!(user.type_name == "UserUpdateMutationInput");
        assert!(!user.required);
        assert!(!user.list);
        assert!(user.input == None);
        /*
            input ProjectIssuesDstUpdateMutationInput {
                Bug: BugUpdateMutationInput
                Feature: FeatureUpdateMutationInput
            }
        */
        let project_issues_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "issues")
            .unwrap();
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
        assert!(bug.input == None);
        let feature = project_issues_dst_update_mutation_input
            .props
            .get("Feature")
            .unwrap();
        assert!(feature.name == "Feature");
        assert!(feature.kind == PropertyKind::Input);
        assert!(feature.type_name == "FeatureUpdateMutationInput");
        assert!(!feature.required);
        assert!(!feature.list);
        assert!(feature.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_props_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(project_owner_since.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_src_query_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_src_query_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerSrcQueryInput"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        assert!(
            fmt_rel_src_query_input_name(&project_type, &project_board_rel)
                == "ProjectBoardSrcQueryInput"
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_dst_query_input_name_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_dst_query_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDstQueryInput"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(!user_input.required);
        assert!(!user_input.list);
        assert!(user_input.input == None);
        /*
            input ProjectBoardNodesQueryInputUnion {
                KanbanBoard: KanbanBoardQueryInput
                ScrumBoard: ScrumBoardQueryInput
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        assert!(!kanbanboard_input.required);
        assert!(!kanbanboard_input.list);
        assert!(kanbanboard_input.input == None);
        let scrumboard_input = project_board_nodes_query_input_union
            .props
            .get("ScrumBoard")
            .unwrap();
        assert!(scrumboard_input.name == "ScrumBoard");
        assert!(scrumboard_input.kind == PropertyKind::Input);
        assert!(scrumboard_input.type_name == "ScrumBoardQueryInput");
        assert!(!scrumboard_input.required);
        assert!(!scrumboard_input.list);
        assert!(scrumboard_input.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_nodes_mutation_input_union_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_nodes_mutation_input_union_name(&project_type, &project_owner_rel)
                == "ProjectOwnerNodesMutationInputUnion"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(!user_input.required);
        assert!(!user_input.list);
        assert!(user_input.input == None);
        /*
            input ProjectBoardNodesQueryInputUnion {
                KanbanBoard: KanbanBoardInput
                ScrumBoard: ScrumBoardInput
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        assert!(!kanbanboard_input.required);
        assert!(!kanbanboard_input.list);
        assert!(kanbanboard_input.input == None);
        let scrumboard_input = project_board_nodes_mutation_input_union
            .props
            .get("ScrumBoard")
            .unwrap();
        assert!(scrumboard_input.name == "ScrumBoard");
        assert!(scrumboard_input.kind == PropertyKind::Input);
        assert!(scrumboard_input.type_name == "ScrumBoardInput");
        assert!(!scrumboard_input.required);
        assert!(!scrumboard_input.list);
        assert!(scrumboard_input.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_create_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerCreateInput"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
                match: ProjectQueryInput
                create: ProjectOwnerCreateMutationInput
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_create_input =
            generate_rel_create_input(&project_type, &project_owner_rel);
        assert!(project_owner_create_input.type_name == "ProjectOwnerCreateInput");
        assert!(project_owner_create_input.type_kind == TypeKind::Input);
        assert!(project_owner_create_input.props.len() == 2);
        let project_owner_match = project_owner_create_input.props.get("match").unwrap();
        assert!(project_owner_match.name == "match");
        assert!(project_owner_match.kind == PropertyKind::Input);
        assert!(project_owner_match.type_name == "ProjectQueryInput");
        assert!(!project_owner_match.required);
        assert!(!project_owner_match.list);
        assert!(project_owner_match.input == None);
        let project_owner_create = project_owner_create_input.props.get("create").unwrap();
        assert!(project_owner_create.name == "create");
        assert!(project_owner_create.kind == PropertyKind::Input);
        assert!(project_owner_create.type_name == "ProjectOwnerCreateMutationInput");
        assert!(!project_owner_create.required);
        assert!(!project_owner_create.list);
        assert!(project_owner_create.input == None);
        /*
            input ProjectBoardCreateInput {
                match: ProjectQueryInput
                create: ProjectBoardCreateMutationInput
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        let project_board_create_input =
            generate_rel_create_input(&project_type, &project_board_rel);
        assert!(project_board_create_input.type_name == "ProjectBoardCreateInput");
        assert!(project_board_create_input.type_kind == TypeKind::Input);
        assert!(project_board_create_input.props.len() == 2);
        let project_board_match = project_board_create_input.props.get("match").unwrap();
        assert!(project_board_match.name == "match");
        assert!(project_board_match.kind == PropertyKind::Input);
        assert!(project_board_match.type_name == "ProjectQueryInput");
        assert!(!project_board_match.required);
        assert!(!project_board_match.list);
        assert!(project_board_match.input == None);
        let project_board_create = project_board_create_input.props.get("create").unwrap();
        assert!(project_board_create.name == "create");
        assert!(project_board_create.kind == PropertyKind::Input);
        assert!(project_board_create.type_name == "ProjectBoardCreateMutationInput");
        assert!(!project_board_create.required);
        assert!(!project_board_create.list);
        assert!(project_board_create.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_update_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerUpdateInput"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
                match: ProjectOwnerQueryInput
                update: ProjectOwnerUpdateMutationInput!
            }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_update_input =
            generate_rel_update_input(&project_type, &project_owner_rel);
        assert!(project_owner_update_input.type_name == "ProjectOwnerUpdateInput");
        assert!(project_owner_update_input.type_kind == TypeKind::Input);
        assert!(project_owner_update_input.props.len() == 2);
        let project_owner_match = project_owner_update_input.props.get("match").unwrap();
        assert!(project_owner_match.name == "match");
        assert!(project_owner_match.kind == PropertyKind::Input);
        assert!(project_owner_match.type_name == "ProjectOwnerQueryInput");
        assert!(!project_owner_match.required);
        assert!(!project_owner_match.list);
        assert!(project_owner_match.input == None);
        let project_owner_update = project_owner_update_input.props.get("update").unwrap();
        assert!(project_owner_update.name == "update");
        assert!(project_owner_update.kind == PropertyKind::Input);
        assert!(project_owner_update.type_name == "ProjectOwnerUpdateMutationInput");
        assert!(project_owner_update.required);
        assert!(!project_owner_update.list);
        assert!(project_owner_update.input == None);
        /*
            input ProjectBoardUpdateInput {
                match: ProjectBoardQueryInput
                update: ProjectBoardUpdateMutationInput!
            }
        */
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        let project_board_update_input =
            generate_rel_update_input(&project_type, &project_board_rel);
        assert!(project_board_update_input.type_name == "ProjectBoardUpdateInput");
        assert!(project_board_update_input.type_kind == TypeKind::Input);
        assert!(project_board_update_input.props.len() == 2);
        let project_board_match = project_board_update_input.props.get("match").unwrap();
        assert!(project_board_match.name == "match");
        assert!(project_board_match.kind == PropertyKind::Input);
        assert!(project_board_match.type_name == "ProjectBoardQueryInput");
        assert!(!project_board_match.required);
        assert!(!project_board_match.list);
        assert!(project_board_match.input == None);
        let project_board_update = project_board_update_input.props.get("update").unwrap();
        assert!(project_board_update.name == "update");
        assert!(project_board_update.kind == PropertyKind::Input);
        assert!(project_board_update.type_name == "ProjectBoardUpdateMutationInput");
        assert!(project_board_update.required);
        assert!(!project_board_update.list);
        assert!(project_board_update.input == None);
    }

    #[test]
    fn test_fmt_rel_delete_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_delete_input_name(&project_type, &project_owner_rel)
                == "ProjectOwnerDeleteInput"
        );
    }

    #[test]
    fn test_generate_rel_delete_input() {
        /*
        input ProjectOwnerDeleteInput {
            match: ProjectOwnerQueryInput
            src: ProjectOwnerSrcMutationInput
            dst: ProjectOwnerDstDeleteMutationInput
        }
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_delete_input =
            generate_rel_delete_input(&project_type, &project_owner_rel);
        assert!(project_owner_delete_input.type_name == "ProjectOwnerDeleteInput");
        let pmatch = project_owner_delete_input.props.get("match").unwrap();
        assert!(pmatch.name == "match");
        assert!(pmatch.kind == PropertyKind::Input);
        assert!(pmatch.type_name == "ProjectOwnerQueryInput");
        assert!(!pmatch.required);
        assert!(!pmatch.list);
        assert!(pmatch.input == None);
        let src = project_owner_delete_input.props.get("src").unwrap();
        assert!(src.name == "src");
        assert!(src.kind == PropertyKind::Input);
        assert!(src.type_name == "ProjectOwnerSrcDeleteMutationInput");
        assert!(!src.required);
        assert!(!src.list);
        assert!(src.input == None);
        let dst = project_owner_delete_input.props.get("dst").unwrap();
        assert!(dst.name == "dst");
        assert!(dst.kind == PropertyKind::Input);
        assert!(dst.type_name == "ProjectOwnerDstDeleteMutationInput");
        assert!(!dst.required);
        assert!(!dst.list);
        assert!(dst.input == None);
    }

    #[test]
    fn test_fmt_rel_src_delete_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(project.input == None);
    }

    #[test]
    fn test_fmt_rel_dst_delete_mutation_input_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
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
        assert!(user.name == "User");
        assert!(user.kind == PropertyKind::Input);
        assert!(user.type_name == "UserDeleteMutationInput");
        assert!(!user.required);
        assert!(!user.list);
        assert!(user.input == None);

        /*
        input ProjectIssuesDstDeleteMutationInput {
            Bug: BugDeleteMutationInput
            Feature: FeatureDeleteMutationInput
        }
        */
        let project_issues_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "issues")
            .unwrap();
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
        assert!(bug.input == None);
        let feature = project_issues_dst_delete_mutation_input
            .props
            .get("Feature")
            .unwrap();
        assert!(feature.name == "Feature");
        assert!(feature.kind == PropertyKind::Input);
        assert!(feature.type_name == "FeatureDeleteMutationInput");
        assert!(!feature.required);
        assert!(!feature.list);
        assert!(feature.input == None);
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_read_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(fmt_rel_read_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwner");
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
        assert!(fmt_rel_read_endpoint_name(&project_type, &project_board_rel) == "ProjectBoard");
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_rel_read_endpoint() {
        /*
            ProjectOwner(input: ProjectOwnerQueryInput): [ProjectOwnerRel]
        */
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_read_endpoint =
            generate_rel_read_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_read_endpoint.name == "ProjectOwner");
        assert!(match &project_owner_read_endpoint.kind {
            PropertyKind::Rel(r) => r == "owner",
            _ => false,
        });
        assert!(project_owner_read_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_read_endpoint.required);
        assert!(project_owner_read_endpoint.list);
        assert!(
            project_owner_read_endpoint.input
                == Some((InputKind::Optional, "ProjectOwnerQueryInput".to_string()))
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_create_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_create_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerCreate"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_create_endpoint =
            generate_rel_create_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_create_endpoint.name == "ProjectOwnerCreate");
        assert!(match &project_owner_create_endpoint.kind {
            PropertyKind::RelCreateMutation(n, r) => n == "Project" && r == "owner",
            _ => false,
        });
        assert!(project_owner_create_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_create_endpoint.required);
        assert!(!project_owner_create_endpoint.list);
        assert!(
            project_owner_create_endpoint.input
                == Some((InputKind::Required, "ProjectOwnerCreateInput".to_string()))
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_update_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_update_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerUpdate"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_update_endpoint =
            generate_rel_update_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_update_endpoint.name == "ProjectOwnerUpdate");
        assert!(match &project_owner_update_endpoint.kind {
            PropertyKind::RelUpdateMutation(n, r) => n == "Project" && r == "owner",
            _ => false,
        });
        assert!(project_owner_update_endpoint.type_name == "ProjectOwnerRel");
        assert!(!project_owner_update_endpoint.required);
        assert!(project_owner_update_endpoint.list);
        assert!(
            project_owner_update_endpoint.input
                == Some((InputKind::Required, "ProjectOwnerUpdateInput".to_string()))
        );
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_fmt_rel_delete_endpoint_name() {
        let project_type = mock_project_type();
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        assert!(
            fmt_rel_delete_endpoint_name(&project_type, &project_owner_rel) == "ProjectOwnerDelete"
        );
        let project_board_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "board")
            .unwrap();
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
        let project_owner_rel = project_type
            .rels
            .iter()
            .find(|&r| r.name == "owner")
            .unwrap();
        let project_owner_delete_endpoint =
            generate_rel_delete_endpoint(&project_type, &project_owner_rel);
        assert!(project_owner_delete_endpoint.name == "ProjectOwnerDelete");
        assert!(match &project_owner_delete_endpoint.kind {
            PropertyKind::RelDeleteMutation(n, r) => n == "Project" && r == "owner",
            _ => false,
        });
        assert!(project_owner_delete_endpoint.type_name == "Int");
        assert!(!project_owner_delete_endpoint.required);
        assert!(!project_owner_delete_endpoint.list);
        assert!(
            project_owner_delete_endpoint.input
                == Some((InputKind::Required, "ProjectOwnerDeleteInput".to_string()))
        );
    }

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
        assert!(
            e1_object.input == Some((InputKind::Required, "UserCreateMutationInput".to_string()))
        );
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
        assert!(e2_object.input == Some((InputKind::Required, "UserQueryInput".to_string())));
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
        assert!(e3_object.input == Some((InputKind::Optional, "BurndownFilter".to_string())));
    }

    /// Passes if the right schema elements are generated
    #[test]
    fn test_generate_schema() {
        init();
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
        let config = Config::new(
            1,
            vec![WarpgrapherType::new(
                "User".to_string(),
                vec![WarpgrapherProp::new(
                    "name".to_string(),
                    "String".to_string(),
                    true,
                    false,
                    None,
                    None,
                )],
                vec![],
                WarpgrapherEndpointsFilter::new(false, true, false, false),
            )],
            Vec::new(),
        );
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
        init();
        let config = mock_config();
        let root_node = create_root_node::<(), ()>(&config);
        assert!(root_node.is_ok());
    }

    /// Passes if a broken reference creates an error
    #[test]
    fn type_lookup_error() {
        init();
        let config = Config::new(1, vec![mock_project_type()], vec![]);
        let root_node = create_root_node::<(), ()>(&config);
        assert!(root_node.is_err());
    }
}
