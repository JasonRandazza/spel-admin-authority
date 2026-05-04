# SPEL Admin Authority

Reusable admin authority primitives for Logos Execution Zone programs written with SPEL.

This crate provides:

- `AdminAuthority` account state for one active admin at a time.
- `transfer` and `revoke` helpers.
- SPEL integration through `#[account(admin)]` in the local framework patch.
- A sample SPEL config PDA update gated by the admin authority.

## Usage

Consumer programs store an `AdminAuthority` value in an `admin_authority` PDA and mark privileged signer accounts with `#[account(admin)]`.

```rust
#[instruction]
pub fn update_config(
    #[account(pda = literal("admin_authority"))]
    admin_authority: AccountWithMetadata,
    #[account(mut, pda = literal("config"))]
    config: AccountWithMetadata,
    #[account(admin)]
    admin: AccountWithMetadata,
    value: u64,
) -> SpelResult {
    // handler runs only after generated admin validation succeeds
    # Ok(SpelOutput::empty())
}
```

The extra transaction account overhead for each gated instruction is one authority-state account plus one admin signer account when the signer is not already otherwise required by the instruction.

## Development

```bash
cargo fmt --check
cargo test --workspace
RISC0_DEV_MODE=1 cargo test -p integration-tests
nix flake check
```
