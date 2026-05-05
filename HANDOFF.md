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

- **`admin-authority`** — reusable library. Defines `AdminAuthority`, stored `AdminKey { Signer, Pda }`, transfer-time `AdminCandidate { Signer, Pda { program_id, seed } }`, and `AdminAuthorityError`. Borsh + serde serializable. Uses `nssa_core::AccountWithMetadata` plus `ProgramId`/`PdaSeed` for checked PDA derivation.
- **`admin-authority-sample`** — sample SPEL program. Demonstrates the full flow (init / transfer / revoke / gated update_config). Uses `#[lez_program]` macro and `#[account(admin)]` annotation.
- **`admin-authority-sample-methods`** — RISC Zero ELF wrapper. `risc0-build::embed_methods()` produces `ADMIN_AUTHORITY_SAMPLE_ELF: &[u8]` and `ADMIN_AUTHORITY_SAMPLE_ID: [u32; 8]` constants for tests/clients.
- **`admin-authority-sample-methods/guest`** — explicit workspace member; thin binary wrapper calling `admin_authority_sample::main()`.
- **`integration-tests`** — two test files: `admin_authority.rs` (LEZ-layer validator tests, 3 tests) and `v03state.rs` (end-to-end V03State tests, 5 tests).

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

This repo now carries the exact SPEL diff at `patches/spel-admin-authority.patch`. GitHub Actions checks out `logos-co/spel` at `3457c7431e9b5b88661ed87b53677511ef88d113`, applies that patch, and checks out `logos-blockchain/logos-execution-zone` at `f37454ed1e730c7588b3980962011b687112d0ac` so the path dependencies resolve in a fresh CI checkout.

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
| Transfer authority to new signer/PDA | Hard — Functionality | `transfer_admin` accepts `AdminCandidate::Signer \| Pda` plus the candidate account as proof | `v03state_transfer_admin_updates_authority`, `v03state_transfer_admin_accepts_deployed_pda` + unit tests |
| Revoke (renounce) authority | Hard — Functionality | `revoke_admin` instruction; permanent | `v03state_revoke_admin_blocks_further_updates` + unit |
| Gated config PDA update | Hard — Functionality | `#[account(admin)]` on `update_config` | `v03state_update_config_succeeds_for_admin` + LEZ-layer reject test |
| SPEL integration, single annotation | Hard — Usability | `#[account(admin)]` macro patch | `idl_marks_admin_gated_accounts` |
| Only one admin at a time | Hard — Usability | `Option<AdminKey>` (single-valued) | enforced by type |
| README + step-by-step integration + e2e example | Hard — Supportability | README.md (4-step guide, full code example) | manual review |
| Transaction size overhead documented | Hard — Performance | README overhead table (+128 / +32 bytes) | — |
| CI green on default branch | Hard — Supportability | `.github/workflows/ci.yml` (scoped fmt + clippy + test with pinned SPEL/LEZ checkouts) | runs on every push |
| Every hard req has a test | Hard — Supportability | 30 tests | listed above |
| Sample program imports library | Hard — Supportability | `admin-authority-sample` depends on `admin-authority` | — |
| Valid signer/PDA only | Soft — Reliability | `AdminCandidate` validates the target at transfer time: signer targets must match an authorized candidate account; PDA targets are derived from program id + seed and must match an initialized/claimed account. | `checked_transfer_to_signer_requires_new_admin_authorization`, `checked_transfer_to_pda_requires_derived_initialized_account`, `v03state_transfer_admin_rejects_unsigned_new_signer`, `v03state_transfer_admin_accepts_deployed_pda` |

---

## Test Inventory

| Crate | File | Tests | Type |
|---|---|---|---|
| `admin-authority` | `src/lib.rs` (mod tests) | 13 | Unit (core library) |
| `admin-authority-sample` | `src/lib.rs` (mod tests) | 7 | Unit (SPEL macro + IDL) |
| `integration-tests` | `tests/admin_authority.rs` | 3 | LEZ validator layer |
| `integration-tests` | `tests/v03state.rs` | 7 | E2E V03State (compiled ELF) |

Total: **30 tests**.

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

8. **Codex review pass (2026-05-05)**:
   - Added real V03State post-transfer coverage: old admin can no longer update config; new admin can.
   - Strengthened `v03state_revoke_admin_blocks_further_updates` so it actually submits a post-revoke `update_config` and expects rejection.
   - Added `v03state_transfer_admin_rejects_invalid_new_admin`, confirming invalid default-key transfer returns an error and leaves authority unchanged.
   - Reworked README examples to use `SpelError`/`?` instead of `.expect()` inside guest handlers and to use non-deprecated `SpelOutput::execute`.
   - Fixed CI reproducibility by checking out pinned SPEL/LEZ siblings, applying `patches/spel-admin-authority.patch`, installing pinned RISC Zero rust/cpp/r0vm components, scoping rustfmt to this repo, and using `RISC0_SKIP_BUILD=1` for clippy.
   - Corrected documentation that overclaimed on-curve/deployed-PDA validation. That remained the main known enhancement if the RFP reviewer treated the soft reliability item as mandatory.

9. **Checked transfer implementation (2026-05-05)**:
   - Replaced the primary transfer path with `AdminAuthority::transfer(current_admin, AdminCandidate, candidate_account)`.
   - Added `AdminCandidate::Signer(AccountId)` proof: the candidate account id must match and `is_authorized` must be true, so LEZ has verified a valid signature for the new signer in the transfer transaction.
   - Added `AdminCandidate::Pda { program_id, seed }` proof: the PDA id is derived with `AccountId::for_public_pda(program_id, PdaSeed::new(seed))`, must match the provided candidate account, and the account must already be initialized/claimed.
   - Updated the sample SPEL `transfer_admin` instruction to include `new_admin_account` and call the checked library transfer.
   - Added unit, LEZ-style, and V03State tests for unsigned signer rejection and deployed PDA acceptance.

---

## Active Work — DO THIS FIRST WHEN RESUMING

There is no known failing code issue at this handoff. Before making new changes, confirm the current working tree and rerun the verification commands below if needed.

```bash
cargo check --workspace
find admin-authority admin-authority-sample admin-authority-sample-methods integration-tests \
  -name '*.rs' -print0 | xargs -0 rustfmt --edition 2024 --check
RISC0_SKIP_BUILD=1 cargo clippy --workspace --all-targets -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace
nix flake check
```

Local note: in the Codex sandbox, V03State/RISC Zero execution failed with `Operation not permitted`; running the same `RISC0_DEV_MODE=1 cargo test --workspace` outside the sandbox passed.

### Commit and push

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
