mod setup;

use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_new_nodes<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, { "dst": { "Commit": { "NEW": { "hash": "11111" } } } } ] })
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity0 = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity0.len(), 2);

    assert!(activity0
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity0
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));

    let p1 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project One", "activity": [ { "dst": { "Commit": { "NEW": { "hash": "22222" } } } }, { "dst": { "Commit": { "NEW": { "hash": "33333" } } } } ] })
        )
        .await
        .unwrap();

    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project One");

    assert!(p1.get("activity").unwrap().is_array());
    let activity1 = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity1.len(), 2);

    assert!(activity1
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity1
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity1
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
    assert!(activity1
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "33333"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 2);

    assert!(projects_a
        .iter()
        .any(|p| p.get("id").unwrap() == p0.get("id").unwrap()));
    assert!(projects_a
        .iter()
        .any(|p| p.get("name").unwrap() == "Project Zero"));
    assert!(projects_a
        .iter()
        .any(|p| p.get("id").unwrap() == p1.get("id").unwrap()));
    assert!(projects_a
        .iter()
        .any(|p| p.get("name").unwrap() == "Project One"));

    let p3 = &projects_a[0];
    assert!(p3.is_object());
    assert_eq!(p3.get("__typename").unwrap(), "Project");

    assert!(p3.get("activity").unwrap().is_array());
    let activity2 = p3.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity2.len(), 2);

    assert!(activity2
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity2
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));

    let p4 = &projects_a[1];
    assert!(p4.is_object());
    assert_eq!(p4.get("__typename").unwrap(), "Project");

    assert!(p4.get("activity").unwrap().is_array());
    let activity3 = p4.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity3.len(), 2);

    assert!(activity3
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity3
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
}

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_existing_nodes<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let c0 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "00000"}),
        )
        .await
        .unwrap();
    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "11111"}),
        )
        .await
        .unwrap();
    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "dst": { "Commit": { "EXISTING": { "hash": {"EQ": "00000" }} } } }, { "dst": { "Commit": {"EXISTING": { "hash": {"EQ": "11111" }}}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity0 = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity0.len(), 2);

    assert!(activity0
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity0
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity0
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id dst { ...on Commit { __typename id hash } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());

    assert!(p1.get("activity").unwrap().is_array());
    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", 
                    "activity": [ {"repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                  {"repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"activity": {"repo": {"EQ": "Repo Zero"}}}))
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship dst object
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"activity": {"dst": {"Commit": {"hash": {"EQ": "11111"}}}}}))
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p1 = &projects_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    let activity = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

/// Passes if the create mutation succeeds and the rel query returns an empty list. Added because
/// of a bug that caused all rels with the same edge label to be returned, even if the source node
/// type was different, under Gremlin.
#[wg_test]
#[allow(dead_code)]
async fn read_rel_by_edge_label<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    client
        .create_node(
            "Portfolio",
            "__typename id",
            Some("1234"),
            &json!({
                "activity": {
                    "dst": {
                        "Commit": {
                            "NEW": {
                                "hash": "111111"
                            }
                        }
                    }
                }
            }),
        )
        .await
        .unwrap();

    let activities = client
        .read_rel("Project", "activity", "__typename id", Some("1234"), None)
        .await
        .unwrap();

    assert!(activities.is_array());
    let activities_a = activities.as_array().unwrap();
    assert_eq!(activities_a.len(), 0);
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_new_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"NEW": {"hash": "22222"}}}}}})
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 3);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_existing_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let _c0 = client
        .create_node(
            "Commit",
            "__typename id hash",
            Some("1234"),
            &json!({"hash": "22222"}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"activity": {"ADD": {"dst": {"Commit": {"EXISTING": {"hash": {"EQ": "22222"}}}}}}})
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 3);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_relationship<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"activity": {"UPDATE": {"MATCH": {"dst": {"Commit": {"hash": {"EQ": "00000"}}}}, "SET": {"repo": "Repo 0"}}}})
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu.iter().any(|a| a.get("repo").unwrap() == "Repo 0"));
    assert!(activityu
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
}

/// Passes if warpgrapher can delete a relationship by its properties
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_relationship_by_rel_props<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One", "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"activity": {"DELETE": {"MATCH": {"repo": {"EQ": "Repo Zero"}}}}})
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 1);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activityu
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
}

/// Passes if warpgrapher can delete a relationship by the properties of the dst object
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_relationship_by_dst_props<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activity = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    let pu = client
        .update_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            Some(&json!({"name": {"EQ":"Project Zero"}})),
            &json!({"activity": {"DELETE": {"MATCH": {"dst": {"Commit": {"hash": {"EQ": "00000"}}}}}}})
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("activity").unwrap().is_array());
    let activityu = p1.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 1);

    assert!(activityu
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activityu
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "00000"));
    assert!(activityu
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activityu
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activityu
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
}

/// Passes if warpgrapher can delete a node by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnst_rel_property<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activityu = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({"activity": {"dst": {"Commit": {"hash": {"EQ": "00000"}}}}})),
            Some(&json!({"activity": [{"MATCH": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }",  Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}

/// Passes if warpgrapher can delete a node by the properties of the dst object at a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnst_dst_property<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }",  Some("1234"),
            &json!({"name": "Project Zero", "activity": [ { "repo": "Repo Zero", "dst": { "Commit": { "NEW": { "hash": "00000" } } } }, 
                                                          { "repo": "Repo One",  "dst": { "Commit": {"NEW": { "hash": "11111" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("activity").unwrap().is_array());
    let activityu = p0.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activityu.len(), 2);

    client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({"activity": {"dst": {"Commit": {"hash": {"EQ": "00000"}}}}})),
            Some(&json!({"activity": [{"MATCH": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name activity { __typename id repo dst { ...on Commit { __typename id hash } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}
