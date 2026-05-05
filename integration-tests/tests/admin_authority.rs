use admin_authority::{AdminAuthority, AdminCandidate, AdminKey};
use nssa_core::account::{Account, AccountId, AccountWithMetadata, Data, Nonce};
use nssa_core::program::ProgramId;

fn account_id(byte: u8) -> AccountId {
    AccountId::new([byte; 32])
}

fn program_id() -> ProgramId {
    [42u32; 8]
}

fn pda(seed: &str) -> AccountId {
    let seed = spel_framework::pda::seed_from_str(seed);
    spel_framework::pda::compute_pda(&program_id(), &[&seed])
}

fn account_with(id: AccountId, data: Data, authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: Account {
            program_owner: program_id(),
            balance: 0,
            data,
            nonce: Nonce(0),
        },
        is_authorized: authorized,
        account_id: id,
    }
}

#[test]
fn lez_style_admin_transfer_allows_new_admin_and_rejects_old_admin() {
    let old_admin = account_with(account_id(1), Data::default(), true);
    let new_admin = account_with(account_id(2), Data::default(), true);

    let mut authority = AdminAuthority::new(AdminKey::Signer(old_admin.account_id)).unwrap();
    authority
        .transfer(
            &old_admin,
            AdminCandidate::Signer(new_admin.account_id),
            &new_admin,
        )
        .unwrap();

    assert!(authority.assert_admin(&new_admin).is_ok());
    assert!(authority.assert_admin(&old_admin).is_err());
}

#[test]
fn lez_style_non_admin_cannot_update_config() {
    let authority = AdminAuthority::new(AdminKey::Signer(account_id(1))).unwrap();
    let accounts = vec![
        account_with(pda("admin_authority"), authority.encode().unwrap(), false),
        account_with(pda("config"), Data::default(), false),
        account_with(account_id(9), Data::default(), true),
    ];

    let err = admin_authority_sample::admin_authority_sample::__validate_update_config(
        &accounts,
        &program_id(),
        &vec![],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        spel_framework::error::SpelError::Unauthorized { .. }
    ));
}

#[test]
fn lez_style_revoke_blocks_future_admin_operations() {
    let admin = account_with(account_id(1), Data::default(), true);
    let mut authority = AdminAuthority::new(AdminKey::Signer(admin.account_id)).unwrap();
    authority.revoke(&admin).unwrap();

    let accounts = vec![
        account_with(pda("admin_authority"), authority.encode().unwrap(), false),
        account_with(pda("config"), Data::default(), false),
        admin,
    ];

    let err = admin_authority_sample::admin_authority_sample::__validate_update_config(
        &accounts,
        &program_id(),
        &vec![],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        spel_framework::error::SpelError::Unauthorized { .. }
    ));
}
