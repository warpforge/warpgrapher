mod setup;

use serde_json::json;
#[cfg(feature = "neo4j")]
use warpgrapher::client::Client;
use warpgrapher::engine::context::RequestContext;
#[cfg(feature = "neo4j")]
use warpgrapher_macros::wg_test;

/// Passes if the create mutation succeeds and the rel query returns an empty list.
#[wg_test]
#[allow(dead_code)]
async fn create_node_read_rel<RequestCtx: RequestContext>(mut client: Client<RequestCtx>) {
    client
        .create_node(
            "Portfolio",
            "__typename id", Some("1234"),
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
        .read_rel(
            "Project",
            "activity",
            "__typename id",
            Some("1234"),
            None,
        )
        .await
        .unwrap();

    assert!(activities.is_array());
    let activities_a = activities.as_array().unwrap();
    assert_eq!(activities_a.len(), 0);
}
