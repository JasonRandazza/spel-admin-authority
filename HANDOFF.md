# SPEL Admin Authority Handoff

## Project

- Location: `/home/jrazz/logos-bootcamp/spel-admin-authority`
- Planned GitHub repo: `https://github.com/JasonRandazza/spel-admin-authority`
- License: MIT OR Apache-2.0

## Context

- RFP: `/home/jrazz/logos-bootcamp/RFP-001-admin-authority-lib.md`
- SPEL local source: `/home/jrazz/logos-bootcamp/spel`
- LEZ/NSSA local source: `/home/jrazz/logos-bootcamp/logos-execution-zone`
- Logos Dev Boost MCP was queried for SPEL docs; it returned broad Logos docs, while the local `spel/` workspace provided the authoritative macro/IDL behavior.

## Implementation Notes

- The reusable crate is `admin-authority`.
- The sample SPEL program is `admin-authority-sample`.
- The local SPEL framework is patched to recognize `#[account(admin)]`.
- Generated admin validation assumes the instruction includes an account named `admin_authority` containing Borsh-encoded `AdminAuthority`, and an account annotated `#[account(admin)]` for the active admin signer/PDA.
- Workspace dependency patching forces SPEL's `nssa_core` dependency to `/home/jrazz/logos-bootcamp/logos-execution-zone/nssa/core`, avoiding duplicate `AccountId`/`AccountWithMetadata` types.

## Local SPEL Patch Files

The new project depends on these local SPEL changes:

- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/types.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/idl.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/idl_gen.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/tests/variable_accounts.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-macros/src/lib.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-client-gen/src/tests.rs`

These are not inside the `spel-admin-authority` git repo, so preserve or upstream them before expecting this project to build against a clean SPEL checkout.

## Required Tests

- `cargo check --workspace`: passing.
- `cargo test --workspace`: passing, 14 tests.
- `RISC0_DEV_MODE=1 cargo test -p integration-tests`: passing, 3 tests.
- `cargo fmt --all`: applied successfully; stable rustfmt emits warnings from the parent `rustfmt.toml`.
- `nix flake check`: passing.

## Next Steps

- Keep `HANDOFF.md` updated after each substantial implementation or verification pass.
- If SPEL upstream accepts the macro patch, replace local path dependency assumptions with the upstream version.
- Expand integration tests to deploy the sample guest ELF into `V03State` once the project adds packaged RISC Zero guest binaries.
