# Warpgrapher Config

In the [Quickstart]() we demonstrated how to initialize a `Config` from a configurationf file read from file at run time. A Warpgrapher `Config` can also be generated programatically. The resulting configs from all the examples below are equivalent. 

### Text Config

```yaml
version: 1
model:

  # User
  - name: User
    props:
      - name: username
        type: String
      - name: email
        type: String

  # Team
  - name: Team
    props:
      - name: teamname
        type: String
    rels:
      - name: members
        nodes: [User]
        list: true
```

### Programmatic Config

```rust
use warpgrapher::engine::config::{Config, Prop, Relationship, Type};

let config = Config {
    version: 1,
    model: vec![

        // User
        Type {
            name: "User".to_string(),
            props: vec![
                Prop {
                    name: "username".to_string(),
                    type_name: "String",
                    required: false,
                    list: false,
                    resolver: None,
                    validator: None
                },
                Prop {
                    name: "email".to_string(),
                    type_name: "String",
                    required: false,
                    list: false,
                    resolver: None,
                    validator: None
                }
            ],
            rels: vec![]
        },

        // Team
        Type {
            name: "Team".to_string(),
            props: vec![
                Prop {
                    name: "teamname".to_string(),
                    type_name: "String",
                    required: false,
                    list: false,
                    resolver: None,
                    validator: None
                },
            ],
            rels: vec![
                Relationship {
                    name: "members".to_string(),
                    list: true,
                    nodes: vec!["User".to_string()],
                    props: vec![],
                    endpoints: EndpointsFilter::default()
                }
            ]
        },
    ],

    endpoints: vec![]
}
```