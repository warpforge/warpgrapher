mod setup;

use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename 
            name
            owner{
                __typename 
                since
                dst{
                    ...on User{
                        __typename 
                        name
                    }
                }
            }",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": { "NEW": { "name": "User Zero" } }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert!(p0.get("__typename").unwrap() == "Project");
    assert!(p0.get("name").unwrap() == "Project Zero");

    let owner0 = p0.get("owner").unwrap();
    assert!(owner0.is_object());
    assert!(owner0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner0.get("since").unwrap() == "yesterday");

    let dst0 = owner0.get("dst").unwrap();
    assert!(dst0.is_object());
    assert!(dst0.get("__typename").unwrap() == "User");
    assert!(dst0.get("name").unwrap() == "User Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename 
            name 
            owner { 
                __typename 
                since 
                dst { 
                    ...on User { 
                        __typename 
                        name 
                    } 
                } 
            }",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("owner").unwrap().is_object());
    let owner = project.get("owner").unwrap();

    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_node_with_rel_to_existing<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let p0 = client
        .create_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }
            ",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    assert!(p0.get("__typename").unwrap() == "Project");
    assert!(p0.get("name").unwrap() == "Project Zero");

    let o0 = p0.get("owner").unwrap();
    assert!(o0.is_object());
    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename 
            name 
            owner { 
                __typename 
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("__typename").unwrap() == "Project");
    assert!(project.get("name").unwrap() == "Project Zero");

    let owner = project.get("owner").unwrap();
    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_multiple_snst_node_with_rel<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _u1 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            &json!({
                "name": "User One"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }
            ",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }
            ",
            &json!({
                "name": "Project One",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User One"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            owner { 
                __typename 
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename 
            owner { 
                __typename 
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project One"}
            })),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User One");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "owner": {
                    "since": {"EQ": "yesterday"}
                }
            })),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("__typename").unwrap() == "Project");
    assert!(project.get("name").unwrap() == "Project Zero");

    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "owner": {
                    "dst": {
                        "User": {
                            "name": {"EQ": "User Zero"}
                        }
                    }
                }
            })),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("__typename").unwrap() == "Project");
    assert!(project.get("name").unwrap() == "Project Zero");

    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_new_rel<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .update_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            &json!({
                "owner": {
                    "ADD": {
                        "since": "today",
                        "dst": {
                            "User": {
                                "NEW": {
                                    "name": "User One"
                                }
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    // Cannot add a rel to a single-node rel slot that's already filled.
    assert_eq!(
        projects.as_array().unwrap()[0]
            .get("owner")
            .unwrap()
            .clone(),
        serde_json::value::Value::Null
    );

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_existing_rel<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _u1 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User One"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            &json!({
                "owner": {
                    "ADD": {
                        "since": "today",
                        "dst": {
                            "User": {
                                "EXISTING": {
                                    "name": {"EQ": "User One"}
                                }
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        pu.as_array().unwrap()[0].get("owner").unwrap().clone(),
        serde_json::value::Value::Null
    );

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .update_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            &json!({
                "owner": {
                    "DELETE": {
                        "MATCH": {
                            "dst": {
                                "User": {
                                    "name": {"EQ": "User Zero"}
                                }
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("__typename").unwrap() == "Project");
    assert!(project.get("name").unwrap() == "Project Zero");

    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .update_node(
            "Project",
            "__typename 
            name
            owner { 
                __typename 
                since
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": {"EQ": "Project Zero"}
            })),
            &json!({
                "owner": {
                    "DELETE": {
                        "MATCH": {
                            "since": {"EQ": "yesterday"}
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("__typename").unwrap() == "Project");
    assert!(project.get("name").unwrap() == "Project Zero");

    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "Project",
            Some(&json!({
                "owner": {
                    "dst": {
                        "User": {
                            "name": {"EQ": "User Zero"}
                        }
                    }
                }
            })),
            Some(&json!({
                "owner": {
                    "MATCH": {}
                }
            })),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                since
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "Project",
            Some(&json!({
                "owner": {
                    "since": {"EQ": "yesterday"}
                }
            })),
            Some(&json!({
                "owner": {
                    "MATCH": {}
                }
            })),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                since
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn detach_snst_rel_by_dst_delete<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            &json!({
                "name": "User Zero"
            }),
            None,
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": {"EQ": "User Zero"}
                            }
                        }
                    }
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "User",
            Some(&json!({
                "name": {"EQ": "User Zero"}
            })),
            Some(&json!({})),
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename
            name
            owner {
                __typename
                since
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("name").unwrap() == "Project Zero");
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some(&json!({
                "name": {"EQ": "User Zero"}
                }
            )),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    assert_eq!(users_a.len(), 0);
}
