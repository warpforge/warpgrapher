# Static Endpoints

Warpgrapher includes built-in static endpoints that provide useful information or functionality. Built-in static endpoints names are preceded by `_`.

## Version

If the `Engine` is built with an explicit version:

```rust
let mut server: Engine<()> = Engine::new(config, db)
    .with_version("0.1.0".to_string())
    .build();
```

the version value can be accessed via the `_version` endpoint:

```
query {
    _version
}
```

```json
{
  "data": {
    "_version": "0.1.0"
  }
}
```

If the server is not configured with an explicit version, the `_version` endpoint will return `null`:

```json
{
  "data": {
    "_version": null
  }
}
```