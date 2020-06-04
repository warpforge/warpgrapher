# Warpgrapher Config

In the [Quickstart]() we demonstrated how to initialize a `Config` from a configurationf file read from file at run time. A Warpgrapher `Config` can also be generated programatically. The resulting configs from all the examples below are equivalent. 

### Text Config

```yaml
version: 1
model:

  # User
  - name: User
    properties:
      - name: username
        type: String
      - name: email
        type: String

  # Team
  - name: Team
    properties:
      - name: teamname
        type: String
    relationships:
      - name: members
        nodes: [User]
        list: true
```

### Programmatic Config

```rust
use warpgrapher::engine::config::{Config, Property, Relationship, Type};

let config = Config::new(
    1,
    vec![
        // User
        Type::new(
            "User".to_string(),
            vec![
                Property::new(
                    "username".to_string(),
                    "String".to_string(),
                    false,
                    false,
                    None,
                    None,
                ),
                Property::new(
                    "email".to_string(),
                    "String".to_string(),
                    false,
                    false,
                    None,
                    None,
                ),
            ],
            Vec::new(),
            EndpointsFilter::all(),
        ),
        // Team
        Type::new(
            "Team".to_string(),
            vec![Property::new(
                "teamname".to_string(),
                "String".to_string(),
                false,
                false,
                None,
                None,
            )],
            vec![Relationship::new(
                "members".to_string(),
                true,
                vec!["User".to_string()],
                Vec::new(),
                EndpointsFilter::default(),
                None,
            )],
            EndpointsFilter::all(),
        ),
    ],
    vec![],
);
```