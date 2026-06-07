Release new version:

1. Read current version from Cargo.toml workspace.package.version
2. Decide new version number (patch/minor/major)
3. Update version in root Cargo.toml workspace.package.version
4. Run cargo test --workspace to verify all tests pass
5. Run cargo clippy --workspace to verify no lint errors
6. Run cargo fmt --check to verify formatting
7. Commit with message: chore: release vX.Y.Z
8. Tag with v followed by the version number and push
