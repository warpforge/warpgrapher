# Warpgrapher
[![Build Status](https://github.com/warpforge/warpgrapher/workflows/Test/badge.svg)](https://github.com/warpforge/warpgrapher/actions?query=workflow%3A%22Test%22+branch%3Amaster)
[![GitHub Release](https://img.shields.io/github/release/warpforge/warpgrapher.svg?style=flat)](https://github.com/warpforge/warpgrapher/releases/tag/v0.9.0)
[![Crates.io](https://img.shields.io/crates/v/warpgrapher.svg)](https://crates.io/crates/warpgrapher)

#### Framework for creating data model-driven GraphQL API services

As a developer you can focus on defining your application's data model, and Warpgrapher takes care of generating a graph-based API for interacting with that model. In addition to generating CRUD APIs for interacting with the model, Warpgrapher provides a set of advanced features to customize and extend your service. 

Supported Databases:

* CosmosDB
* Gremlin-based Databases (e.g. Apache Tinkerpop and AWS Neptune)
* Neo4j

## Quickstart

To get started right away, visit the [Quickstart](https://warpforge.github.io/warpgrapher/warpgrapher/quickstart.html) section of the Book, or look through our sample integration projects:

* [warpgrapher-rocket](https://github.com/warpforge/warpgrapher-rocket)
* [warpgrapher-actixweb](https://github.com/warpforge/warpgrapher-actixweb)
* [warpgrapher-lambda](https://github.com/warpforge/warpgrapher-lambda)

## Documentation

See the [Warpgrapher Book](https://warpforge.github.io/warpgrapher/) for in-depth usage documentation. 

## Contributing

See the [Contribution Guide](https://github.com/warpforge/warpgrapher/blob/master/CONTRIBUTING.md). 

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Under Active Development

The project is currently in active development. Prior to reaching 1.0.0:

1. Minor versions represent breaking changes.
2. Patch versions represent fixes and features.
3. There are no deprecation warnings between releases.
