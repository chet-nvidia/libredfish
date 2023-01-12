# libredfish
Native Rust redfish library for https://redfish.dmtf.org/

Publish a new version:
- (one time only) Get an Artifactory token from https://urm.nvidia.com/ui/ far right "Welcome, <username>" / Edit Profile / Generate an Identity Token
- Bump the version number in Cargo.toml
- `cargo publish --token "Bearer [token]"`. You can add `--dry-run` to sanity check before publishing.

