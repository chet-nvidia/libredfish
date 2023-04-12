# libredfish
Native Rust redfish library for https://redfish.dmtf.org/

## Publish a new version

- (one time only) Get an Artifactory token from https://urm.nvidia.com/ui/ far right "Welcome, <username>" / Edit Profile / Generate an Identity Token
- Bump the version number in Cargo.toml
- `cargo publish --token "Bearer [token]"`. You can add `--dry-run` to sanity check before publishing.

## Development

The [carbide repo](https://gitlab-master.nvidia.com/nvmetal/carbide)'s `forge-admin-cli redfish --help` is a command line client for calling libredfish. This is ideal for trying out new libredfish commands.

To get forge-admin-cli to build against your local checkout of libredfish, so you don't have to publish, make these two changes:

1. Edit `carbide/.cargo/config`

In `[registries]` add:
```
cratesio = { index = "https://github.com/rust-lang/crates.io-index" }
```

2. Edit `carbide/admin/Cargo.toml` change `libredfish` dependency to your libredfish checkout:

```
libredfish = { path = "/home/graham/src/libredfish" }
```

The forge-admin-cli redfish subcommands are in `carbide/admin/src/redfish.rs`.
