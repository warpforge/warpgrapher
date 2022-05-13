mod setup;

use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_rel<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let o0a = client
        .create_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"since": "yesterday", "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
            None,
        )
        .await
        .unwrap();

    assert!(o0a.is_array());
    assert_eq!(o0a.as_array().unwrap().len(), 1);
    let o0 = o0a.as_array().unwrap().iter().next().unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
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
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("since").unwrap() == "yesterday");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_rel_with_id<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let o0a = client
        .create_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"id": "6d5dca5e-3082-4152-8d25-a16beace1e90", "since": "yesterday", "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
            None
        )
        .await
        .unwrap();

    assert!(o0a.is_array());
    assert_eq!(o0a.as_array().unwrap().len(), 1);
    let o0 = o0a.as_array().unwrap().iter().next().unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename id since dst{...on User{__typename name}}}",
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
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("id").unwrap() == "6d5dca5e-3082-4152-8d25-a16beace1e90");
    assert!(owner.get("since").unwrap() == "yesterday");
}

/// Passes if warpgrapher does not create the destination node if it can't find any source nodes
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn snst_without_src_no_new_dst<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let o0 = client
        .create_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"since": "yesterday", "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
            None,
        )
        .await
        .unwrap();

    assert!(o0.is_array());
    assert_eq!(o0.as_array().unwrap().len(), 0);

    let users = client
        .read_node(
            "User",
            "id name",
            Some(&json!({"name": {"EQ": "User Zero"}})),
            None,
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    assert_eq!(users_a.len(), 0)
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_rel_existing_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({"name": "Project Zero"}),
            None,
        )
        .await
        .unwrap();

    let _u0 = client
        .create_node(
            "User",
            "__typename name",
            &json!({"name": "User Zero"}),
            None,
        )
        .await
        .unwrap();

    let o0a = client
        .create_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({
                "since": "yesterday",
                "dst": {"User": {"EXISTING": {"name": {"EQ": "User Zero"}}}}
            }),
            None,
        )
        .await
        .unwrap();

    assert!(o0a.is_array());
    assert_eq!(o0a.as_array().unwrap().len(), 1);
    let o0 = o0a.as_array().unwrap().iter().next().unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects.as_array().unwrap();
    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("since").unwrap() == "yesterday");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            Some(&json!({"since": {"EQ": "yesterday"}})),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_src_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner.iter().any(|o| o.get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                  "since": "yesterday",
                  "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            Some(&json!({"since": {"EQ": "yesterday"}})),
            &json!({"since": "today"}),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner.iter().any(|o| o.get("since").unwrap() == "today"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .any(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
            None,
        )
        .await
        .unwrap();

    let projects_a = projects1.as_array().unwrap();

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("since").unwrap() != "yesterday");
    assert!(owner.get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename since dst{...on User{__typename name}}",
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            &json!({"since": "today"}),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() == "today"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
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
    assert!(owner.get("since").unwrap() == "today");
    assert!(owner.get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                  }
            }),
            None,
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename since dst {...on User{__typename name}}",
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
            &json!({"since": "today"}),
            None,
        )
        .await
        .unwrap();

    let owner = o0.as_array().unwrap();
    assert_eq!(owner.len(), 1);

    assert!(owner.iter().all(|o| o.is_object()));
    assert!(owner
        .iter()
        .all(|o| o.get("__typename").unwrap() == "ProjectOwnerRel"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() == "today"));
    assert!(owner.iter().all(|o| o.get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
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
    assert!(owner.get("since").unwrap() == "today");
    assert!(owner.get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                      "since": "yesterday",
                      "dst": {"User": {"NEW": {"name": "User Zero"}}}
                    }
            }),
            None,
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some(&json!({"since": {"EQ": "yesterday"}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_dst_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            None,
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_src_prop<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            &json!({
                "name": "Project Zero",
                "owner": {
                    "since": "yesterday",
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
                }
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
                "owner": {
                    "since": "today",
                    "dst": {"User": {"NEW": {"name": "User One"}}}
                }
            }),
            None,
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
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
            "owner{__typename since dst{...on User{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            None,
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename since dst{...on User{__typename name}}}",
            Some(&json!({"name": {"EQ": "Project One"}})),
            None,
        )
        .await
        .unwrap();

    assert!(projects0.is_array());
    let projects_a = projects0.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    assert_eq!(
        project.get("owner").unwrap(),
        &serde_json::value::Value::Null
    );

    assert!(projects1.is_array());
    let projects_a = projects1.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let project = &projects_a[0];
    let owner = project.get("owner").unwrap();

    assert!(owner.is_object());
    assert!(owner.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User One");
}
