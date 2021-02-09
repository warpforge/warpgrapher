mod setup;

use serde_json::json;
use setup::AppRequestCtx;
use warpgrapher::client::Client;
use warpgrapher_macros::wg_test;

async fn create_test_fixtures(client: &mut Client<AppRequestCtx>) {
    client
        .create_node("Project", "id", None, &json!({"name": "STARDUST"}))
        .await
        .unwrap();
    client
        .create_node("Project", "id", None, &json!({"name": "STARSCREAM"}))
        .await
        .unwrap();
    client
        .create_node("Project", "id", None, &json!({"name": "BLACKWING"}))
        .await
        .unwrap();
    client
        .create_node(
            "Feature",
            "id",
            None,
            &json!({"name": "Kyber Prism", "points": 10}),
        )
        .await
        .unwrap();
    client
        .create_node(
            "Feature",
            "id",
            None,
            &json!({"name": "Kyber Refractor", "points": 15}),
        )
        .await
        .unwrap();
    client
        .create_node(
            "Feature",
            "id",
            None,
            &json!({"name": "CINDER Orbital Platforms", "points": 7}),
        )
        .await
        .unwrap();
    client
        .create_node(
            "Feature",
            "id",
            None,
            &json!({"name": "CINDER Particle Weapons", "points": 20}),
        )
        .await
        .unwrap();
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_read_node_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    // EQ
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "EQ": "STARDUST" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 1);
    assert_eq!(results_array[0].get("name").unwrap(), "STARDUST");

    // NOTEQ
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "NOTEQ": "STARDUST" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARSCREAM"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));

    // CONTAINS
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "CONTAINS" : "STAR" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARSCREAM"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARDUST"));

    // CONTAINS
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "CONTAINS": "BLACK" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 1);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));

    // NOTCONTAINS
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "NOTCONTAINS" : "STARDUST" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARSCREAM"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));

    // NOTCONTAINS
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "NOTCONTAINS": "STAR" }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 1);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));

    // IN
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "IN": ["STARDUST", "STARSCREAM", "BLACKWING"] }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 3);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARSCREAM"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARDUST"));

    // NOTIN
    let results = client
        .read_node(
            "Project",
            "__typename id name",
            None,
            Some(&json!({"name": { "NOTIN": ["STARDUST"] }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "BLACKWING"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "STARSCREAM"));

    // GT
    let results = client
        .read_node(
            "Feature",
            "__typename id name",
            None,
            Some(&json!({"points": { "GT": 10 }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "Kyber Refractor"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "CINDER Particle Weapons"));

    // GTE
    let results = client
        .read_node(
            "Feature",
            "__typename id name",
            None,
            Some(&json!({"points": { "GTE": 10 }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 3);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "Kyber Prism"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "Kyber Refractor"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "CINDER Particle Weapons"));

    // LT
    let results = client
        .read_node(
            "Feature",
            "__typename id name",
            None,
            Some(&json!({"points": { "LT": 10 }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 1);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "CINDER Orbital Platforms"));

    // LTE
    let results = client
        .read_node(
            "Feature",
            "__typename id name",
            None,
            Some(&json!({"points": { "LTE": 10 }})),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "CINDER Orbital Platforms"));
    assert!(results_array
        .iter()
        .any(|i| i.get("name").unwrap() == "Kyber Prism"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_create_node_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    // create node with nested rels to 2 rels
    let p1 = client
        .create_node(
            "Project",
            "__typename 
            issues { 
                dst { 
                    ... on Feature { 
                        name 
                    } 
                } 
            }",
            None,
            &json!({
                "name": "CINDER",
                "issues": {
                    "dst": {
                        "Feature": {
                            "EXISTING": {
                                "name": {
                                    "CONTAINS": "CINDER"
                                }
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();
    assert!(p1.is_object());
    assert!(p1.get("issues").unwrap().is_array());
    let issues1 = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues1.len(), 2);
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "CINDER Orbital Platforms"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "CINDER Particle Weapons"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_update_node_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    // update node to create nested rels to 2 rels using comparisons
    let results = client
        .update_node(
            "Project",
            "__typename 
            issues { 
                dst { 
                    ... on Feature { 
                        name 
                    } 
                } 
            }",
            None,
            Some(&json!({
                "name": { "EQ": "STARDUST" }
            })),
            &json!({
                "issues": {
                    "ADD": {
                        "dst": {
                            "Feature": {
                                "EXISTING": {
                                    "name": {
                                        "CONTAINS": "Kyber"
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let p1 = &results.as_array().unwrap()[0];
    assert!(p1.is_object());
    assert!(p1.get("issues").unwrap().is_array());
    let issues1 = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues1.len(), 2);
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Kyber Prism"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Kyber Refractor"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_delete_node_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    // delete nodes
    let results = client
        .delete_node("Feature", None, Some(&json!({"points": {"GT": 10 }})), None)
        .await
        .unwrap();
    assert_eq!(results, 2);
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_read_rel_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    let _results = client
        .update_node(
            "Project",
            "__typename 
            issues { 
                dst { 
                    ... on Feature { 
                        name 
                    } 
                } 
            }",
            None,
            Some(&json!({
                "name": { "EQ": "STARDUST" }
            })),
            &json!({
                "issues": {
                    "ADD": {
                        "props": {
                            "since": "5 BBY"
                        },
                        "dst": {
                            "Feature": {
                                "EXISTING": {
                                    "name": {
                                        "CONTAINS": "Kyber"
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    // query node by rel comparison match
    let results = client
        .read_node(
            "Project",
            "__typename 
            id 
            name",
            None,
            Some(&json!({
                "issues": 
                    {
                        "props": {
                            "since": { "IN": ["5 BBY", "10 BBY", "15 BBY"]}
                        }
                    }
                
            })),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 1);

    // query rel by rel comparison match
    let results = client
        .read_rel(
            "Project",
            "issues",
            "__typename 
            dst {
                ... on Feature {
                    __typename
                    name
                }
            }",
            None,
            Some(&json!({
                "props": {
                    "since": { "EQ": "5 BBY" }
                }
            })),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Kyber Refractor"));
    assert!(results_array
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Kyber Prism"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_update_rel_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    let _results = client
        .update_node(
            "Project",
            "__typename 
            issues { 
                dst { 
                    ... on Feature { 
                        name 
                    } 
                } 
            }",
            None,
            Some(&json!({
                "name": { "EQ": "STARDUST" }
            })),
            &json!({
                "issues": {
                    "ADD": {
                        "props": {
                            "since": "5 BBY"
                        },
                        "dst": {
                            "Feature": {
                                "EXISTING": {
                                    "name": {
                                        "CONTAINS": "Kyber"
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let _results = client
        .update_rel(
            "Project",
            "issues",
            "__typename 
            dst {
                ... on Feature {
                    __typename
                    name
                }
            }",
            None,
            Some(&json!({
                "props": {
                    "since": { "EQ": "5 BBY" }
                }
            })),
            &json!({
                "props": {
                    "since": "0 BBY"
                }
            }),
        )
        .await
        .unwrap();

    let results = client
        .read_rel(
            "Project",
            "issues",
            "__typename
            props {
                since
            }
            dst {
                ... on Feature {
                    __typename
                    name
                }
            }",
            None,
            Some(&json!({
                "dst": {
                    "Feature": {
                        "name": { "CONTAINS": "Kyber" }
                    }
                }
            })),
        )
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 2);
    assert!(results_array.iter().any(|i| {
        i.get("dst").unwrap().get("name").unwrap() == "Kyber Refractor"
            && i.get("props").unwrap().get("since").unwrap() == "0 BBY"
    }));
    assert!(results_array.iter().any(|i| {
        i.get("dst").unwrap().get("name").unwrap() == "Kyber Prism"
            && i.get("props").unwrap().get("since").unwrap() == "0 BBY"
    }));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn test_delete_rel_comparison(mut client: Client<AppRequestCtx>) {
    create_test_fixtures(&mut client).await;

    // create rels
    let _results = client
        .update_node(
            "Project",
            "__typename 
            issues { 
                dst { 
                    ... on Feature { 
                        name 
                    } 
                } 
            }",
            None,
            Some(&json!({
                "name": { "EQ": "STARDUST" }
            })),
            &json!({
                "issues": {
                    "ADD": {
                        "props": {
                            "since": "5 BBY"
                        },
                        "dst": {
                            "Feature": {
                                "EXISTING": {
                                    "name": {
                                        "CONTAINS": "Kyber"
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    // delete rels by comparison match
    client
        .delete_rel(
            "Project",
            "issues",
            None,
            Some(&json!({
                "props": {
                    "since": { "CONTAINS": "BBY" }
                }
            })),
            None,
            None,
        )
        .await
        .unwrap();

    // verify rels where deleted
    let results = client
        .read_rel("Project", "issues", "__typename", None, None)
        .await
        .unwrap();
    let results_array = results.as_array().unwrap();
    assert_eq!(results_array.len(), 0);
}
