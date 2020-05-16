mod setup;

use log::trace;
use serde_json::json;
#[cfg(feature = "graphson2")]
use setup::graphson2_test_client;
#[cfg(feature = "neo4j")]
use setup::neo4j_test_client;
#[cfg(feature = "graphson2")]
use setup::server::test_server_graphson2;
#[cfg(feature = "neo4j")]
use setup::server::test_server_neo4j;
#[cfg(any(feature = "graphson2", feature = "neo4j"))]
use setup::{clear_db, init};
use warpgrapher::client::Client;

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn create_mnmt_new_nodes_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    create_mnmt_new_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnmt_new_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnmt_new_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to another new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_new_nodes(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }",Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("issues").unwrap().is_array());
    let issues0 = p0.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues0.len(), 2);

    assert!(issues0
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));

    let p1 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }",Some("1234".to_string()),
            &json!({"name": "Project One", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug One" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature One" }}}} ] }))
        .await
        .unwrap();

    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project One");

    assert!(p1.get("issues").unwrap().is_array());
    let issues1 = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues1.len(), 2);

    assert!(issues1
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug One"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues1
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature One"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 2);

    let p3 = &projects_a[0];
    assert!(p3.is_object());
    assert_eq!(p3.get("__typename").unwrap(), "Project");
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

    assert_eq!(p3.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(p3
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p3
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p3
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));

    let p4 = &projects_a[1];
    assert!(p4.is_object());
    assert_eq!(p4.get("__typename").unwrap(), "Project");

    assert_eq!(p4.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(p4
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p4
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p4
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn create_mnmt_existing_nodes_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    create_mnmt_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn create_mnmt_existing_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    create_mnmt_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_existing_nodes(mut client: Client) {
    let b0 = client
        .create_node(
            "Bug",
            "__typename id name",
            Some("1234".to_string()),
            &json!({"name": "Bug Zero"}),
        )
        .await
        .unwrap();
    assert!(b0.is_object());
    assert_eq!(b0.get("__typename").unwrap(), "Bug");
    assert_eq!(b0.get("name").unwrap(), "Bug Zero");

    let f0 = client
        .create_node(
            "Feature",
            "__typename id name",
            Some("1234".to_string()),
            &json!({"name": "Feature Zero"}),
        )
        .await
        .unwrap();
    assert!(f0.is_object());
    assert_eq!(f0.get("__typename").unwrap(), "Feature");
    assert_eq!(f0.get("name").unwrap(), "Feature Zero");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "EXISTING": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"EXISTING": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("issues").unwrap().is_array());
    let issues0 = p0.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues0.len(), 2);

    assert!(issues0
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
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
    assert_eq!(p1.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn read_mnmt_by_rel_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    read_mnmt_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnmt_by_rel_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnmt_by_rel_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_by_rel_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",             "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),

            Some(&json!({"issues": {"props": {"since": "today"}}}))
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

    trace!("Issues: {:#?}", p1.get("issues").unwrap());
    let issues = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p1
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn read_mnmt_by_dst_props_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    read_mnmt_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn read_mnmt_by_dst_props_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    read_mnmt_by_dst_props(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can query for a relationship by the properties of a destination node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_by_dst_props(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234".to_string()),
            &json!({"name": "Project One", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug One" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature One" }}}} ] }))
        .await
        .unwrap();

    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project One");

    let projects = client
        .read_node(
            "Project", 
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"issues": {"dst": {"Bug": {"name": "Bug Zero"}}}}))
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p2 = &projects_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");
    assert_eq!(p2.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p2.get("name").unwrap(), "Project Zero");

    trace!("Issues: {:#?}", p2.get("issues").unwrap());
    let issues = p2.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues.len(), 2);

    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_mnmt_new_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_mnmt_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_new_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_new_node(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can update a node to add a relationship to a new node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_new_node(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some("1234".to_string()),
            &json!({"name": "Project Zero", "description": "Powered armor"}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
            &json!({"issues": {"ADD": {"dst": { "Bug": { "NEW": {"name": "Bug Zero"}}}}}}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "Project Zero");

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("issues").unwrap().is_array());
    let issuesu = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issuesu.len(), 1);

    let issueu = &issuesu[0];
    assert_eq!(issueu.get("__typename").unwrap(), "ProjectIssuesRel");
    assert_eq!(issueu.get("dst").unwrap().get("__typename").unwrap(), "Bug");
    assert_eq!(issueu.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p2 = &projects_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");
    assert_eq!(p2.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p2.get("issues").unwrap().as_array().unwrap().len(), 1);

    let issue2 = &p2.get("issues").unwrap().as_array().unwrap()[0];
    assert_eq!(issue2.get("__typename").unwrap(), "ProjectIssuesRel");
    assert_eq!(issue2.get("dst").unwrap().get("__typename").unwrap(), "Bug");
    assert_eq!(issue2.get("dst").unwrap().get("name").unwrap(), "Bug Zero");
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_mnmt_existing_nodes_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_mnmt_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_existing_nodes_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_existing_nodes(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can update a node to add a relationship to an existing node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_existing_nodes(mut client: Client) {
    let b0 = client
        .create_node(
            "Bug",
            "__typename id name",
            Some("1234".to_string()),
            &json!({"name": "Bug Zero"}),
        )
        .await
        .unwrap();
    assert!(b0.is_object());
    assert_eq!(b0.get("__typename").unwrap(), "Bug");
    assert_eq!(b0.get("name").unwrap(), "Bug Zero");

    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active", Some("1234".to_string()),
            &json!({"name": "Project Zero", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
            &json!({"issues": {"ADD": {"dst": { "Bug": { "EXISTING": {"name": "Bug Zero"}}}}}}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "Project Zero");

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("issues").unwrap().is_array());
    let issuesu = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issuesu.len(), 1);

    let issueu = &issuesu[0];
    assert_eq!(issueu.get("__typename").unwrap(), "ProjectIssuesRel");
    assert_eq!(issueu.get("dst").unwrap().get("__typename").unwrap(), "Bug");
    assert_eq!(issueu.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p2 = &projects_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");
    assert_eq!(p2.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p2.get("issues").unwrap().as_array().unwrap().len(), 1);

    let issue2 = &p2.get("issues").unwrap().as_array().unwrap()[0];
    assert_eq!(issue2.get("__typename").unwrap(), "ProjectIssuesRel");
    assert_eq!(issue2.get("dst").unwrap().get("__typename").unwrap(), "Bug");
    assert_eq!(issue2.get("dst").unwrap().get("name").unwrap(), "Bug Zero");
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_mnmt_relationship_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_mnmt_relationship_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can update a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_relationship(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("issues").unwrap().is_array());
    let issues0 = p0.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues0.len(), 2);

    assert!(issues0
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
            &json!({"issues": {"UPDATE": {"match": {"dst": { "Feature": { "name": "Feature Zero"}}}, "update": {"props": {"since": "Forever"}}}}}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "Project Zero");

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("issues").unwrap().is_array());
    let issuesu = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issuesu.len(), 2);

    assert!(issuesu
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "Forever"));

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p2 = &projects_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");

    assert_eq!(p2.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "Forever"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn update_only_correct_mnmt_relationship_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    update_only_correct_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn update_only_correct_mnmt_relationship_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    update_only_correct_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher only updates the correct matching relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_only_correct_mnmt_relationship(mut client: Client) {
    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()), 
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }",Some("1234".to_string()), 
            &json!({"name": "Project One", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    client
        .update_node(
            "Project",
            "__typename id name status issues { __typename props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
            &json!({"issues": {"UPDATE": {"match": {"dst": { "Feature": { "name": "Feature Zero"}}}, "update": {"props": {"since": "Forever"}}}}}),
        )
        .await
        .unwrap();

    let p_zero = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
        )
        .await
        .unwrap();

    assert!(p_zero.is_array());
    let p_zero_a = p_zero.as_array().unwrap();
    assert_eq!(p_zero_a.len(), 1);

    let p2 = &p_zero_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");

    assert_eq!(p2.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(!p2
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "Forever"));

    let p_one = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project One"})),
        )
        .await
        .unwrap();

    assert!(p_one.is_array());
    let p_one_a = p_one.as_array().unwrap();
    assert_eq!(p_one_a.len(), 1);

    let p3 = &p_one_a[0];
    assert!(p3.is_object());
    assert_eq!(p3.get("__typename").unwrap(), "Project");

    assert_eq!(p3.get("issues").unwrap().as_array().unwrap().len(), 2);
    assert!(p3
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i.get("props").unwrap().get("since").unwrap() == "Forever"));
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_mnmt_relationship_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_mnmt_relationship_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_mnmt_relationship(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can update a node to delete a relationship
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnmt_relationship(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("issues").unwrap().is_array());
    let issues0 = p0.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues0.len(), 2);

    assert!(issues0
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Feature"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Feature Zero"));

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
            &json!({"issues": {"DELETE": {"match": {"dst": { "Feature": { "name": "Feature Zero"}}}}}}),
        )
        .await
        .unwrap();

    assert!(pu.is_array());
    let pu_a = pu.as_array().unwrap();
    assert_eq!(pu_a[0].get("__typename").unwrap(), "Project");
    assert_eq!(pu_a[0].get("name").unwrap(), "Project Zero");

    let p1 = &pu_a[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project Zero");

    assert!(p1.get("issues").unwrap().is_array());
    let issuesu = p1.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issuesu.len(), 1);

    let issueu = &issuesu[0];
    assert_eq!(issueu.get("__typename").unwrap(), "ProjectIssuesRel");
    assert_eq!(issueu.get("dst").unwrap().get("__typename").unwrap(), "Bug");
    assert_eq!(issueu.get("dst").unwrap().get("name").unwrap(), "Bug Zero");

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 1);

    let p2 = &projects_a[0];
    assert!(p2.is_object());
    assert_eq!(p2.get("__typename").unwrap(), "Project");
    assert_eq!(p2.get("id").unwrap(), p0.get("id").unwrap());
    assert_eq!(p2.get("name").unwrap(), "Project Zero");

    let issues1 = p2.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues1.len(), 1);

    assert_eq!(issues1[0].get("__typename").unwrap(), "ProjectIssuesRel");

    let bug1 = issues1[0].get("dst").unwrap();
    assert_eq!(bug1.get("__typename").unwrap(), "Bug");
    assert_eq!(bug1.get("name").unwrap(), "Bug Zero");
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_node_by_mnmt_rel_property_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_node_by_mnmt_rel_property(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_node_by_mnmt_rel_property_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_node_by_mnmt_rel_property(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node based on matching a property on a rel.
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnmt_rel_property(mut client: Client) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "never" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } } ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    assert!(p0.get("issues").unwrap().is_array());
    let issues0 = p0.get("issues").unwrap().as_array().unwrap();
    assert_eq!(issues0.len(), 1);

    assert!(issues0
        .iter()
        .any(|i| i.get("__typename").unwrap() == "ProjectIssuesRel"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("__typename").unwrap() == "Bug"));
    assert!(issues0
        .iter()
        .any(|i| i.get("dst").unwrap().get("name").unwrap() == "Bug Zero"));

    client
        .delete_node(
            "Project",
            Some("1234".to_string()),
            Some(&json!({"issues": {"props": {"since": "never"}}})),
            Some(&json!({"issues": [{"match": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}

#[cfg(feature = "graphson2")]
#[tokio::test]
async fn delete_node_graphson2() {
    init();
    clear_db();

    let mut server = test_server_graphson2("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = graphson2_test_client();
    delete_node(client).await;

    assert!(server.shutdown().is_ok());
}

#[cfg(feature = "neo4j")]
#[tokio::test]
async fn delete_node_neo4j() {
    init();
    clear_db();

    let mut server = test_server_neo4j("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    let client = neo4j_test_client();
    delete_node(client).await;

    assert!(server.shutdown().is_ok());
}

/// Passes if warpgrapher can delete a node
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node(mut client: Client) {
    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234".to_string()),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "never" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } } ] }))
        .await
        .unwrap();

    let projects_pre = client
        .read_node("Project", "id", Some("1234".to_string()), None)
        .await
        .unwrap();
    assert!(projects_pre.is_array());
    assert_eq!(projects_pre.as_array().unwrap().len(), 1);

    let bugs_pre = client
        .read_node("Bug", "id", Some("1234".to_string()), None)
        .await
        .unwrap();
    assert!(bugs_pre.is_array());
    assert_eq!(bugs_pre.as_array().unwrap().len(), 1);

    client
        .delete_node(
            "Project",
            Some("1234".to_string()),
            Some(&json!({"name": "Project Zero"})),
            Some(&json!({})),
        )
        .await
        .unwrap();

    let projects_post = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234".to_string()),
            None,
        )
        .await
        .unwrap();

    assert!(projects_post.is_array());
    assert_eq!(projects_post.as_array().unwrap().len(), 0);

    let bugs_post = client
        .read_node("Bug", "id", Some("1234".to_string()), None)
        .await
        .unwrap();
    assert!(bugs_post.is_array());
    assert_eq!(bugs_post.as_array().unwrap().len(), 1);
}
