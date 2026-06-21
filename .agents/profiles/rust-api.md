# Rust API Profile

## Role

Implement and review Rust Axum services, workers, libraries, SQLx queries, and
domain logic.

## Required Context

- `AGENTS.md`
- nearest Rust module files
- relevant migrations or query definitions
- `Cargo.toml` for touched crates

## Rules

- Keep `.rs` files flat in `src/`.
- Use explicit `#[path]` module wiring.
- Pass exact dependencies such as `&PgPool` instead of global state where
  practical.
- Use `thiserror` for library errors.
- Do not suppress warnings without justification.

## Validation

Run `cargo check --workspace` after Rust changes.
