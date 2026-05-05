//! Sample SPEL program showing a config PDA gated by admin authority.

use admin_authority::{AdminAuthority, AdminAuthorityError, AdminCandidate, AdminKey};
use borsh::{BorshDeserialize, BorshSerialize};
use spel_framework::prelude::*;

#[account_type]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    pub value: u64,
}

fn encode_admin(authority: &AdminAuthority) -> Result<nssa_core::account::Data, SpelError> {
    authority
        .encode()
        .map_err(|e| SpelError::SerializationError {
            message: format!("admin authority encode failed: {e}"),
        })
}

fn decode_admin(
    account: &AccountWithMetadata,
    account_index: usize,
) -> Result<AdminAuthority, SpelError> {
    AdminAuthority::from_account(account).map_err(|e| SpelError::DeserializationError {
        account_index,
        message: format!("admin authority decode failed: {e}"),
    })
}

fn encode_config(config: &Config) -> Result<nssa_core::account::Data, SpelError> {
    let bytes = borsh::to_vec(config).map_err(|e| SpelError::SerializationError {
        message: format!("config encode failed: {e}"),
    })?;
    nssa_core::account::Data::try_from(bytes).map_err(|_| SpelError::SerializationError {
        message: "config exceeds account data size".to_string(),
    })
}

fn admin_error_to_spel(err: AdminAuthorityError) -> SpelError {
    SpelError::Unauthorized {
        message: err.to_string(),
    }
}

#[lez_program]
mod admin_authority_sample {
    #[allow(unused_imports)]
    use super::*;

    #[instruction]
    pub fn initialize(
        #[account(init, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
        #[account(init, pda = literal("config"))] mut config: AccountWithMetadata,
        #[account(signer)] admin: AccountWithMetadata,
        value: u64,
    ) -> SpelResult {
        let authority =
            AdminAuthority::new(AdminKey::Signer(admin.account_id)).map_err(admin_error_to_spel)?;

        admin_authority.account.data = encode_admin(&authority)?;
        config.account.data = encode_config(&Config { value })?;

        Ok(SpelOutput::execute(
            vec![admin_authority, config, admin],
            vec![],
        ))
    }

    #[instruction]
    pub fn transfer_admin(
        #[account(mut, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
        new_admin_account: AccountWithMetadata,
        new_admin: AdminCandidate,
    ) -> SpelResult {
        let mut authority = decode_admin(&admin_authority, 0)?;
        authority
            .transfer(&admin, new_admin, &new_admin_account)
            .map_err(admin_error_to_spel)?;

        admin_authority.account.data = encode_admin(&authority)?;

        Ok(SpelOutput::execute(
            vec![admin_authority, admin, new_admin_account],
            vec![],
        ))
    }

    #[instruction]
    pub fn revoke_admin(
        #[account(mut, pda = literal("admin_authority"))] mut admin_authority: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
    ) -> SpelResult {
        let mut authority = decode_admin(&admin_authority, 0)?;
        authority.revoke(&admin).map_err(admin_error_to_spel)?;

        admin_authority.account.data = encode_admin(&authority)?;

        Ok(SpelOutput::execute(vec![admin_authority, admin], vec![]))
    }

    #[instruction]
    pub fn update_config(
        #[account(pda = literal("admin_authority"))] admin_authority: AccountWithMetadata,
        #[account(mut, pda = literal("config"))] mut config: AccountWithMetadata,
        #[account(admin)] admin: AccountWithMetadata,
        value: u64,
    ) -> SpelResult {
        config.account.data = encode_config(&Config { value })?;

        Ok(SpelOutput::execute(
            vec![admin_authority, config, admin],
            vec![],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{Account, AccountId, Nonce};

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
            account_with(
                pda("config"),
                encode_config(&Config { value: 1 }).unwrap(),
                false,
            ),
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
            account_with(
                pda("config"),
                encode_config(&Config { value: 1 }).unwrap(),
                false,
            ),
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
            account_with(
                pda("config"),
                encode_config(&Config { value: 1 }).unwrap(),
                false,
            ),
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
        let config = account_with(
            pda("config"),
            encode_config(&Config { value: 1 }).unwrap(),
            false,
        );
        let admin = account_with(admin_id, nssa_core::account::Data::default(), true);

        let output =
            admin_authority_sample::update_config(admin_authority, config, admin, 99).unwrap();
        let updated =
            Config::try_from_slice(output.post_states[1].account().data.as_ref()).unwrap();
        assert_eq!(updated.value, 99);
    }

    #[test]
    fn transfer_admin_handler_requires_new_signer_authorization() {
        let admin_id = account_id(1);
        let new_admin_id = account_id(2);
        let authority = AdminAuthority::new(AdminKey::Signer(admin_id)).unwrap();
        let admin_authority =
            account_with(pda("admin_authority"), authority.encode().unwrap(), false);
        let admin = account_with(admin_id, nssa_core::account::Data::default(), true);
        let new_admin = account_with(new_admin_id, nssa_core::account::Data::default(), false);

        let err = admin_authority_sample::transfer_admin(
            admin_authority,
            admin,
            new_admin,
            AdminCandidate::Signer(new_admin_id),
        )
        .unwrap_err();

        assert!(matches!(err, SpelError::Unauthorized { .. }));
    }

    #[test]
    fn transfer_admin_handler_stores_checked_new_signer() {
        let admin_id = account_id(1);
        let new_admin_id = account_id(2);
        let authority = AdminAuthority::new(AdminKey::Signer(admin_id)).unwrap();
        let admin_authority =
            account_with(pda("admin_authority"), authority.encode().unwrap(), false);
        let admin = account_with(admin_id, nssa_core::account::Data::default(), true);
        let new_admin = account_with(new_admin_id, nssa_core::account::Data::default(), true);

        let output = admin_authority_sample::transfer_admin(
            admin_authority,
            admin,
            new_admin,
            AdminCandidate::Signer(new_admin_id),
        )
        .unwrap();
        let updated =
            AdminAuthority::decode(output.post_states[0].account().data.as_ref()).unwrap();

        assert_eq!(updated.admin, Some(AdminKey::Signer(new_admin_id)));
        assert!(!updated.revoked);
    }
}
