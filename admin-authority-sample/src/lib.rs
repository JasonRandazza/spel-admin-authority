//! Sample SPEL program showing a config PDA gated by admin authority.

#![allow(dead_code, deprecated, unused_imports, unused_variables)]

use admin_authority::{AdminAuthority, AdminKey};
use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::AccountId;
use spel_framework::prelude::*;

#[account_type]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    pub value: u64,
}

fn encode_admin(authority: &AdminAuthority) -> nssa_core::account::Data {
    authority
        .encode()
        .expect("admin authority must fit in account data")
}

fn decode_admin(account: &AccountWithMetadata) -> AdminAuthority {
    AdminAuthority::from_account(account).expect("admin authority state must decode")
}

fn encode_config(config: &Config) -> nssa_core::account::Data {
    nssa_core::account::Data::try_from(borsh::to_vec(config).expect("config must serialize"))
        .expect("config must fit in account data")
}

fn decode_config(account: &AccountWithMetadata) -> Config {
    Config::try_from_slice(account.account.data.as_ref()).expect("config state must decode")
}

#[lez_program]
mod admin_authority_sample {
    #[allow(unused_imports)]
    use super::*;

    #[instruction]
    pub fn initialize(
        #[account(init, pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
        #[account(init, pda = literal("config"))] config: AccountWithMetadata,
        #[account(signer)] admin: AccountWithMetadata,
        value: u64,
    ) -> SpelResult {
        let mut admin_post = admin_authority.account.clone();
        admin_post.data = encode_admin(
            &AdminAuthority::new(AdminKey::Signer(admin.account_id))
                .expect("initial admin key must be valid"),
        );

        let mut config_post = config.account.clone();
        config_post.data = encode_config(&Config { value });

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new_claimed(admin_post, Claim::Authorized),
            AccountPostState::new_claimed(config_post, Claim::Authorized),
            AccountPostState::new(admin.account.clone()),
        ]))
    }

    #[instruction]
    pub fn transfer_admin(
        #[account(mut, pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
        new_admin: AccountId,
    ) -> SpelResult {
        let mut authority = decode_admin(&admin_authority);
        authority
            .transfer(&admin, AdminKey::Signer(new_admin))
            .expect("admin transfer must be authorized");

        let mut admin_post = admin_authority.account.clone();
        admin_post.data = encode_admin(&authority);

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(admin_post),
            AccountPostState::new(admin.account.clone()),
        ]))
    }

    #[instruction]
    pub fn revoke_admin(
        #[account(mut, pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
    ) -> SpelResult {
        let mut authority = decode_admin(&admin_authority);
        authority
            .revoke(&admin)
            .expect("admin revoke must be authorized");

        let mut admin_post = admin_authority.account.clone();
        admin_post.data = encode_admin(&authority);

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(admin_post),
            AccountPostState::new(admin.account.clone()),
        ]))
    }

    #[instruction]
    pub fn update_config(
        #[account(pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
        #[account(mut, pda = literal("config"))] config: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
        value: u64,
    ) -> SpelResult {
        let mut config_post = config.account.clone();
        config_post.data = encode_config(&Config { value });

        Ok(SpelOutput::states_only(vec![
            AccountPostState::new(admin_authority.account.clone()),
            AccountPostState::new(config_post),
            AccountPostState::new(admin.account.clone()),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{Account, Nonce};

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

    fn empty_ix_data() -> Vec<u32> {
        vec![]
    }

    fn account_with(
        id: AccountId,
        data: nssa_core::account::Data,
        authorized: bool,
    ) -> AccountWithMetadata {
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
    fn idl_marks_admin_gated_accounts() {
        let idl = __program_idl();
        let update = idl
            .instructions
            .iter()
            .find(|ix| ix.name == "update_config")
            .unwrap();
        assert!(update.accounts[2].admin);

        let idl_json: serde_json::Value = serde_json::from_str(PROGRAM_IDL_JSON).unwrap();
        let update_json = idl_json["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|ix| ix["name"] == "update_config")
            .unwrap();
        assert_eq!(update_json["accounts"][2]["admin"], true);
    }

    #[test]
    fn admin_validation_accepts_current_admin() {
        let admin_id = account_id(1);
        let authority = AdminAuthority::new(AdminKey::Signer(admin_id)).unwrap();
        let accounts = vec![
            account_with(pda("admin_authority"), authority.encode().unwrap(), false),
            account_with(pda("config"), encode_config(&Config { value: 1 }), false),
            account_with(admin_id, nssa_core::account::Data::default(), true),
        ];

        admin_authority_sample::__validate_update_config(
            &accounts,
            &program_id(),
            &empty_ix_data(),
        )
        .unwrap();
    }

    #[test]
    fn admin_validation_rejects_non_admin_before_handler() {
        let authority = AdminAuthority::new(AdminKey::Signer(account_id(1))).unwrap();
        let accounts = vec![
            account_with(pda("admin_authority"), authority.encode().unwrap(), false),
            account_with(pda("config"), encode_config(&Config { value: 1 }), false),
            account_with(account_id(9), nssa_core::account::Data::default(), true),
        ];

        let err = admin_authority_sample::__validate_update_config(
            &accounts,
            &program_id(),
            &empty_ix_data(),
        )
        .unwrap_err();
        assert!(matches!(err, SpelError::Unauthorized { .. }));
    }

    #[test]
    fn revoked_admin_blocks_update() {
        let mut authority = AdminAuthority::new(AdminKey::Signer(account_id(1))).unwrap();
        let admin = account_with(account_id(1), nssa_core::account::Data::default(), true);
        authority.revoke(&admin).unwrap();

        let accounts = vec![
            account_with(pda("admin_authority"), authority.encode().unwrap(), false),
            account_with(pda("config"), encode_config(&Config { value: 1 }), false),
            admin,
        ];

        let err = admin_authority_sample::__validate_update_config(
            &accounts,
            &program_id(),
            &empty_ix_data(),
        )
        .unwrap_err();
        assert!(matches!(err, SpelError::Unauthorized { .. }));
    }

    #[test]
    fn update_config_handler_writes_new_value_after_validation() {
        let admin_id = account_id(1);
        let authority = AdminAuthority::new(AdminKey::Signer(admin_id)).unwrap();
        let admin_authority =
            account_with(pda("admin_authority"), authority.encode().unwrap(), false);
        let config = account_with(pda("config"), encode_config(&Config { value: 1 }), false);
        let admin = account_with(admin_id, nssa_core::account::Data::default(), true);

        let output =
            admin_authority_sample::update_config(admin_authority, config, admin, 99).unwrap();
        let updated =
            Config::try_from_slice(output.post_states[1].account().data.as_ref()).unwrap();
        assert_eq!(updated.value, 99);
    }
}
