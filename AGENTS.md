# AGENTS.md

## Purpose

This repository is an idiomatic Rust codebase. You may discuss tasks with maintainers in Russian or English,
but all deliverables in the repository must be in English.

## Language rules (strict)

- Chat/discussion language: Russian is allowed.
- Repository outputs: English only:
    - Source code identifiers (names of modules, functions, variables, types)
    - Comments and doc comments (///, //!), error messages, log messages
    - Commit messages
    - PR title and PR description
    - GitHub Issues and review notes created in this repo

If you must include Russian text (rare), put it only in quoted user-provided content, not in new writing.

## Rust expectations

- Prefer stable Rust and 2024 edition.
- No unsafe code unless absolutely necessary; if used, justify with a comment and a minimal unsafe scope.
- Prefer explicit, readable code over cleverness.
- Keep APIs small and coherent; document public items with rustdoc.
- Add tests for new behavior (unit tests + at least one integration test when it matters).

## Commands you MUST run before proposing changes

- Format:
    - `cargo fmt --all`
- Lint (no warnings):
    - `cargo clippy --all-targets --all-features -- -D warnings`
- Tests:
    - `cargo test --all --all-features`

## PR expectations

- PR description must include:
    - What changed and why
    - How it was tested (commands + results)
    - Any tradeoffs / follow-ups

## Review guidelines

- Do not introduce breaking changes without documenting them.
- Avoid adding dependencies unless clearly justified.
- Keep error handling consistent (use `thiserror` for library errors and `anyhow` for binaries when appropriate).
- No secrets, tokens, or PII in code or tests.
