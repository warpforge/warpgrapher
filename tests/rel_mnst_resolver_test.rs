mod setup;

use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_new_rel<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{"repo": "Repo Zero", "dst": {"Commit": {"NEW": {"hash": "00000"}}}},
                    {"repo": "Repo One", "dst": {"Commit": {"NEW": {"hash": "11111"}}}}]),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));

    let projects = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_rel_existing_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node("Project", "name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();

    let c0 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "00000"}), None)
        .await
        .unwrap();

    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "00000");

    let c1 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "11111"}), None)
        .await
        .unwrap();

    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "11111");

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{"repo": "Repo Zero", "dst": {"Commit": {"EXISTING": {"hash": {"EQ": "00000"}}}}},
                    {"repo": "Repo One", "dst": {"Commit": {"EXISTING": {"hash": {"EQ": "11111"}}}}}]),
            None
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));

    let projects = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
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
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnst_unique_ids<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node("Project", "name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();

    let c0 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "1"}), None)
        .await
        .unwrap();

    assert!(c0.is_object());
    assert_eq!(c0.get("__typename").unwrap(), "Commit");
    assert_eq!(c0.get("hash").unwrap(), "1");

    let c1 = client
        .create_node("Commit", "__typename hash", &json!({"hash": "2"}), None)
        .await
        .unwrap();

    assert!(c1.is_object());
    assert_eq!(c1.get("__typename").unwrap(), "Commit");
    assert_eq!(c1.get("hash").unwrap(), "2");

    let a0 = client
        .create_rel(
            "Project",
            "activity",
            "__typename id repo dst{...on Commit{__typename hash}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!([{"repo": "Repo Zero", "dst": {"Commit": {"EXISTING": {"hash": {"GT": "0"}}}}}]),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "1"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "2"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Zero"));

    let first_id = activity[0].get("id").unwrap();
    let second_id = activity[1].get("id").unwrap();

    assert_ne!(first_id, second_id);

    let projects = client
        .read_node(
            "Project",
            "activity{__typename id repo dst{...on Commit{__typename hash}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "1"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "2"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Zero"));

    let first_id = activity[0].get("id").unwrap();
    let second_id = activity[1].get("id").unwrap();

    assert_ne!(first_id, second_id);
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            Some(&json!({"repo": {"EQ": "Repo Zero"}})),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

/// Passes if reading rels with specific ordering works
#[wg_test]
#[allow(dead_code)]
async fn read_in_order<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let r0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst { ...on Commit{ __typename hash } }",
            None,
            Some(&json!({"sort": [{"direction": "ascending", "orderBy": "repo"}]})),
        )
        .await
        .unwrap();

    let a0 = r0.as_array().unwrap();
    assert_eq!(a0.len(), 2);
    assert_eq!(a0[0].get("repo").unwrap(), "Repo One");
    assert_eq!(a0[1].get("repo").unwrap(), "Repo Zero");

    let r1 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst { ...on Commit{ __typename hash } }",
            None,
            Some(&json!({"sort": [{"direction": "descending", "orderBy": "repo"}]})),
        )
        .await
        .unwrap();

    let a1 = r1.as_array().unwrap();
    assert_eq!(a1.len(), 2);
    assert_eq!(a1[0].get("repo").unwrap(), "Repo Zero");
    assert_eq!(a1[1].get("repo").unwrap(), "Repo One");
}

/// Passes if reading rels with specific ordering by destination property works
#[wg_test]
#[allow(dead_code)]
async fn read_in_dst_order<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let r0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst { ...on Commit{ __typename hash } }",
            None,
            Some(&json!({"sort": [{"direction": "ascending", "orderBy": "dst:hash"}]})),
        )
        .await
        .unwrap();

    let a0 = r0.as_array().unwrap();
    assert_eq!(a0.len(), 2);
    assert!(a0[0].get("repo").unwrap() == "Repo Zero");
    assert!(a0[1].get("repo").unwrap() == "Repo One");

    let r1 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst { ...on Commit{ __typename hash } }",
            None,
            Some(&json!({"sort": [{"direction": "descending", "orderBy": "dst:hash"}]})),
        )
        .await
        .unwrap();

    let a1 = r1.as_array().unwrap();
    assert_eq!(a1.len(), 2);
    assert!(a1[0].get("repo").unwrap() == "Repo One");
    assert!(a1[1].get("repo").unwrap() == "Repo Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_src_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{ __typename hash}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
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
        .any(|a| a.get("repo").unwrap() == "Repo One"));
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

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnst_rel_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .read_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            Some(&json!({"dst": {"Commit": {"hash": {"EQ": "00000"}}}})),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            Some(&json!({"repo": {"EQ": "Repo Zero"}})),
            &json!({"repo": "Repo Two"}),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            &json!({"repo": "Repo Two"}),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnst_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let a0 = client
        .update_rel(
            "Project",
            "activity",
            "__typename repo dst{...on Commit{__typename hash}}",
            Some(&json!({"dst": {"Commit": {"hash": {"EQ": "00000"}}}})),
            &json!({"repo": "Repo Two"}),
            None,
        )
        .await
        .unwrap();

    let activity = a0.as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo Zero"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo One"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "11111"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                      "repo": "Repo Zero",
                      "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                      "repo": "Repo One",
                      "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"repo": {"EQ": "Repo One"}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "11111"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename id name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                      "repo": "Repo Zero",
                      "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                      "repo": "Repo One",
                      "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"dst": {"Commit": {"hash": {"EQ": "11111"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 1);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() == "Repo Zero"));
    assert!(activity
        .iter()
        .all(|a| a.get("repo").unwrap() != "Repo One"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() == "00000"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("hash").unwrap() != "11111"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnst_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "activity": [
                    {
                        "repo": "Repo Zero",
                        "dst": {"Commit": {"NEW": {"hash": "00000"}}}
                    },
                    {
                        "repo": "Repo One",
                        "dst": {"Commit": {"NEW": {"hash": "11111"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project One",
                "activity": [
                    {
                        "repo": "Repo Two",
                        "dst": {"Commit": {"NEW": {"hash": "22222"}}}
                    },
                    {
                        "repo": "Repo Three",
                        "dst": {"Commit": {"NEW": {"hash": "33333"}}}
                    }
                ]
            }),
            None,
        )
        .await
        .unwrap();

    let _a0 = client
        .delete_rel(
            "Project",
            "activity",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None,
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "activity{__typename repo dst{...on Commit{__typename hash}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None,
        )
        .await
        .unwrap();

    let projects_a = projects0.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 0);

    let projects_a = projects1.as_array().unwrap();
    let project = &projects_a[0];

    let activity = project.get("activity").unwrap().as_array().unwrap();
    assert_eq!(activity.len(), 2);

    assert!(activity.iter().all(|a| a.is_object()));
    assert!(activity
        .iter()
        .all(|a| a.get("__typename").unwrap() == "ProjectActivityRel"));
    assert!(activity
        .iter()
        .all(|a| a.get("dst").unwrap().get("__typename").unwrap() == "Commit"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Two"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "22222"));
    assert!(activity
        .iter()
        .any(|a| a.get("repo").unwrap() == "Repo Three"));
    assert!(activity
        .iter()
        .any(|a| a.get("dst").unwrap().get("hash").unwrap() == "33333"));
}
