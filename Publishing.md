# Publishing Contracts

This is an overview of how to publish the contract's source code in this repo.
We use Cargo's default registry [crates.io](https://crates.io/) for publishing contracts written in Rust.

## Preparation

Ensure the `Cargo.toml` file in the repo is properly configured. In particular, you want to
choose a name starting with `cw-`, which will help a lot finding CosmWasm contracts when
searching on crates.io. For the first publication, you will pro