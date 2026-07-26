# C-Based Toolchain Hardening

This repository tracks C-family and native dependency hardening against the OWASP C-Based Toolchain Hardening Cheat Sheet. The source of truth is `docs/security/c-toolchain-hardening-controls.json`.

The current native surface is intentionally small:

- `vendor/xmlsec/bindings.h` is the C header input used by bindgen.
- `vendor/xmlsec/bindings.rs` is the Cargo build script that invokes `pkg-config`, `xmlsec1-config`, and bindgen for `xmlsec1`.

Native builds that must enforce C/C++ hardening should source the wrapper before Cargo:

```bash
source scripts/c-toolchain-hardened-env.sh release
cargo build --workspace --release
```

The wrapper exports `NVBES_C_TOOLCHAIN_HARDENING=required`, `CFLAGS`, `CXXFLAGS`, and `LDFLAGS` with the repository baseline: aggressive warnings, format-security errors, `_FORTIFY_SOURCE=3`, stack protector, PIE-compatible objects, retained frame pointers, and platform linker hardening.

When the hardening mode is required, the vendored `xmlsec` build script fails if the native flags are missing. The CI gate also detects new C-family files or native Cargo build scripts and requires them to be registered with owner and evidence.

Run:

```bash
pnpm check:c-toolchain-hardening
```
