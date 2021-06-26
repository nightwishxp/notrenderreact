# Publishing Contracts

This is an overview of how to publish the contract's source code in this repo.
We use Cargo's default registry [crates.io](https://crates.io/) for publishing contracts written in Rust.

## Preparation

Ensure the `Cargo.toml` file in the repo is properly configured. In particular, you want to
choose a name starting with `cw-`, which will help a lot finding CosmWasm contracts when
searching on crates.io. For the first publication, you will probably want version `0.1.0`.
If you have tested this on a public net already and/or had an audit on the code,
you can start with `1.0.0`, but that should imply some level of stability and confidence.
You will want entries like the following in `Cargo.toml`:

```toml
name = "cw-escrow"
version = "0.1.0"
description = "Simple CosmWasm contract for an escrow with arbiter and timeout"
repository = "https://github.com/confio/cosmwasm-examples"
```

You will also want to add a valid [SPDX license statement](https://spdx.org/licenses/),
so others know the rules for using this crate. You can use any license you wish,
even a commercial license, but we recommend choosing one of the following, unless you have
specific requirements.

* Permissive: [`Apache-2.0`](https://spdx.org/licenses/Apache-2.0.html#licenseText) or [`MIT`](https://spdx.org/licenses/MIT.html#licenseText)
* Copyleft: [`GPL-3.0-or-later`](https://spdx.org/licenses/GPL-3.0-or-later.html#licenseText) or [`AGPL-3.0-or-later`](https://spdx.org/licenses/AGPL-3.0-or-later.html#licenseText)
* Commercial license: `Commercial` (not sure if this works, I cannot find examples)

It is also helpful to download the LICENSE text (linked to above) and store this
in a LICENSE file in your repo. Now, you have properly configured your crate for use
in a larger ecosystem.

### Updating schema

To allow easy use of the contract, we can publish the schema (`schema/*.json`) together
with the source code.

```sh
cargo schema
```

Ensure you c