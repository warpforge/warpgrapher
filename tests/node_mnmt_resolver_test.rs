mod setup;

use log::trace;
use serde_json::json;
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
use warpgrapher_macros::wg_test;

/// Passes if warpgrapher can create a node with a relationship to another new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_new_nodes<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }",Some("1234"),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }",Some("1234"),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher can create a node with a relationship to an existing node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn create_mnmt_existing_nodes<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let b0 = client
        .create_node(
            "Bug",
            "__typename id name",
            Some("1234"),
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
            Some("1234"),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234"),
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "EXISTING": { "name": {"EQ": "Bug Zero" }} } } }, { "dst": { "Feature": {"EXISTING": { "name": {"EQ": "Feature Zero" }}}}} ] }))
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher can query for a relationship by the properties of a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_by_rel_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let projects = client
        .read_node(
            "Project",             
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"issues": {"props": {"since": {"EQ": "today"}}}}))
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

/// Passes if warpgrapher can query for a relationship by the properties of a destination node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn read_mnmt_by_dst_props<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "Project Zero");

    let p1 = client
        .create_node(
            "Project",
            "__typename id name", Some("1234"),
            &json!({"name": "Project One", "issues": [ { "props": { "since": "today" }, "dst": { "Bug": { "NEW": { "name": "Bug One" } } } }, { "props": { "since": "yesterday" },  "dst": { "Feature": {"NEW": { "name": "Feature One" }}}} ] }))
        .await
        .unwrap();

    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "Project One");

    let projects = client
        .read_node(
            "Project", 
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"issues": {"dst": {"Bug": {"name": {"EQ": "Bug Zero"}}}}}))
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

/// Passes if warpgrapher can update a node to add a relationship to a new node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_new_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name description status priority estimate active",
            Some("1234"),
            &json!({"name": "Project Zero", "description": "Powered armor"}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher can update a node to add a relationship to an existing node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_existing_nodes<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let b0 = client
        .create_node(
            "Bug",
            "__typename id name",
            Some("1234"),
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
            "__typename id name description status priority estimate active", Some("1234"),
            &json!({"name": "Project Zero", "description": "Powered armor", "status": "GREEN", "priority": 1, "estimate": 3.3, "active": true}),
        )
        .await
        .unwrap();

    let pu = client
        .update_node(
            "Project",
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"issues": {"ADD": {"dst": { "Bug": { "EXISTING": {"name": {"EQ": "Bug Zero"}}}}}}}),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher can update a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_mnmt_relationship<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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
            "__typename id name status issues { __typename props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"issues": {"UPDATE": {"MATCH": {"dst": { "Feature": { "name": {"EQ": "Feature Zero"}}}}, "SET": {"props": {"since": "Forever"}}}}}),
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
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher only updates the correct matching relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn update_only_correct_mnmt_relationship<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"), 
            &json!({"name": "Project Zero", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }",Some("1234"), 
            &json!({"name": "Project One", "issues": [ { "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } }, { "dst": { "Feature": {"NEW": { "name": "Feature Zero" }}}} ] }))
        .await
        .unwrap();

    client
        .update_node(
            "Project",
            "__typename id name status issues { __typename props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project One"}})),
            &json!({"issues": {"UPDATE": {"MATCH": {"dst": { "Feature": { "name": {"EQ":"Feature Zero"}}}}, "SET": {"props": {"since": "Forever"}}}}}),
        )
        .await
        .unwrap();

    let p_zero = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
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
            "__typename id name issues { __typename id props { since } dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project One"}})),
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

/// Passes if warpgrapher can update a node to delete a relationship
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_mnmt_relationship<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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
            "__typename id name status issues { __typename dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            &json!({"issues": {"DELETE": {"MATCH": {"dst": { "Feature": { "name": {"EQ": "Feature Zero"}}}}}}}),
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
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
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

/// Passes if warpgrapher can delete a node based on matching a property on a rel.
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node_by_mnmt_rel_property<RequestCtx: RequestContext>(
    mut client: Client<RequestCtx>,
) {
    let p0 = client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234"),
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
            Some("1234"),
            Some(&json!({"issues": {"props": {"since": {"EQ": "never"}}}})),
            Some(&json!({"issues": [{"MATCH": {}}]})),
        )
        .await
        .unwrap();

    let projects = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects.is_array());
    let projects_a = projects.as_array().unwrap();
    assert_eq!(projects_a.len(), 0);
}

/// Passes if warpgrapher can delete a node
#[wg_test]
#[allow(clippy::cognitive_complexity, dead_code)]
async fn delete_node<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    client
        .create_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature {__typename id name } } }", Some("1234"),
            &json!({"name": "Project Zero", "issues": [ { "props": { "since": "never" }, "dst": { "Bug": { "NEW": { "name": "Bug Zero" } } } } ] }))
        .await
        .unwrap();

    let projects_pre = client
        .read_node("Project", "id", Some("1234"), None)
        .await
        .unwrap();
    assert!(projects_pre.is_array());
    assert_eq!(projects_pre.as_array().unwrap().len(), 1);

    let bugs_pre = client
        .read_node("Bug", "id", Some("1234"), None)
        .await
        .unwrap();
    assert!(bugs_pre.is_array());
    assert_eq!(bugs_pre.as_array().unwrap().len(), 1);

    client
        .delete_node(
            "Project",
            Some("1234"),
            Some(&json!({"name": {"EQ": "Project Zero"}})),
            Some(&json!({})),
        )
        .await
        .unwrap();

    let projects_post = client
        .read_node(
            "Project",
            "__typename id name issues { __typename id dst { ...on Bug { __typename id name } ...on Feature { __typename id name } } }", Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(projects_post.is_array());
    assert_eq!(projects_post.as_array().unwrap().len(), 0);

    let bugs_post = client
        .read_node("Bug", "id", Some("1234"), None)
        .await
        .unwrap();
    assert!(bugs_post.is_array());
    assert_eq!(bugs_post.as_array().unwrap().len(), 1);
}
