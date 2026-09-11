# CircleCI CI/CD

The CircleCI pipeline in `.circleci/config.yml` validates every branch and pull
request. On `main` and version tags it also builds and tests native packages for
Linux, macOS, and Windows. Version tags publish all release assets together.

## CI behavior

The `build_and_test` job uses the pinned `cimg/rust:1.97.1` image and runs the
same gates documented for local development:

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli
git diff --exit-code
git diff --check
```

Configuration, data, cache, and temporary directories are isolated inside each
job. Cargo downloads and Linux build output are cached by architecture, Rust
version, and `Cargo.lock` checksum.

The platform jobs produce and test these installable archives:

- `lbc-VERSION-x86_64-unknown-linux-gnu.tar.gz`, with `lbc` and `install.sh`.
- `lbc-VERSION-universal-apple-darwin.tar.gz`, with an Intel and Apple Silicon
  universal `lbc` binary and `install.sh`.
- `lbc-VERSION-x86_64-pc-windows-msvc.zip`, with `lbc.exe` and `install.ps1`.

Every archive has a sibling `.sha256` file. The macOS and Windows jobs run the
full test suite, exercise the packaged release binary, install it into a
temporary prefix, verify the installed version, and exercise uninstall. These
jobs run for pushes to `main` and release tags; Linux CI continues to run for
every branch and pull request.

The macOS universal binary receives an ad-hoc signature so its combined binary
is internally consistent, but release artifacts are not Developer ID-signed,
notarized, or Authenticode-signed. Those trust signatures require separately
managed Apple and Microsoft signing credentials.

## CD setup

1. Add this GitHub repository as a CircleCI project. CircleCI automatically
   reads `.circleci/config.yml`.
2. In **Project Settings → Environment Variables**, add `GITHUB_TOKEN`. Use a
   fine-grained GitHub token limited to this repository with **Contents: Read
   and write** permission. CI for branches and pull requests does not need this
   secret.
3. Update the package version in `Cargo.toml` and `Cargo.lock`, merge the tested
   change, then push a matching annotated tag:

   ```bash
   git tag -a v0.4.0 -m "libraryCube v0.4.0"
   git push origin v0.4.0
   ```

Tags must match `vMAJOR.MINOR.PATCH`, with optional SemVer prerelease and build
suffixes, and must equal `v` plus the package version in `Cargo.toml`. After all
CI gates pass on all three operating systems, `publish_github_release` creates a
GitHub Release with generated notes and uploads every archive and checksum.
Re-running the publish job replaces assets of the same name, so recovery from a
failed upload is safe.

The pipeline deliberately does not publish to crates.io. Add a separate,
approval-gated job and a scoped `CARGO_REGISTRY_TOKEN` if crate publication is
required later.
