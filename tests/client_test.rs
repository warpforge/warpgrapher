mod setup;

#[cfg(feature = "cypher")]
use serde_json::json;
#[cfg(feature = "cypher")]
use setup::{clear_db, cypher_test_client, init};

#[cfg(feature = "cypher")]
#[tokio::test]
async fn client_node_crud() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/minimal.yml").await;

    let p0 = client
        .create_node(
            "Project",
            "id name description status",
            &json!({"name": "MJOLNIR", "description": "Advanced armor", "status": "PENDING"}),
            None,
        )
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("name").unwrap(), "MJOLNIR");
    assert_eq!(p0.get("description").unwrap(), "Advanced armor");
    assert_eq!(p0.get("status").unwrap(), "PENDING");

    let projects = client
        .read_node("Project", "id name status", None, None)
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);
    assert_eq!(projects_a[0].get("status").unwrap(), "PENDING");

    let pu = client
        .update_node(
            "Project",
            "__typename id name status",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            &json!({"status": "ACTIVE"}),
            None,
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a.len(), 1);
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "MJOLNIR");
    assert_eq!(pu_a[0].get("status").unwrap(), "ACTIVE");

    let u_projects = client
        .read_node("Project", "id status", None, None)
        .await
        .unwrap();

    assert!(u_projects.is_array());
    let u_projects_a = u_projects.as_array().unwrap();
    assert_eq!(u_projects_a.len(), 1);
    assert_eq!(u_projects_a[0].get("status").unwrap(), "ACTIVE");

    let pd = client
        .delete_node(
            "Project",
            Some(&json!({"name": {"EQ": "MJOLNIR"}})),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(pd, 1);

    let d_projects = client
        .read_node("Project", "id status", None, None)
        .await
        .unwrap();

    assert!(d_projects.is_array());
    let d_projects_a = d_projects.as_array().unwrap();
    assert_eq!(d_projects_a.len(), 0);
}

#[cfg(feature = "cypher")]
#[tokio::test]
async fn client_rel_crud() {
    init();
    clear_db().await;
    let mut client = cypher_test_client("./tests/fixtures/minimal.yml").await;

    client
        .create_node("Project", "id name", &json!({"name": "Project Zero"}), None)
        .await
        .unwrap();
    client
        .create_node("Bug", "id name", &json!({"name": "Bug Zero"}), None)
        .await
        .unwrap();

    let results = client
        .create_rel(
            "Project",
            "issues",
            "id 
        since 
        src { 
            id 
            name 
        } 
        dst { 
            ...on Bug { 
                id 
                name 
            } 
        }",
            &json!({
                "name": {"EQ": "Project Zero"}
            }),
            &json!([
                {
                    "since": "2000",
                    "dst": {"Bug": {"EXISTING": {"name": {"EQ": "Bug Zero"}} }}
                }
            ]),
            None,
        )
        .await
        .unwrap();

    assert!(results.is_array());
    let r0 = &results[0];
    assert!(r0.is_object());
    assert_eq!(r0.get("since").unwrap(), "2000");
    assert_eq!(r0.get("src").unwrap().get("name").unwrap(), "Project Zero");
    assert_eq!(r0.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let rels = client
        .read_rel("Project", "issues", "id since", None, None)
        .await
        .unwrap();

    assert!(rels.is_array());
    let rels_a = rels.as_array().unwrap();
    assert_eq!(rels_a.len(), 1);
    assert_eq!(rels_a[0].get("since").unwrap(), "2000");

    let ru = client
        .update_rel(
            "Project",
            "issues",
            "id since",
            Some(&json!({"since": { "EQ": "2000"}})),
            &json!({"since": "2010"}),
            None,
        )
        .await
        .unwrap();

    assert!(ru.is_array());
    let ru_a = ru.as_array().unwrap();
    assert_eq!(ru_a.len(), 1);
    assert_eq!(ru_a[0].get("since").unwrap(), "2010");

    let u_rels = client
        .read_rel("Project", "issues", "id since", None, None)
        .await
        .unwrap();

    assert!(u_rels.is_array());
    let u_rels_a = u_rels.as_array().unwrap();
    assert_eq!(u_rels_a.len(), 1);
    assert_eq!(u_rels_a[0].get("since").unwrap(), "2010");

    let rd = client
        .delete_rel(
            "Project",
            "issues",
            Some(&json!({"since": {"EQ": "2010"}})),
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(rd, 1);

    let d_rels = client
        .read_rel("Project", "issues", "id", None, None)
        .await
        .unwrap();

    assert!(d_rels.is_array());
    let d_rels_a = d_rels.as_array().unwrap();
    assert_eq!(d_rels_a.len(), 0);
}
