# SPEL Admin Authority Handoff

## Project

- Location: `/home/jrazz/logos-bootcamp/spel-admin-authority`
- GitHub repo: `https://github.com/JasonRandazza/spel-admin-authority`
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

## Dependency Update Notes

- After updating `logos-execution-zone` (commit `f37454ed` "Refactor signatures"), `AccountId::from((program_id, &pda_seed))` was removed in favour of `AccountId::for_public_pda(program_id, &pda_seed)`. The SPEL patch file `spel-framework-core/src/pda.rs:87` was updated accordingly.

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
- `cargo test --workspace`: passing, 16 tests (after V03State work; see in-progress note below).
- `RISC0_DEV_MODE=1 cargo test -p integration-tests`: passing, 5 tests (3 original + 2 V03State).
- `cargo fmt --all`: applied successfully; stable rustfmt emits warnings from the parent `rustfmt.toml`.
- `nix flake check`: passing (fmt check covers all four crates including `admin-authority-sample-methods`).

## Integration Tests (V03State)

`integration-tests/tests/v03state.rs` deploys the compiled guest ELF into `V03State` and exercises real transactions end-to-end:

- `v03state_initialize_sets_admin_and_config` — deploys sample program, calls `Initialize { value: 42 }`, asserts `AdminAuthority.admin == Some(AdminKey::Signer(admin_id))` and `Config.value == 42`.
- `v03state_update_config_succeeds_for_admin` — initialises then calls `UpdateConfig { value: 99 }` with nonce 1, asserts `Config.value == 99`.

Requires `RISC0_DEV_MODE=1` (skips ZK proof generation while still executing the guest ELF).

## GitHub Actions CI

`.github/workflows/ci.yml` was added (hard RFP requirement). It runs on every push and PR to `main`:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (with `RISC0_DEV_MODE=1`)

## RFP Competitive Analysis (conducted 2026-05-04)

Three proposals are in play for RFP-001:

- **ygd58 (#26)** — Has working code (12 tests, localnet deploy) but misunderstood the process; built a standalone binary, not a library. No SPEL integration, no CI, no docs, no milestone proposal filed yet. Core team (`fryorcraken`) corrected them 2026-04-28. Stalled.
- **mmlado (#46)** — Strongest technical competitor. 19 years experience, two prior Logos deliveries. Proposes `#[admin_authority]` / `#[require_admin]` proc macros upstream to `logos-co/spel`. $3,500 budget. Risk: depends on upstream maintainer review.
- **JasonRandazza (#50)** — Our proposal. $2,000, 4 weeks, SPEL integration via `#[account(admin)]` annotation, already-running code with V03State integration tests. Cheapest price, no upstream dependency risk.

## In-Progress Work (incomplete — resume here next session)

The following changes were started but NOT yet verified to compile and pass tests:

### 1. `transfer_admin` accepts `AdminKey` instead of `AccountId`

**Why:** RFP soft requirement — admin authority can only be set to a valid on-curve signer OR deployed PDA, not an arbitrary account. The old signature hardcoded `AdminKey::Signer`, preventing PDA-admin transfers and failing to expose the Signer/Pda distinction to callers.

**What changed:**
- `admin-authority-sample/src/lib.rs`: `transfer_admin` parameter changed from `new_admin: AccountId` → `new_admin: AdminKey`
- `admin-authority/Cargo.toml`: added `serde.workspace = true` (SPEL instruction macro requires `Serialize + Deserialize` on all non-account parameters)
- `admin-authority/src/lib.rs`: added `use serde::{Deserialize, Serialize}` and `Serialize, Deserialize` to `AdminKey` derive

**Status: NOT yet verified. Run `RISC0_DEV_MODE=1 cargo check --workspace` to confirm it compiles.**

If it compiles: update `integration-tests/tests/v03state.rs` to add two more tests:
- `v03state_transfer_admin_updates_authority` — init, then transfer to a new key, verify `AdminAuthority.admin` changed
- `v03state_revoke_admin_blocks_privileged_calls` — init, revoke, verify `update_config` fails

The `Instruction::TransferAdmin { new_admin: AdminKey }` variant will now require passing `AdminKey::Signer(account_id)` or `AdminKey::Pda(account_id)` in the transaction instruction data. Update test construction accordingly.

### 2. README overhaul (NOT YET STARTED)

**Why:** RFP hard requirement — README must document how to add the library as a dependency, step-by-step integration into a SPEL program, at least one end-to-end example, and transaction overhead.

**What to write:**
- Dependency section: `Cargo.toml` snippet adding `admin-authority` from git
- Step-by-step integration: 4 steps (add dep, define admin_authority PDA account, annotate instructions with `#[account(admin)]`, call transfer/revoke helpers)
- End-to-end example: the full `update_config` handler from `admin-authority-sample`
- Transaction overhead table: `admin_authority` account = 35 bytes on-chain (Borsh); per-transaction overhead = +32 bytes per accounts list entry (the admin_authority PDA AccountId) + optionally +96 bytes if admin signer is not already signing for another reason (32 bytes AccountId + 64 bytes signature)
- Development commands section (already partially exists)

### 3. V03State tests for transfer and revoke (NOT YET STARTED)

After the `transfer_admin` fix compiles, add to `integration-tests/tests/v03state.rs`:

```rust
#[test]
fn v03state_transfer_admin_updates_authority() {
    // setup: init with admin_key ([1u8; 32])
    // action: transfer to new_admin_key ([2u8; 32]) using Instruction::TransferAdmin { new_admin: AdminKey::Signer(new_admin_id) }
    // assert: AdminAuthority.admin == Some(AdminKey::Signer(new_admin_id))
}

#[test]
fn v03state_revoke_admin_blocks_update_config() {
    // setup: init, then revoke_admin
    // assert: AdminAuthority.revoked == true, admin == None
    // optionally: try update_config → expect failure
}
```

Nonce tracking: after init (nonce 0→1), after transfer/revoke (nonce 1→2). Use `Nonce(1)` for the second transaction.

## Next Steps (ordered by priority)

1. **Resume in-progress work above** — verify `transfer_admin` change compiles, add transfer/revoke integration tests.
2. **Write the README overhaul** as described in in-progress section 2.
3. **Run full test suite** — `RISC0_DEV_MODE=1 cargo test --workspace` must still show all tests passing.
4. **Run `nix flake check`** — must stay green.
5. **Commit and push** to `https://github.com/JasonRandazza/spel-admin-authority`.
6. **Update proposal comment on issue #50** — explicitly call out CI, transaction overhead docs, and that V03State integration tests are already running (differentiates from competitors who only have unit tests or no code at all).
7. If SPEL upstream accepts the macro patch, replace local path dependency assumptions with the upstream version.
