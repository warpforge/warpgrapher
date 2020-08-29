mod setup;

use serde_json::json;
#[cfg(feature = "cosmos")]
use setup::cosmos_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(any(feature = "cosmos", feature = "neo4j"))]
use setup::{clear_db, init};
use setup::{AppGlobalCtx, AppRequestCtx};
use warpgrapher::client::Client;

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snst_new_node_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_new_node(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snst_new_node_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_new_node(client).await;
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_node(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename 
            name
            owner{
                __typename 
                props{
                    __typename 
                    since
                } 
                dst{
                    ...on User{
                        __typename 
                        name
                    }
                }
            }",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": { "since": "yesterday" },
                    "dst": {
                        "User": { "NEW": { "name": "User Zero" } }
                    }
                }
            }),
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert!(p0.get("__typename").unwrap() == "Project");
    assert!(p0.get("name").unwrap() == "Project Zero");

    let owner0 = p0.get("owner").unwrap();
    assert!(owner0.is_object());
    assert!(owner0.get("__typename").unwrap() == "ProjectOwnerRel");

    let props0 = owner0.get("props").unwrap();
    assert!(props0.is_object());
    assert!(props0.get("__typename").unwrap() == "ProjectOwnerProps");
    assert!(props0.get("since").unwrap() == "yesterday");

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
                props { 
                    __typename 
                    since 
                } 
                dst { 
                    ...on User { 
                        __typename 
                        name 
                    } 
                } 
            }",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    assert!(project.get("owner").unwrap().is_object());
    let owner = project.get("owner").unwrap();

    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_node_with_rel_to_existing_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_node_with_rel_to_existing(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_node_with_rel_to_existing_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_node_with_rel_to_existing(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_node_with_rel_to_existing(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
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
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_multiple_snst_node_with_rel_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_multiple_snst_node_with_rel(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_multiple_snst_node_with_rel_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_multiple_snst_node_with_rel(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_multiple_snst_node_with_rel(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _u1 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            Some("1234"),
            &json!({
                "name": "User One"
            }),
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
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            &json!({
                "name": "Project One",
                "owner": {
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User One"
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
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
            Some("1234"),
            Some(&json!({
                "name": "Project One"
            })),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snst_node_by_rel_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_node_by_rel_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snst_node_by_rel_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_node_by_rel_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_rel_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "owner": {
                    "props": {
                        "since": "yesterday"
                    }
                }
            })),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snst_node_by_dst_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_node_by_dst_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snst_node_by_dst_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_node_by_dst_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_dst_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "owner": {
                    "dst": {
                        "User": {
                            "name": "User Zero"
                        }
                    }
                }
            })),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snst_node_with_new_rel_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_node_with_new_rel(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snst_node_with_new_rel_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_node_with_new_rel(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_new_rel(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
            &json!({
                "owner": {
                    "ADD": {
                        "props": {
                            "since": "today"
                        },
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snst_node_with_existing_rel_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_node_with_existing_rel(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snst_node_with_existing_rel_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_node_with_existing_rel(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_existing_rel(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _u1 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User One"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
            &json!({
                "owner": {
                    "ADD": {
                        "props": {
                            "since": "today"
                        },
                        "dst": {
                            "User": {
                                "EXISTING": {
                                    "name": "User One"
                                }
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_rel_by_dst_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_dst_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_rel_by_dst_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_dst_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_dst_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
            &json!({
                "owner": {
                    "DELETE": {
                        "match": {
                            "dst": {
                                "User": {
                                    "name": "User Zero"
                                }
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_rel_by_rel_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_rel_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_rel_by_rel_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_rel_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
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
                props {
                    since
                }
                dst { 
                    ... on User {
                        __typename 
                        name
                    }
                } 
            }",
            Some("1234"),
            Some(&json!({
                "name": "Project Zero"
            })),
            &json!({
                "owner": {
                    "DELETE": {
                        "match": {
                            "props": {
                                "since": "yesterday"
                            }
                        }
                    }
                }
            }),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_node_by_dst_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_node_by_dst_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_node_by_dst_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_node_by_dst_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_dst_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({
                "owner": {
                    "dst": {
                        "User": {
                            "name": "User Zero"
                        }
                    }
                }
            })),
            Some(&json!({
                "owner": {
                    "match": {}
                }
            })),
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
                props {
                    since
                }
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            Some("1234"),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_node_by_rel_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_node_by_rel_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_node_by_rel_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_node_by_rel_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_rel_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({
                "owner": {
                    "props": {
                        "since": "yesterday"
                    }
                }
            })),
            Some(&json!({
                "owner": {
                    "match": {}
                }
            })),
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
                props {
                    since
                }
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            Some("1234"),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    let user = &users_a[0];

    assert!(user.get("name").unwrap() == "User Zero");
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn detach_snst_rel_by_dst_delete_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    detach_snst_rel_by_dst_delete(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn detach_snst_rel_by_dst_delete_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    detach_snst_rel_by_dst_delete(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn detach_snst_rel_by_dst_delete(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234"),
            &json!({
                "name": "User Zero"
            }),
        )
        .await
        .unwrap();

    let _p0 = client
        .create_node(
            "Project",
            "__typename",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {
                        "since": "yesterday"
                    },
                    "dst": {
                        "User": {
                            "EXISTING": {
                                "name": "User Zero"
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let _projects = client
        .delete_node(
            "User",
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
            })),
            Some(&json!({})),
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
                props {
                    since
                }
                dst {
                    ...on User {
                        __typename
                        name
                    }
                }
            }",
            Some("1234"),
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
            Some("1234"),
            Some(&json!({
                "name": "User Zero"
                }
            )),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    assert_eq!(users_a.len(), 0);
}
