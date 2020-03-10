mod setup;

use serde_json::json;
use serial_test::serial;
use setup::server::test_server;
use setup::{clear_db, init, test_client};

/// Passes if the create mutation and the read query both succeed.
#[test]
#[serial]
fn scalar_lists_test() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            &json!({
                "string_list": ["string0", "string1", "string2", "string3"],
                "bool_list": [true, false, true, false],
                "int_list": [0, 1, 2, 3],
                "float_list": [0.0, 1.1, 2.2, 3.3]
            }),
        )
        .unwrap();

    let strings = result.get("string_list").unwrap();
    assert!(strings.is_array());
    assert_eq!(strings.get(0).unwrap().as_str().unwrap(), "string0");
    assert_eq!(strings.get(1).unwrap().as_str().unwrap(), "string1");
    assert_eq!(strings.get(2).unwrap().as_str().unwrap(), "string2");
    assert_eq!(strings.get(3).unwrap().as_str().unwrap(), "string3");

    let bools = result.get("bool_list").unwrap();
    assert!(bools.is_array());
    assert_eq!(bools.get(0).unwrap().as_bool().unwrap(), true);
    assert_eq!(bools.get(1).unwrap().as_bool().unwrap(), false);
    assert_eq!(bools.get(2).unwrap().as_bool().unwrap(), true);
    assert_eq!(bools.get(3).unwrap().as_bool().unwrap(), false);

    let ints = result.get("int_list").unwrap();
    assert!(ints.is_array());
    assert_eq!(ints.get(0).unwrap().as_i64().unwrap(), 0);
    assert_eq!(ints.get(1).unwrap().as_i64().unwrap(), 1);
    assert_eq!(ints.get(2).unwrap().as_i64().unwrap(), 2);
    assert_eq!(ints.get(3).unwrap().as_i64().unwrap(), 3);

    let floats = result.get("float_list").unwrap();
    assert!(floats.is_array());
    assert_eq!(floats.get(0).unwrap().as_f64().unwrap(), 0.0);
    assert_eq!(floats.get(1).unwrap().as_f64().unwrap(), 1.1);
    assert_eq!(floats.get(2).unwrap().as_f64().unwrap(), 2.2);
    assert_eq!(floats.get(3).unwrap().as_f64().unwrap(), 3.3);

    assert!(server.shutdown().is_ok());
}

/// Passes if the create mutation and the read query both succeed.
#[test]
#[serial]
fn scalar_lists_no_array_test() {
    init();
    clear_db();

    let mut client = test_client();
    let mut server = test_server("./tests/fixtures/scalar_list.yml");
    assert!(server.serve(false).is_ok());

    let result = client
        .create_node(
            "TestType",
            "string_list
             bool_list
             int_list
             float_list
            ",
            &json!({
                "string_list": "string0",
                "bool_list": false,
                "int_list": 0,
                "float_list": 0.0,
            }),
        )
        .unwrap();

    assert_eq!(result.get("string_list").unwrap().as_str().unwrap(), "string0");

    assert_eq!(result.get("bool_list").unwrap().as_bool().unwrap(), false);

    assert_eq!(result.get("int_list").unwrap().as_i64().unwrap(), 0);

    assert_eq!(result.get("float_list").unwrap().as_f64().unwrap(), 0.0);

    assert!(server.shutdown().is_ok());
}
