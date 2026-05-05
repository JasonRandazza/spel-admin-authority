# SPEL Admin Authority Deployment Runbook

This runbook tracks the path from green V03State tests to a real LEZ localnet/devnet deployment.

Status: planning only. Do not run the live sample until the user explicitly resumes this work.

## Readiness Checklist

- [ ] Confirm target network: scaffold-managed localnet, public devnet, or core-team hosted endpoint.
- [ ] Confirm wallet home and funding source.
- [ ] Build a release guest ELF for `admin_authority_sample`.
- [ ] Compute and record the program id from the submitted ELF.
- [ ] Deploy the ELF with `logos-scaffold deploy --program-path` or raw `wallet deploy-program`.
- [ ] Add a host runner/client for custom sample instructions.
- [ ] Execute and record the full admin lifecycle on a live sequencer.
- [ ] Capture final account states for `admin_authority` and `config`.
- [ ] Update `HANDOFF.md` with commands, program id, tx identifiers, block ids, and any failures.

## Known Tooling

### Scaffold

`logos-scaffold` is the preferred wrapper for a standalone localnet flow. Local source is at:

```text
/home/jrazz/logos-bootcamp/logos-scaffold
```

Relevant commands from scaffold:

```bash
logos-scaffold init
logos-scaffold setup
logos-scaffold localnet start
logos-scaffold localnet status --json
logos-scaffold doctor
logos-scaffold build
logos-scaffold deploy --program-path <path-to-guest-bin> --json
logos-scaffold wallet list --long
logos-scaffold wallet topup
logos-scaffold wallet -- check-health
logos-scaffold wallet -- chain-info current-block-id
```

Scaffold notes from the local implementation:

- `deploy --program-path` bypasses guest discovery and submits a custom ELF directly.
- It prints `program_id` by asking project-pinned `spel inspect` for the RISC Zero image id.
- Wallet home is project-local under `.scaffold/wallet`.
- Default local sequencer address is `http://127.0.0.1:3040`.
- Scaffold deployment currently confirms wallet submission, but does not expose a rich inclusion receipt.

### Raw LEZ Wallet

The raw wallet binary lives at:

```text
/home/jrazz/logos-bootcamp/logos-execution-zone/target/release/wallet
```

Useful raw commands:

```bash
export NSSA_WALLET_HOME_DIR=/path/to/wallet/home
wallet check-health
wallet account list
wallet chain-info current-block-id
wallet deploy-program <path-to-guest-bin>
```

The raw wallet can deploy arbitrary binaries, but it does not currently expose a generic CLI command for arbitrary custom public transactions. The admin sample lifecycle should therefore use a small host runner/client.

## Build The Guest ELF

The sample guest crate is:

```text
admin-authority-sample-methods/guest
```

Expected build command:

```bash
cargo build --release --manifest-path admin-authority-sample-methods/Cargo.toml
```

Expected artifact pattern after a release guest build:

```bash
find target admin-authority-sample-methods -path '*release*' -name 'admin_authority_sample*.bin'
```

If scaffold is adopted in this repo, `logos-scaffold build` can also build project methods, but the current workspace is not yet a scaffold-created project.

## Deploy Workflow

### Option A: Scaffold Localnet

Use this when we want a repeatable local deployment with project-local wallet state.

```bash
cd /home/jrazz/logos-bootcamp/spel-admin-authority
logos-scaffold init
logos-scaffold setup
logos-scaffold localnet start
logos-scaffold localnet status --json
logos-scaffold doctor
logos-scaffold wallet list --long
logos-scaffold wallet topup
cargo build --release --manifest-path admin-authority-sample-methods/Cargo.toml
logos-scaffold deploy --program-path <path-to-admin_authority_sample.bin> --json
logos-scaffold wallet -- chain-info current-block-id
```

Record:

- scaffold commit/ref
- LEZ commit/ref
- SPEL commit/ref and whether `patches/spel-admin-authority.patch` was applied
- wallet home path
- sequencer address
- guest binary path
- program id
- deploy tx id, if exposed
- current block id after deployment

### Option B: Existing Wallet/Node

Use this if a local wallet and sequencer are already running.

