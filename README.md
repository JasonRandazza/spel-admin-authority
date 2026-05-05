# SPEL Admin Authority

A reusable admin authority library for [Logos Execution Zone](https://github.com/logos-co/logos-execution-zone) programs written with [SPEL](https://github.com/logos-co/spel).

[![CI](https://github.com/JasonRandazza/spel-admin-authority/actions/workflows/ci.yml/badge.svg)](https://github.com/JasonRandazza/spel-admin-authority/actions/workflows/ci.yml)

## What it does

- Stores a single active admin account id (`Signer` or `Pda`) in an `AdminAuthority` account.
- Gates any SPEL instruction behind admin authority with one annotation: `#[account(admin)]`.
- Provides `transfer` and `revoke` helpers that enforce the single-admin invariant and reject default/zero keys.
- Revocation is permanent — once revoked, no further privileged calls are accepted.

---

## Adding the library as a dependency

In your program's `Cargo.toml`:

```toml
[dependencies]
admin-authority = { git = "https://github.com/JasonRandazza/spel-admin-authority" }
```

You also need the SPEL framework patch that recognises `#[account(admin)]`. Until the patch is merged upstream, add the local path dependency in your workspace root:

```toml
[patch."https://github.com/logos-co/spel.git"]
spel-framework       = { path = "../spel/spel-framework" }
spel-framework-core  = { path = "../spel/spel-framework-core" }
spel-framework-macros = { path = "../spel/spel-framework-macros" }
```

---

## Step-by-step integration into a SPEL program

### Step 1 — Store `AdminAuthority` in a PDA

Every program that uses admin authority needs an `admin_authority` PDA to hold the authority state. Declare it in your `initialize` instruction:

```rust
#[instruction]
pub fn initialize(
    #[account(init, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
    #[account(signer)] admin: AccountWithMetadata,
) -> SpelResult {
    let authority =
        AdminAuthority::new(AdminKey::Signer(admin.account_id)).map_err(|e| {
            SpelError::Unauthorized {
                message: e.to_string(),
            }
        })?;

    admin_authority.account.data =
        authority.encode().map_err(|e| SpelError::SerializationError {
            message: format!("admin authority encode failed: {e}"),
        })?;

    Ok(SpelOutput::execute(vec![admin_authority, admin], vec![]))
}
```

### Step 2 — Gate privileged instructions with `#[account(admin)]`

Add `#[account(admin)]` to any instruction that must be restricted to the current admin. The SPEL framework automatically injects an authorization check before your handler runs — no manual check needed in the handler body.

```rust
#[instruction]
pub fn update_config(
    #[account(pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
    #[account(mut, pda = literal("config"))]    mut config: AccountWithMetadata,
    #[account(admin)]                           admin: AccountWithMetadata,
    value: u64,
) -> SpelResult {
    config.account.data = encode_config(&Config { value })?;

    Ok(SpelOutput::execute(vec![admin_authority, config, admin], vec![]))
}
```

The generated validator reads `admin_authority`, decodes the `AdminAuthority` state, and calls `assert_admin` before your handler body executes. A wrong signer or revoked authority returns `SpelError::Unauthorized` and the transaction is rejected.

### Step 3 — Add transfer and revoke instructions

Use the `transfer` and `revoke` helpers from `admin-authority`:

```rust
use admin_authority::{AdminAuthority, AdminKey};

#[instruction]
pub fn transfer_admin(
    #[account(mut, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
    #[account(admin)] admin: AccountWithMetadata,
    new_admin: AdminKey, // AdminKey::Signer(id) or AdminKey::Pda(id)
) -> SpelResult {
    let mut authority = AdminAuthority::from_account(&admin_authority)
        .map_err(|e| SpelError::DeserializationError {
            account_index: 0,
            message: format!("admin authority decode failed: {e}"),
        })?;
    authority
        .transfer(&admin, new_admin)
        .map_err(|e| SpelError::Unauthorized {
            message: e.to_string(),
        })?;

    admin_authority.account.data =
        authority.encode().map_err(|e| SpelError::SerializationError {
            message: format!("admin authority encode failed: {e}"),
        })?;

    Ok(SpelOutput::execute(vec![admin_authority, admin], vec![]))
}

#[instruction]
pub fn revoke_admin(
    #[account(mut, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
    #[account(admin)] admin: AccountWithMetadata,
) -> SpelResult {
    let mut authority = AdminAuthority::from_account(&admin_authority)
        .map_err(|e| SpelError::DeserializationError {
            account_index: 0,
            message: format!("admin authority decode failed: {e}"),
        })?;
    authority.revoke(&admin).map_err(|e| SpelError::Unauthorized {
        message: e.to_string(),
    })?;

    admin_authority.account.data =
        authority.encode().map_err(|e| SpelError::SerializationError {
            message: format!("admin authority encode failed: {e}"),
        })?;

    Ok(SpelOutput::execute(vec![admin_authority, admin], vec![]))
}
```

### Step 4 — Build and test

```bash
# Unit and integration tests (no ZK proof generation)
RISC0_DEV_MODE=1 cargo test --workspace

# Format check scoped to this workspace's source files
find admin-authority admin-authority-sample admin-authority-sample-methods integration-tests \
  -name '*.rs' -print0 | xargs -0 rustfmt --edition 2024 --check

# Clippy without rebuilding the RISC Zero guest ELF
RISC0_SKIP_BUILD=1 cargo clippy --workspace --all-targets -- -D warnings

# Nix reproducible build check
nix flake check
```

---

## End-to-end example

The `admin-authority-sample` crate in this workspace is a complete working example. It implements:

- `initialize` — sets up the `admin_authority` PDA and a `config` PDA
- `update_config` — gated by `#[account(admin)]`; updates `Config.value`
- `transfer_admin` — transfers authority to a new key (Signer or PDA)
- `revoke_admin` — permanently revokes authority

The `integration-tests` crate compiles the sample to a RISC Zero ELF, deploys it into `V03State`, and runs full end-to-end transactions covering all four instructions.

---

## Transaction overhead

Every instruction gated with `#[account(admin)]` requires two additional accounts compared to the same instruction without gating:

| Extra account | Purpose | Transaction size overhead |
| --- | --- | --- |
| `admin_authority` PDA | Holds the `AdminAuthority` state (35 bytes on-chain) | +32 bytes (AccountId in accounts list) |
| admin signer | The account exercising authority | +32 bytes AccountId + 64 bytes signature = **+96 bytes** |

**Total per-instruction overhead: ~128 bytes** when the admin signer is not already signing for another reason. If the admin signer is already required by the instruction (e.g., it is also the fee payer), only the `admin_authority` account adds overhead: **+32 bytes**.

The `AdminAuthority` account itself is 35 bytes on-chain (Borsh encoding of `Option<AdminKey>` + `bool`).

---

## API reference

### `AdminAuthority`

```rust
pub struct AdminAuthority {
    pub admin: Option<AdminKey>, // None after revocation
    pub revoked: bool,
}

impl AdminAuthority {
    pub fn new(admin: AdminKey) -> Result<Self>;
    pub fn transfer(&mut self, current: &AccountWithMetadata, new: AdminKey) -> Result<()>;
    pub fn revoke(&mut self, current: &AccountWithMetadata) -> Result<()>;
    pub fn assert_admin(&self, authority: &AccountWithMetadata) -> Result<()>;
    pub fn encode(&self) -> Result<Data>;
    pub fn decode(data: &[u8]) -> Result<Self>;
    pub fn from_account(account: &AccountWithMetadata) -> Result<Self>;
}
```

### `AdminKey`

```rust
pub enum AdminKey {
    Signer(AccountId), // public signer account id
    Pda(AccountId),    // program-derived account id
}
```

`AdminKey::validate()` rejects the default (all-zeros) `AccountId`. A stored signer is proven usable when it later authorizes a gated instruction; a stored PDA is proven usable when the runtime marks that PDA account as authorized.

### `AdminAuthorityError`

| Variant | Meaning |
| --- | --- |
| `Revoked` | Authority was permanently revoked |
| `MissingAdmin` | No admin set (should not occur outside of revoked state) |
| `InvalidAdminKey` | Candidate key is the default AccountId |
| `NotAdmin` | Presented account does not match stored admin |
| `MissingSignature` | Account is not marked `is_authorized` by the LEZ |

---

## Development

```bash
cargo check --workspace
RISC0_DEV_MODE=1 cargo test --workspace
find admin-authority admin-authority-sample admin-authority-sample-methods integration-tests \
  -name '*.rs' -print0 | xargs -0 rustfmt --edition 2024 --check
RISC0_SKIP_BUILD=1 cargo clippy --workspace --all-targets -- -D warnings
nix flake check
```
