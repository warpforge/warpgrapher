mod setup;

use serde_json::json;
#[cfg(feature = "cosmos")]
use setup::cosmos_test_client;
#[cfg(feature = "gremlin")]
use setup::gremlin_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(any(feature = "cosmos", feature = "gremlin", feature = "neo4j"))]
use setup::{clear_db, init};
use setup::{AppGlobalCtx, AppRequestCtx};
use warpgrapher::client::Client;

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snst_new_rel_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_new_rel(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snst_new_rel_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_new_rel(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snst_new_rel_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_new_rel(client).await;
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_new_rel(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
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
            &json!({"name": "Project Zero"}),
            &json!({"props": {"since": "yesterday"}, "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn snst_without_src_no_new_dst_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    snst_without_src_no_new_dst(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn snst_without_src_no_new_dst_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    snst_without_src_no_new_dst(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn snst_without_src_no_new_dst_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    snst_without_src_no_new_dst(client).await;
}

/// Passes if warpgrapher does not create the destination node if it can't find any source nodes
#[allow(clippy::cognitive_complexity, dead_code)]
async fn snst_without_src_no_new_dst(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let o0 = client
        .create_rel(
            "Project",
            "owner",
            "__typename props{since} dst{...on User{__typename name}}", Some("1234"),
            &json!({"name": "Project Zero"}),
            &json!({"props": {"since": "yesterday"}, "dst": {"User": {"NEW": {"name": "User Zero"}}}}),
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
            Some(&json!({"name": "User Zero"})),
        )
        .await
        .unwrap();

    let users_a = users.as_array().unwrap();
    assert_eq!(users_a.len(), 0)
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snst_rel_existing_node_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_rel_existing_node(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snst_rel_existing_node_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_rel_existing_node(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snst_rel_existing_node_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    create_snst_rel_existing_node(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_snst_rel_existing_node(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
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
            &json!({"name": "Project Zero"}),
            &json!({
                "props": {"since": "yesterday"},
                "dst": {"User": {"EXISTING": {"name": "User Zero"}}}
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_snst_rel_by_rel_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_rel_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snst_rel_by_rel_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_rel_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snst_rel_by_rel_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_rel_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_rel_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"props": {"since": "yesterday"}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snst_rel_by_src_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_src_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snst_rel_by_src_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_src_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_snst_rel_by_src_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_src_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_src_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_snst_rel_by_dst_props_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_dst_props(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn read_snst_rel_by_dst_props_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_dst_props(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn read_snst_rel_by_dst_props_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    read_snst_rel_by_dst_props(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_snst_rel_by_dst_props(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snst_rel_by_rel_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_rel_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn create_snst_rel_by_rel_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_rel_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn create_snst_rel_by_rel_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_rel_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_rel_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                  "props": {"since": "yesterday"},
                  "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"props": {"since": "yesterday"}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snst_rel_by_src_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_src_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snst_rel_by_src_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_src_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_snst_rel_by_src_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_src_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_src_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_snst_rel_by_dst_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_dst_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn update_snst_rel_by_dst_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_dst_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn update_snst_rel_by_dst_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    update_snst_rel_by_dst_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_snst_rel_by_dst_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_rel_by_rel_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_rel_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_rel_by_del_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_rel_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snst_rel_by_del_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_rel_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_rel_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                      "props": {"since": "yesterday"},
                      "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"props": {"since": "yesterday"}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_rel_by_dst_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_dst_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_rel_by_dst_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_dst_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snst_rel_by_dst_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_dst_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_dst_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
            Some(&json!({"dst": {"User": {"name": "User Zero"}}})),
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

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_snst_rel_by_src_prop_neo4j() {
    init();
    clear_db().await;

    let client = neo4j_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_src_prop(client).await;
}

#[cfg(feature = "cosmos")]
#[tokio::test]
async fn delete_snst_rel_by_src_prop_cosmos() {
    init();
    clear_db().await;

    let client = cosmos_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_src_prop(client).await;
}

#[cfg(feature = "gremlin")]
#[tokio::test]
async fn delete_snst_rel_by_src_prop_gremlin() {
    init();
    clear_db().await;

    let client = gremlin_test_client("./tests/fixtures/minimal.yml").await;
    delete_snst_rel_by_src_prop(client).await;
}

#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_snst_rel_by_src_prop(mut client: Client<AppGlobalCtx, AppRequestCtx>) {
    let _p0 = client
        .create_node(
            "Project",
            "__typename name",
            Some("1234"),
            &json!({
                "name": "Project Zero",
                "owner": {
                    "props": {"since": "yesterday"},
                    "dst": {"User": {"NEW": {"name": "User Zero"}}}
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
                    "dst": {"User": {"NEW": {"name": "User One"}}}
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
            Some(&json!({"src": {"Project": {"name": "Project Zero"}}})),
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
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    let projects1 = client
        .read_node(
            "Project",
            "owner{__typename props{since} dst{...on User{__typename name}}}",
            Some("1234"),
            Some(&json!({"name": "Project One"})),
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