```bash
cd /home/jrazz/logos-bootcamp/spel-admin-authority
export NSSA_WALLET_HOME_DIR=/home/jrazz/logos-bootcamp/logos-scaffold/my-lez-app/.scaffold/wallet
/home/jrazz/logos-bootcamp/logos-execution-zone/target/release/wallet check-health
/home/jrazz/logos-bootcamp/logos-execution-zone/target/release/wallet account list
cargo build --release --manifest-path admin-authority-sample-methods/Cargo.toml
/home/jrazz/logos-bootcamp/logos-execution-zone/target/release/wallet deploy-program <path-to-admin_authority_sample.bin>
/home/jrazz/logos-bootcamp/logos-execution-zone/target/release/wallet chain-info current-block-id
```

This path is less reproducible unless the wallet home, sequencer address, and funding source are documented.

## Live Call Workflow

The sample instructions are public transactions. The host runner should use:

- `admin_authority_sample::Instruction`
- `admin_authority::{AdminCandidate, AdminKey}`
- `admin_authority_sample_methods::{ADMIN_AUTHORITY_SAMPLE_ELF, ADMIN_AUTHORITY_SAMPLE_ID}`
- `nssa::public_transaction::{Message, WitnessSet}`
- `nssa::{PublicTransaction, PrivateKey, PublicKey}`
- `wallet::WalletCore` or the sequencer client used by LEZ examples

Account ordering must match the SPEL handlers:

| Instruction | Accounts | Signers / Nonces |
| --- | --- | --- |
| `Initialize { value }` | `[admin_authority_pda, config_pda, admin_id]` | admin |
| `UpdateConfig { value }` | `[admin_authority_pda, config_pda, admin_id]` | admin |
| `TransferAdmin { Signer(new_admin_id) }` | `[admin_authority_pda, current_admin_id, new_admin_id]` | current admin + new admin |
| `TransferAdmin { Pda { program_id, seed } }` | `[admin_authority_pda, current_admin_id, candidate_pda]` | current admin |
| `RevokeAdmin` | `[admin_authority_pda, admin_id]` | admin |

The PDA ids are derived exactly as in tests:

```rust
use spel_framework::pda::{compute_pda, seed_from_str};

let admin_authority_pda = compute_pda(&program_id, &[&seed_from_str("admin_authority")]);
let config_pda = compute_pda(&program_id, &[&seed_from_str("config")]);
```

Live acceptance flow:

- [ ] Deploy program.
- [ ] Initialize admin and config with `value = 1`.
- [ ] Query/verify `admin_authority.admin == Some(Signer(admin_id))`.
- [ ] Query/verify `config.value == 1`.
- [ ] Update config as admin to `99`.
- [ ] Submit update as non-admin and confirm rejection.
- [ ] Transfer to a new signer with both current and new signer signatures.
- [ ] Confirm old admin update is rejected.
- [ ] Confirm new admin update succeeds.
- [ ] Transfer to an initialized PDA, if the target PDA/account is appropriate for the live flow.
- [ ] Revoke admin.
- [ ] Confirm all later privileged updates are rejected.

## Host Runner Needed

Before running the sample live, add a runner binary such as:

```text
integration-tests? no
admin-authority-client/src/bin/admin_authority_flow.rs
```

The runner should:

- read wallet/network config from `WalletCore::from_env()`
- accept `--program-id` or compute it from the ELF
- create or load two signer accounts
- derive the sample PDAs
- submit each instruction as a public transaction
- wait for block inclusion or poll `chain-info current-block-id`
- print a structured JSON transcript for README/HANDOFF evidence

The runner should not use test-only `V03State`. Its job is to prove the code path against an actual sequencer.

## Evidence Template

Fill this in after the live run:

```text
date:
network:
sequencer_addr:
wallet_home:
LEZ commit:
SPEL commit:
admin-authority commit:
guest_binary:
program_id:
deploy_tx:
deploy_block:
admin_authority_pda:
config_pda:
admin_id:
new_admin_id:

initialize:
update_as_admin:
update_as_non_admin_rejected:
transfer_to_signer:
old_admin_rejected:
new_admin_update:
transfer_to_pda:
revoke:
post_revoke_update_rejected:

notes:
```
