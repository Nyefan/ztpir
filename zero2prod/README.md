## Dev Environment Setup

### Prerequisites

```shell
# generally required development tooling
rustup update stable
rustup component add rustfmt llvm-tools-preview clippy
cargo install cargo-llvm-cov cargo-watch cargo-audit

#required for some development tasks
rustup toolchain install nightly --allow-downgrade
cargo install cargo-expand
```