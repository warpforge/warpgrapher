mod setup;

use serde_json::json;
use setup::AppRequestCtx;
use warpgrapher::client::Client;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_rel(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let o0a = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}", Some("1234"),
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"props": {"since": "yesterday"}, "dst": {"User": {"$NEW": {"name": "User Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(o0a.is_array());
    assert_eq!(o0a.as_array().unwrap().len(), 1);
    let o0 = o0a.as_array().unwrap().iter().next().unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("props").unwrap().get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
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
    assert!(owner.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");
}

/// Passes if warpgrapher does not create the destination node if it can't find any source nodes
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn snst_without_src_no_new_dst(mut client: Client<AppRequestCtx>) {
    let o0 = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}", Some("1234"),
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({"props": {"since": "yesterday"}, "dst": {"User": {"$NEW": {"name": "User Zero"}}}}),
        )
        .await
        .unwrap();

    assert!(o0.is_array());
    assert_eq!(o0.as_array().unwrap().len(), 0);

    let users = client
        .read_node(
            "User",
            "id name",
            Some("1234"),
            Some(&json!({"name": {"EQ": "User Zero"}})),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    assert_eq!(users_a.len(), 0)
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_rel_existing_node(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({"name": "Project Zero"}),
        )
        .await
        .unwrap();

    let _u0 = client
        .create_node(
            "User",
            "__typename name",
            Some("1234"),
            &json!({"name": "User Zero"}),
        )
        .await
        .unwrap();

    let o0a = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            &json!({"name": {"EQ": "Project Zero"}}),
            &json!({
                "props": {"since": "yesterday"},
                "dst": {"User": {"$EXISTING": {"name": {"EQ": "User Zero"}}}}
            }),
        )
        .await
        .unwrap();

    assert!(o0a.is_array());
    assert_eq!(o0a.as_array().unwrap().len(), 1);
    let o0 = o0a.as_array().unwrap().iter().next().unwrap();

    assert!(o0.get("__typename").unwrap() == "ProjectOwnerRel");
    assert!(o0.get("dst").unwrap().get("__typename").unwrap() == "User");
    assert!(o0.get("dst").unwrap().get("name").unwrap() == "User Zero");
    assert!(o0.get("props").unwrap().get("since").unwrap() == "yesterday");

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "yesterday");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_rel_props(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"props": {"since": {"EQ": "yesterday"}}})),
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
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_src_props(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
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
        .any(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_dst_props(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .read_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
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
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("__typename").unwrap() == "User"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_rel_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                  "props": {"since": "yesterday"},
                  "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"props": {"since": {"EQ": "yesterday"}}})),
            &json!({"props": {"since": "today"}}),
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
    assert!(owner
        .iter()
        .any(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .any(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_src_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            &json!({"props": {"since": "today"}}),
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
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_dst_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                  }
            }),
        )
        .await
        .unwrap();

    let o0 = client
        .update_rel(
            "Project",
            "owner",
            "__typename props {since} dst {...on User{__typename name}}",
            Some("1234"),
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
            &json!({"props": {"since": "today"}}),
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
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() == "today"));
    assert!(owner
        .iter()
        .all(|o| o.get("props").unwrap().get("since").unwrap() != "yesterday"));
    assert!(owner
        .iter()
        .all(|o| o.get("dst").unwrap().get("name").unwrap() == "User Zero"));

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("props").unwrap().get("since").unwrap() != "yesterday");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User Zero");
}

#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                      "props": {"since": "yesterday"},
                      "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                    }
            }),
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some("1234"),
            Some(&json!({"props": {"since": {"EQ": "yesterday"}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
async fn delete_snst_rel_by_dst_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some("1234"),
            Some(&json!({"dst": {"User": {"name": {"EQ": "User Zero"}}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
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
async fn delete_snst_rel_by_src_prop(mut client: Client<AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"$NEW": {"name": "User Zero"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _p1 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project One",
                "owner": {
                    "props": {"since": "today"},
                    "dst": {"User": {"$NEW": {"name": "User One"}}}
                }
            }),
        )
        .await
        .unwrap();

    let _o0 = client
        .delete_rel(
            "Project",
            "owner",
            Some("1234"),
            Some(&json!({"src": {"Project": {"name": {"EQ": "Project Zero"}}}})),
            None,
            None,
        )
        .await
        .unwrap();

    let projects0 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
            Some(&json!({"name": {"EQ": "Project One"}})),
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
    assert!(owner.get("props").unwrap().get("since").unwrap() == "today");
    assert!(owner.get("dst").unwrap().get("name").unwrap() == "User One");
}
