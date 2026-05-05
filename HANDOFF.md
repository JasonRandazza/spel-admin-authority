# SPEL Admin Authority — Handoff

> **For the next agent:** read this entire document before touching code. The "Current State" and "Active Work" sections below describe exactly what is in progress. The "Competitive Context" section explains *why* the work is shaped the way it is — this is an RFP submission and decisions are deliberately framed against two competing proposals.

---

## Project

- **Location:** `/home/jrazz/logos-bootcamp/spel-admin-authority`
- **GitHub:** https://github.com/JasonRandazza/spel-admin-authority
- **License:** MIT OR Apache-2.0
- **RFP:** logos-co/rfp issue #50 (our proposal), specification at `/home/jrazz/logos-bootcamp/RFP-001-admin-authority-lib.md`

## Author

Jason Randazza — Logos Bootcamp participant. New to crypto/blockchain development; learning by building. Other relevant Logos Bootcamp work on his GitHub:
- [p2p-polling](https://github.com/JasonRandazza/p2p-polling) — full-stack Logos Basecamp module using Logos Delivery messaging (C++ backend + Qt6/QML frontend).
- [hello-world-ui](https://github.com/JasonRandazza/hello-world-ui) — Logos `ui_qml` module reference.

The lack of long industry experience is a real disadvantage vs. competitor #46 (mmlado, 19 years), but is being offset by **shipping working code before the proposal review concludes**.

---

## Competitive Context

Three proposals are open against RFP-001 (admin authority library). Anything we ship has to be measured against these:

### Competitor 1 — ygd58 (logos-co/rfp issue #26)

- **Repo:** https://github.com/ygd58/admin-authority-poc (master branch, 3 commits)
- **What they have:** Working code with 12 unit tests, deployed manually on Logos devnet localnet.
- **Critical gaps:** It is a *standalone LEE guest binary*, not a library. No SPEL framework integration, no `#[account(admin)]` annotation, no CI, no README integration guide, no sample program that *imports* an authority library (their program *is* the program), raw byte-offset instruction dispatch with no type safety (`[u8; 41]` config layout, `[u8; 34]` freeze layout), no `borsh`/`bytemuck` derives.
- **Process problem:** Built it as a lambda-prize-style submission. On 2026-04-28, core team (`fryorcraken`) corrected them: *"For RFP, we expect a proposal with paid milestone breakdown, we can then contract the applicant to deliver the product."* They have not yet filed a proper budget proposal — `under-review` label is stalled.
- **Implication for us:** Their working-code lead is real but their structural mismatch with RFP requirements (it's not a library, no SPEL integration) is large. Our advantage is delivering everything they have *plus* everything they're missing.

### Competitor 2 — mmlado (logos-co/rfp issue #46)

- **Approach:** Proc-macro contribution to `spel-framework-macros` — `#[admin_authority]` at module level + `#[require_admin]` at instruction level. Mirrors Solidity's Ownable pattern.
- **Budget:** $3,500 (M1: $875 / M2: $1,750 / M3: $875), 16-24 days.
- **Strengths:** 19 years software dev, two prior completed Logos deliveries (LP-0009 Keycard NIP-46, LP-0010 Shell dApp PoC), commits to draft PR upstream to `logos-co/spel` in M1, doc packet to `logos-co/logos-docs`.
- **Weaknesses:** $1,500 more expensive than us, depends on upstream `logos-co/spel` maintainer review/merge in M1 (they explicitly call this out as a risk in their proposal), no working code yet.
- **Implication for us:** mmlado is the credible technical competitor. Our differentiation: cheaper, no upstream-PR dependency risk, and we already have running V03State integration tests proving capability.

### Our Proposal — Jason Randazza (logos-co/rfp issue #50)

- **Budget:** $2,000 (M1: $500 / M2: $1,000 / M3: $500), 4 weeks.
- **Status:** Updated 2026-05-05 with progress comment listing every hard/soft RFP requirement as ✅ Complete, formal milestone table, link to working repo with green CI.
- **Same reviewer assigned:** `hackyguru` (also assigned to mmlado's #46).

---

## Implementation Architecture

### Crates

- **`admin-authority`** — reusable library. Defines `AdminAuthority`, `AdminKey { Signer, Pda }`, `AdminAuthorityError`. Borsh + serde serializable. No SPEL or LEZ-specific code beyond `nssa_core::AccountWithMetadata`.
- **`admin-authority-sample`** — sample SPEL program. Demonstrates the full flow (init / transfer / revoke / gated update_config). Uses `#[lez_program]` macro and `#[account(admin)]` annotation.
- **`admin-authority-sample-methods`** — RISC Zero ELF wrapper. `risc0-build::embed_methods()` produces `ADMIN_AUTHORITY_SAMPLE_ELF: &[u8]` and `ADMIN_AUTHORITY_SAMPLE_ID: [u32; 8]` constants for tests/clients.
- **`admin-authority-sample-methods/guest`** — explicit workspace member; thin binary wrapper calling `admin_authority_sample::main()`.
- **`integration-tests`** — two test files: `admin_authority.rs` (LEZ-layer validator tests, 3 tests) and `v03state.rs` (end-to-end V03State tests, 4 tests).

### SPEL Integration Pattern

Programs gate any instruction with a single annotation: `#[account(admin)]`. The SPEL framework's generated validator decodes the `admin_authority` PDA, calls `AdminAuthority::assert_admin`, and rejects unauthorized transactions before the handler runs.

### Local SPEL Patch Files (NOT in this repo — preserve before clean checkout)

The project depends on local SPEL changes that are not yet upstreamed:

- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/types.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/idl.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/idl_gen.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/src/pda.rs` (AccountId::for_public_pda fix)
- `/home/jrazz/logos-bootcamp/spel/spel-framework-core/tests/variable_accounts.rs`
- `/home/jrazz/logos-bootcamp/spel/spel-framework-macros/src/lib.rs` (recognises `#[account(admin)]`)
- `/home/jrazz/logos-bootcamp/spel/spel-client-gen/src/tests.rs`

If SPEL upstream merges these (mmlado's #46 commits to a draft PR upstream), our `Cargo.toml` patch section can be dropped.

### Workspace dependency patching

`Cargo.toml` patches `https://github.com/logos-blockchain/logos-execution-zone.git` to use the local `nssa_core` checkout, avoiding duplicate `AccountId` types.

After upstream `logos-execution-zone` commit `f37454ed` "Refactor signatures", `AccountId::from((program_id, &pda_seed))` was replaced by `AccountId::for_public_pda(program_id, &pda_seed)` — applied in the SPEL `pda.rs` patch.

---

## RFP Requirements Coverage

Cross-referenced against the RFP spec at `/home/jrazz/logos-bootcamp/RFP-001-admin-authority-lib.md`:

| Requirement | Type | Implementation | Test |
|---|---|---|---|
| Admin set at program initialization | Hard — Functionality | `initialize` instruction in sample | `v03state_initialize_sets_admin_and_config` |
| Transfer authority to new signer/PDA | Hard — Functionality | `transfer_admin` accepts `AdminKey::Signer \| Pda` | `v03state_transfer_admin_updates_authority` + 4 unit tests |
| Revoke (renounce) authority | Hard — Functionality | `revoke_admin` instruction; permanent | `v03state_revoke_admin_blocks_further_updates` + unit |
| Gated config PDA update | Hard — Functionality | `#[account(admin)]` on `update_config` | `v03state_update_config_succeeds_for_admin` + LEZ-layer reject test |
| SPEL integration, single annotation | Hard — Usability | `#[account(admin)]` macro patch | `idl_marks_admin_gated_accounts` |
| Only one admin at a time | Hard — Usability | `Option<AdminKey>` (single-valued) | enforced by type |
| README + step-by-step integration + e2e example | Hard — Supportability | README.md (4-step guide, full code example) | manual review |
| Transaction size overhead documented | Hard — Performance | README overhead table (+128 / +32 bytes) | — |
| CI green on default branch | Hard — Supportability | `.github/workflows/ci.yml` (fmt + clippy + test) | runs on every push |
| Every hard req has a test | Hard — Supportability | 20 tests (will become 23 after current work) | listed above |
| Sample program imports library | Hard — Supportability | `admin-authority-sample` depends on `admin-authority` | — |
| Valid signer/PDA only | Soft — Reliability | `AdminKey::validate()` rejects default; `transfer_admin` takes `AdminKey` typed param | `transfer_rejects_invalid_new_admin` |

---

## Test Inventory

| Crate | File | Tests | Type |
|---|---|---|---|
| `admin-authority` | `src/lib.rs` (mod tests) | 6 → **9** (+3 added in current session) | Unit (core library) |
| `admin-authority-sample` | `src/lib.rs` (mod tests) | 5 | Unit (SPEL macro + IDL) |
| `integration-tests` | `tests/admin_authority.rs` | 3 | LEZ validator layer |
| `integration-tests` | `tests/v03state.rs` | 4 | E2E V03State (compiled ELF) |

Total: **20 → 21 tests** (current session added 3 unit tests; the count below depends on whether the optional V03State failure-path test gets added).

---

## Recently Completed (current session, 2026-05-05)

In strict order:

1. **GitHub Actions CI** (`.github/workflows/ci.yml`) — fmt, clippy `-D warnings`, test with `RISC0_DEV_MODE=1`. Hard RFP requirement.
2. **README overhaul** — 4-step integration guide, end-to-end example, transaction overhead table (+128 bytes per gated instruction when admin signer dedicated, +32 bytes when shared), API reference. Hard RFP requirement.
3. **`transfer_admin` accepts `AdminKey`** instead of `AccountId` — satisfies the soft requirement properly (caller distinguishes `Signer` from `Pda`).
4. **Added `serde::{Serialize, Deserialize}` derives to `AdminKey`** — required by SPEL instruction macro for non-account params; `serde.workspace = true` added to `admin-authority/Cargo.toml`.
5. **2 new V03State integration tests** — `v03state_transfer_admin_updates_authority`, `v03state_revoke_admin_blocks_further_updates`. Plus an `initialized_state()` helper to deduplicate setup across all 4 V03State tests.
6. **Code review pass + fixes (in progress as of this write):**
   - **Replaced `.expect()` panics in handlers with proper `SpelError` returns.** This was the highest-severity finding: `admin-authority-sample/src/lib.rs` previously used `.expect("admin transfer must be authorized")` which would panic *inside the LEZ guest* if a caller passed `AdminKey::Signer(AccountId::default())` to `transfer_admin`. The framework's own `error.rs` documents itself as "*Replaces the current pattern of `panic!` and `.expect()` with proper Result-based error handling.*" Helper functions `encode_admin`, `decode_admin`, `encode_config` now return `Result<_, SpelError>` and propagate via `?`. Added `admin_error_to_spel(AdminAuthorityError) -> SpelError::Unauthorized` adapter.
   - **Consolidated `assert_admin` match arms.** The `Signer` and `Pda` arms in `admin-authority/src/lib.rs:84-102` did identical work. Collapsed using `AdminKey::account_id()` accessor.
   - **Added `AdminKey::Pda` test coverage.** Three new unit tests in `admin-authority`: `assert_admin_accepts_pda_variant`, `assert_admin_rejects_pda_without_authorization`, `transfer_to_pda_variant_succeeds`. Previously every test used only `Signer`.

7. **Posted progress update on issue #50** (user did this manually) — comment maps every hard/soft RFP requirement to ✅ Complete, includes formal milestone table with deliverables/acceptance criteria/payment, links to repo with green CI.

---

## Active Work — DO THIS FIRST WHEN RESUMING

A `cargo test --workspace` run was kicked off after the .expect() / consolidation fixes but its result has **not yet been verified by the agent writing this document**. The next steps depend on the test outcome:

### Step 1 — Verify the test run

Check the most recent test output. If all tests pass, proceed to Step 2. If any fail, fix them.

```bash
RISC0_DEV_MODE=1 cargo test --workspace
```

Expected: 21 tests passing (was 20; +3 new unit tests for `AdminKey::Pda`; no test was removed).

**Watch for:** the consolidated `assert_admin` should not change behaviour but the test counts and assertions need to confirm. If `revoke_blocks_future_privileged_calls` fails, the consolidation has a bug.

### Step 2 — (Optional but recommended) Add a V03State failure-path test

This is **not RFP-required** but strengthens the proposal: confirms the LEZ rejects bad input gracefully (now that handlers return `SpelError` instead of panicking).

```rust
#[test]
fn v03state_transfer_admin_rejects_invalid_new_admin() {
    let (mut state, admin_key, admin_id) = initialized_state();
    let invalid = Message::try_new(
        ADMIN_AUTHORITY_SAMPLE_ID,
        vec![pda("admin_authority"), admin_id],
        vec![Nonce(1)],
        Instruction::TransferAdmin {
            new_admin: AdminKey::Signer(AccountId::default()),
        },
    ).unwrap();
    // Expect the V03State transition to error, not panic.
    let result = state.transition_from_public_transaction(
        &PublicTransaction::new(
            invalid.clone(),
            WitnessSet::for_message(&invalid, &[&admin_key]),
        ), 2, 0,
    );
    assert!(result.is_err());
}
```

**Caveat:** the exact error returned by `V03State::transition_from_public_transaction` when a SPEL handler returns `Err(SpelError)` may need investigation — it might be wrapped, surface as a generic transaction-failed error, or panic out of the guest depending on how risc0/V03State surface guest errors. **If unsure, run the test and inspect the error variant before asserting on it.**

### Step 3 — Run final checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
nix flake check
```

If `nix flake check` fails on formatting (it has a separate sandboxed `rustfmt` that can disagree with the local one when there's a parent `rustfmt.toml`), run `rustfmt --edition 2024` directly on the changed files first. See git history (commit `f00d8fe`) for an example of this issue.

### Step 4 — Commit and push

The user has authorised commits and pushes for finished, verified work. Suggested commit message structure: lead with the bug-fix (handler panics), then the dedupe, then the new tests. Use `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>`.

---

## New Goals (post-current-work)

The user proposed (2026-05-05): "*Maybe we should move on to implementing this on the blockchain even though it has not been fully approved yet.*" — i.e., deploy the program to a real Logos network instead of just V03State simulation.

**Open questions for this goal:**
1. Is there a public Logos devnet RPC endpoint available, or is this gated behind core-team access?
2. The user previously said they don't have a node/sequencer running — confirm whether they need one for devnet or whether a hosted endpoint exists.
3. The deployment tooling (logos-scaffold? lez-cli? a custom client?) needs to be identified — `logos-dev-boost` MCP tools may help here.
4. Funding/faucet for the deployment account on devnet.

This goal is **NOT yet started**. Resume from the test verification first, then consult the user before pursuing deployment — premature on-chain work could waste limited devnet resources or expose the project before reviewers see the polished pre-approval state.

---

## Reference

- RFP spec: `/home/jrazz/logos-bootcamp/RFP-001-admin-authority-lib.md`
- SPEL local source: `/home/jrazz/logos-bootcamp/spel`
- LEZ/NSSA local source: `/home/jrazz/logos-bootcamp/logos-execution-zone`
- Logos Dev Boost MCP — provides scaffolding/docs/build-help tools (was disconnected during VSCode reload, reconnects automatically)
- Competing repos: https://github.com/ygd58/admin-authority-poc (working code, structural mismatch); mmlado (#46) has no public repo yet
