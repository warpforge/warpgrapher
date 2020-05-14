mod setup;

use serde_json::json;
#[cfg(feature = "graphson2")]
use setup::server::test_server_graphson2;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use setup::{clear_db, init};
#[cfg(feature = "graphson2")]
use setup::graphson2_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
use warpgrapher::client::Client;

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snst_new_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_snst_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn create_snst_new_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    create_snst_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_node(mut client: Client) {
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_node_with_rel_to_existing(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn create_node_with_rel_to_existing_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    create_node_with_rel_to_existing(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_node_with_rel_to_existing(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_multiple_snst_node_with_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn read_multiple_snst_node_with_rel_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    read_multiple_snst_node_with_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_multiple_snst_node_with_rel(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename
            name
            ",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_snst_node_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn read_snst_node_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    read_snst_node_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_rel_props(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_snst_node_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn read_snst_node_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    read_snst_node_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_node_by_dst_props(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_snst_node_with_new_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_snst_node_with_new_rel_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_snst_node_with_new_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_new_rel(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    assert_eq!(projects, serde_json::value::Value::Null);

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_snst_node_with_existing_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_snst_node_with_existing_rel_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_snst_node_with_existing_rel(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_node_with_existing_rel(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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

    assert_eq!(pu, serde_json::value::Value::Null);

    let users = client
        .read_node(
            "User",
            "__typename 
            name
            ",
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_snst_rel_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_snst_rel_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_snst_rel_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_dst_props(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_snst_rel_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_snst_rel_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_snst_rel_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_props(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_snst_node_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_snst_node_by_dst_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_snst_node_by_dst_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_dst_prop(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_snst_node_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_snst_node_by_rel_prop_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_snst_node_by_rel_prop(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_node_by_rel_prop(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    detach_snst_rel_by_dst_delete(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn detach_snst_rel_by_dst_delete_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    detach_snst_rel_by_dst_delete(client).await;

    assert!(server.shutdown().is_ok());
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn detach_snst_rel_by_dst_delete(mut client: Client) {
    let _u0 = client
        .create_node(
            "User",
            "__typename",
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
            Some("1234".to_string()),
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
