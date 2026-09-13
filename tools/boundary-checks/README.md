# Boundary Checks

Architecture checks that enforce the active V1 domain ownership rules.

## Checks

- `check-product-boundaries.mjs`: active Rust services cannot depend on another
  service application crate, product libraries cannot depend on runtime or
  adapter crates, and Billing migrations remain owned by Billing.
- `scripts/check-nx-boundaries.mjs`: validates project metadata and package
  dependencies, then parses JavaScript and TypeScript imports through the
  TypeScript AST. Cross-project relative imports and forbidden domain/layer
  dependencies fail the gate.
  Rules for archived products remain available through Git history and are not
  part of the active V1 gate.
