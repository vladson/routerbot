# Routerbot

Routerbot is a local-first, configurable ChatOps bot for routers and home servers.

The first supported chat interface is Telegram. The first primary runtime target is a
Keenetic router via cross-compilation. Routerbot is designed as a small, typed,
permission-controlled command router rather than a generic remote shell.

## Status

This repository is at Stage 4: typed configuration boundary.

The core domain, application orchestration layer, and configuration model are being
added in small reviewable stages. Runtime integrations are still intentionally
behind adapter crates and are not wired into the binary yet.

## Workspace

- `crates/core`: domain types and domain errors.
- `crates/app`: orchestration, authorization, automation, and capability routing.
- `crates/config`: configuration loading and validation.
- `crates/chat/telegram`: Telegram command handling.
- `crates/adapters/transmission`: Transmission RPC adapter.
- `crates/adapters/keenetic`: Keenetic device control adapter.
- `crates/adapters/kubernetes`: Kubernetes workload control adapter.
- `crates/adapters/dlna-k8s`: DLNA media indexing through Kubernetes rollout restart.
- `crates/adapters/command`: statically configured command adapter.
- `crates/bin`: final `routerbot` binary.

## Local Checks

Run the full local check script:

```bash
./scripts/check.sh
```

Or run the commands directly:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo check --workspace --all-features
```

## Documentation Language

All repository deliverables must be written in English, including source code,
comments, documentation, examples, commit messages, pull request text, and issue text.

## License

Licensed under MIT OR Apache-2.0.
