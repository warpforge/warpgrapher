///! SNMT Relationship: board (KanbanBoard, ScrumBoard)
mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if a node is created with an SNMT rel to a new node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_node_with_rel_to_new() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create new Project with rel to new KanbanBoard
    let results0 = client
        .create_node(
            "Project",
            "__typename
            id
            board {
                __typename
                id
                dst {
                    ... on KanbanBoard {
                        __typename
                        id
                        name       
                    }
                }
            }",
            &json!({
                "name": "SPARTAN-V",
                "board": {
                    "dst": {
                        "KanbanBoard": {
                            "NEW": {
                                "name": "SPARTAN-V Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    assert!(results0.is_object());
    let p0_board = results0.get("board").unwrap();
    assert!(p0_board.is_object());
    assert_eq!(p0_board.get("__typename").unwrap(), "ProjectBoardRel");
    let p0_board_dst = p0_board.get("dst").unwrap();
    assert!(p0_board_dst.is_object());
    assert_eq!(p0_board_dst.get("__typename").unwrap(), "KanbanBoard");
    assert_eq!(p0_board_dst.get("name").unwrap(), "SPARTAN-V Board");

    // read Kanbanboard
    let results1 = client
        .read_node(
            "KanbanBoard",
            "__typename
            id
            name",
            Some(&json!({
                "name": "SPARTAN-V Board"
            })),
        )
        .unwrap();

    assert!(results1.is_array());
    let b0 = &results1[0];
    assert!(b0.is_object());
    assert_eq!(b0.get("__typename").unwrap(), "KanbanBoard");
    assert_eq!(b0.get("name").unwrap(), "SPARTAN-V Board");

    // read Project
    let results2 = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            board { 
                __typename 
                dst { 
                    ... on KanbanBoard {
                        __typename 
                        id
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": "SPARTAN-V"
            })),
        )
        .unwrap();

    assert!(results2.is_array());
    assert_eq!(results2.as_array().unwrap().len(), 1);
    let p1 = &results2[0];
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "SPARTAN-V");
    let p1_board = p1.get("board").unwrap();
    assert_eq!(p1_board.get("__typename").unwrap(), "ProjectBoardRel");
    let p1_board_dst = p1_board.get("dst").unwrap();
    assert_eq!(p1_board_dst.get("__typename").unwrap(), "KanbanBoard");
    assert_eq!(p1_board_dst.get("name").unwrap(), "SPARTAN-V Board");
    assert_eq!(p1_board_dst.get("id").unwrap(), b0.get("id").unwrap());

    assert!(server.shutdown().is_ok());
}

/// Passes if a node is created with an SNMT rel to existing node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn create_node_with_rel_to_existing() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create new ScrumBoard
    let results0 = client
        .create_node(
            "ScrumBoard",
            "__typename
            id
            name
            ",
            &json!({
                "name": "SPARTAN-VI Board"
            }),
        )
        .unwrap();
    assert!(results0.is_object());
    assert_eq!(results0.get("__typename").unwrap(), "ScrumBoard");

    // create new Project with rel to existing ScrumBoard
    let results1 = client
        .create_node(
            "Project",
            "__typename
            id
            board {
                __typename
                id
                dst {
                    ... on ScrumBoard {
                        __typename
                        id
                        name       
                    }
                }
            }",
            &json!({
                "name": "SPARTAN-VI",
                "board": {
                    "dst": {
                        "ScrumBoard": {
                            "EXISTING": {
                                "name": "SPARTAN-VI Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    assert!(results1.is_object());
    assert_eq!(results1.get("__typename").unwrap(), "Project");

    // read project
    let results2 = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            board { 
                dst { 
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                } 
            }",
            Some(&json!({
                "name": "SPARTAN-VI"
            })),
        )
        .unwrap();

    assert!(results2.is_array());
    assert_eq!(results2.as_array().unwrap().len(), 1);
    let p1 = &results2[0];
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    let p1_board = p1.get("board").unwrap();
    let b1 = p1_board.get("dst").unwrap();
    assert_eq!(b1.get("__typename").unwrap(), "ScrumBoard");
    assert_eq!(b1.get("name").unwrap(), "SPARTAN-VI Board");
    assert!(server.shutdown().is_ok());
}

/// Passes if multiple nodes with multiple rels are read and
/// the relationships associate correctly
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_multiple_nodes_with_multiple_rels() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create multiple nodes with multiple rels
    let results0 = client
        .create_node(
            "Project",
            "__typename
            id
            ",
            &json!({
                "name": "SPARTAN-10",
                "board": {
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "SPARTAN-10 Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();
    let results1 = client
        .create_node(
            "Project",
            "__typename
            id
            ",
            &json!({
                "name": "SPARTAN-11",
                "board": {
                    "dst": {
                        "KanbanBoard": {
                            "NEW": {
                                "name": "SPARTAN-11 Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();
    let results2 = client
        .create_node(
            "Project",
            "__typename
            id
            ",
            &json!({
                "name": "SPARTAN-12",
                "board": {
                    "dst": {
                        "KanbanBoard": {
                            "NEW": {
                                "name": "SPARTAN-12 Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    // read nodes
    let results3 = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            board { 
                __typename 
                dst { 
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                } 
            }",
            None,
        )
        .unwrap();

    assert!(results3.is_array());
    let projects = results3.as_array().unwrap();

    let p0 = projects
        .iter()
        .find(|&x| x.get("id").unwrap() == results0.get("id").unwrap())
        .unwrap();
    assert!(p0.is_object());
    let p0_board = p0.get("board").unwrap();
    assert_eq!(p0_board.get("__typename").unwrap(), "ProjectBoardRel");
    let b0 = p0_board.get("dst").unwrap();
    assert_eq!(b0.get("__typename").unwrap(), "ScrumBoard");
    assert_eq!(b0.get("name").unwrap(), "SPARTAN-10 Board");

    let p1 = projects
        .iter()
        .find(|&x| x.get("id").unwrap() == results1.get("id").unwrap())
        .unwrap();
    assert!(p1.is_object());
    let p1_board = p1.get("board").unwrap();
    assert_eq!(p1_board.get("__typename").unwrap(), "ProjectBoardRel");
    let b1 = p1_board.get("dst").unwrap();
    assert_eq!(b1.get("__typename").unwrap(), "KanbanBoard");
    assert_eq!(b1.get("name").unwrap(), "SPARTAN-11 Board");

    let p2 = projects
        .iter()
        .find(|&x| x.get("id").unwrap() == results2.get("id").unwrap())
        .unwrap();
    assert!(p2.is_object());
    let p2_board = p2.get("board").unwrap();
    assert_eq!(p2_board.get("__typename").unwrap(), "ProjectBoardRel");
    let b2 = p2_board.get("dst").unwrap();
    assert_eq!(b2.get("__typename").unwrap(), "KanbanBoard");
    assert_eq!(b2.get("name").unwrap(), "SPARTAN-12 Board");
    assert!(server.shutdown().is_ok());
}

/// Passes if nodes matching props on a relationship are returned
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_node_with_matching_props_on_rel() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create nodes with rel with props
    let results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
                "board": {
                    "props": {
                        "public": false
                    },
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "ORION Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();
    assert!(results0.is_object());
    let results1 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "SPARTAN",
                "board": {
                    "props": {
                        "public": true
                    },
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "SPARTAN Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();
    assert!(results1.is_object());

    // read projects matching props on rels
    let results3 = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            board { 
                __typename 
                props {
                    public
                }
                dst { 
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                } 
            }",
            Some(&json!({
                "board": {
                    "props": {
                        "public": true
                    }
                }
            })),
        )
        .unwrap();
    assert!(results3.is_array());
    let projects0 = results3.as_array().unwrap();
    assert_eq!(projects0.len(), 1);
    let p0 = &projects0[0];
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "SPARTAN");
    let p0_board = p0.get("board").unwrap();
    assert_eq!(p0_board.get("__typename").unwrap(), "ProjectBoardRel");
    let b0 = p0_board.get("dst").unwrap();
    assert_eq!(b0.get("name").unwrap(), "SPARTAN Board");

    // read projects matching props on rels
    let results4 = client
        .read_node(
            "Project",
            "__typename 
            id 
            name 
            board { 
                __typename 
                dst { 
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                } 
            }",
            Some(&json!({
                "board": {
                    "props": {
                        "public": false
                    }
                }
            })),
        )
        .unwrap();
    assert!(results4.is_array());
    let projects1 = results4.as_array().unwrap();
    assert_eq!(projects1.len(), 1);
    let p1 = &projects1[0];
    assert!(p1.is_object());
    assert_eq!(p1.get("__typename").unwrap(), "Project");
    assert_eq!(p1.get("name").unwrap(), "ORION");
    let p1_board = p1.get("board").unwrap();
    assert_eq!(p1_board.get("__typename").unwrap(), "ProjectBoardRel");
    let b1 = p1_board.get("dst").unwrap();
    assert_eq!(b1.get("name").unwrap(), "ORION Board");
    assert!(server.shutdown().is_ok());
}

/// Passes if it returns nodes with relationship dst nodes
/// with matching props
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn read_node_with_matching_props_on_rel_dst_node() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create nodes with rel with props
    let _results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
                "board": {
                    "props": {
                        "public": false
                    },
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "ORION Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();
    let _results1 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "SPARTAN",
                "board": {
                    "props": {
                        "public": false
                    },
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "SPARTAN Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    // read nodes matching rel dst node props
    let results2 = client
        .read_node(
            "Project",
            "__typename
             id
             name
             board {
                __typename 
                dst { 
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                    ... on KanbanBoard {
                        __typename 
                        id
                        name
                    }
                } 
             }
            ",
            Some(&json!({
                "board": {
                    "dst": {
                        "ScrumBoard": {
                            "name": "SPARTAN Board"
                        }
                    }
                }
            })),
        )
        .unwrap();
    assert!(results2.is_array());
    let projects = results2.as_array().unwrap();
    assert_eq!(projects.len(), 1);
    let p0 = &projects[0];
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "SPARTAN");

    assert!(server.shutdown().is_ok());
}

/// Passes if a relationship to a new node is created
/// for an existing node
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_existing_node_with_rel_to_new_node() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create project node
    let _results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
            }),
        )
        .unwrap();

    // update project node to create a rel to a new node
    let _results1 = client
        .update_node(
            "Project",
            "__typename
            id
            name
            board {
                dst {
                    ... on ScrumBoard {
                        __typename 
                        id
                        name
                    }
                }
            }
            ",
            Some(&json!({
                "name": "ORION"
            })),
            &json!({
                "board": {
                    "ADD": {
                        "dst": {
                            "KanbanBoard": {
                                "NEW": {
                                    "name": "ORION Board"
                                }
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    // read nodes matching rel dst node props
    let results2 = client
        .read_node(
            "Project",
            "__typename
             id
             name
             board {
                 __typename
                 dst {
                    ... on ScrumBoard {
                        id
                        name
                    }
                }
             }
            ",
            None,
        )
        .unwrap();
    assert!(results2.is_array());
    let projects = results2.as_array().unwrap();
    assert_eq!(projects.len(), 1);
    let p0 = &projects[0];
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "ORION");
    let p0_board = p0.get("board").unwrap();
    assert_eq!(p0_board.get("__typename").unwrap(), "ProjectBoardRel");
    assert_eq!(
        p0_board.get("dst").unwrap().get("name").unwrap(),
        "ORION Board"
    );

    assert!(server.shutdown().is_ok());
}

///
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn update_existing_node_with_rel_to_existing_node() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create project node
    let _results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
            }),
        )
        .unwrap();

    // create board node
    let _results1 = client.create_node(
        "ScrumBoard",
        "id",
        &json!({
            "name": "ORION Board"
        }),
    );

    // update project node to create a rel to a new node
    let _results2 = client
        .update_node(
            "Project",
            "__typename
            id
            name
            ",
            Some(&json!({
                "name": "ORION"
            })),
            &json!({
                "board": {
                    "ADD": {
                        "dst": {
                            "ScrumBoard": {
                                "EXISTING": {
                                    "name": "ORION Board"
                                }
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    // read nodes matching rel dst node props
    let results2 = client
        .read_node(
            "Project",
            "__typename
             id
             name
             board {
                 __typename
                 dst {
                     ... on ScrumBoard {
                         id
                         name
                     }
                 }
             }
            ",
            None,
        )
        .unwrap();
    assert!(results2.is_array());
    let projects = results2.as_array().unwrap();
    assert_eq!(projects.len(), 1);
    let p0 = &projects[0];
    assert!(p0.is_object());
    assert_eq!(p0.get("__typename").unwrap(), "Project");
    assert_eq!(p0.get("name").unwrap(), "ORION");
    let p0_board = p0.get("board").unwrap();
    assert_eq!(p0_board.get("__typename").unwrap(), "ProjectBoardRel");
    assert_eq!(
        p0_board.get("dst").unwrap().get("name").unwrap(),
        "ORION Board"
    );

    assert!(server.shutdown().is_ok());
}

///
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_node_with_matching_props_on_rel_dst_node() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create project nodes
    let _results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
            }),
        )
        .unwrap();
    let _results1 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "SPARTAN-II",
            }),
        )
        .unwrap();

    // delete node with matching props
    let _results2 = client
        .delete_node("Project", Some(&json!({"name": "ORION"})), None)
        .unwrap();

    // read projects
    let results3 = client
        .read_node(
            "Project",
            "__typename
             id
             name
            ",
            None,
        )
        .unwrap();
    assert!(results3.is_array());
    assert_eq!(results3.as_array().unwrap().len(), 1);
    assert_eq!(results3[0].get("name").unwrap(), "SPARTAN-II");

    assert!(server.shutdown().is_ok());
}

///
#[allow(clippy::cognitive_complexity)]
#[test]
#[serial]
fn delete_node_with_force_detach_of_rels() {
    init();
    clear_db();
    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/minimal.yml");
    assert!(server.serve(false).is_ok());

    // create project nodes
    let _results0 = client
        .create_node(
            "Project",
            "id",
            &json!({
                "name": "ORION",
                "board": {
                    "dst": {
                        "ScrumBoard": {
                            "NEW": {
                                "name": "ORION Board"
                            }
                        }
                    }
                }
            }),
        )
        .unwrap();

    // delete node with matching props
    let _results1 = client
        .delete_node(
            "Project",
            Some(&json!({"name": "ORION"})),
            Some(&json!({"force": true})),
        )
        .unwrap();

    // read projects
    let results2 = client
        .read_node(
            "Project",
            "__typename
             id
             name
            ",
            None,
        )
        .unwrap();
    assert!(results2.is_array());
    assert_eq!(results2.as_array().unwrap().len(), 0);

    // verify dst node was not deleted
    let results3 = client
        .read_node(
            "ScrumBoard",
            "__typename
             id
             name
            ",
            None,
        )
        .unwrap();
    assert!(results3.is_array());
    assert_eq!(results3.as_array().unwrap().len(), 1);
    assert_eq!(results3[0].get("name").unwrap(), "ORION Board");

    assert!(server.shutdown().is_ok());
}
