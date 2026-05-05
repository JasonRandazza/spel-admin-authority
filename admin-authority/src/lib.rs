//! Reusable admin authority primitives for SPEL/LEZ programs.

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Data},
    program::{DEFAULT_PROGRAM_ID, PdaSeed, ProgramId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AdminAuthorityError {
    #[error("admin authority has been revoked")]
    Revoked,
    #[error("admin authority is not set")]
    MissingAdmin,
    #[error("candidate admin key must be non-default")]
    InvalidAdminKey,
    #[error("candidate admin account does not match requested admin key")]
    AdminAccountMismatch,
    #[error("candidate admin PDA is not initialized")]
    UndeployedPda,
    #[error("account is not the current admin")]
    NotAdmin,
    #[error("admin signer authorization is missing")]
    MissingSignature,
    #[error("failed to decode admin authority: {0}")]
    Decode(String),
    #[error("failed to encode admin authority")]
    Encode,
}

pub type Result<T> = core::result::Result<T, AdminAuthorityError>;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub enum AdminKey {
    Signer(AccountId),
    Pda(AccountId),
}

impl AdminKey {
    pub fn account_id(&self) -> AccountId {
        match self {
            Self::Signer(id) | Self::Pda(id) => *id,
        }
    }

    pub fn validate(self) -> Result<Self> {
        if self.account_id() == AccountId::default() {
            return Err(AdminAuthorityError::InvalidAdminKey);
        }
        Ok(self)
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub enum AdminCandidate {
    Signer(AccountId),
    Pda {
        program_id: ProgramId,
        seed: [u8; 32],
    },
}

impl AdminCandidate {
    pub fn to_admin_key(self) -> Result<AdminKey> {
        match self {
            Self::Signer(id) => AdminKey::Signer(id).validate(),
            Self::Pda { program_id, seed } => {
                let id = AccountId::for_public_pda(&program_id, &PdaSeed::new(seed));
                AdminKey::Pda(id).validate()
            }
        }
    }

    pub fn validate_with_account(self, account: &AccountWithMetadata) -> Result<AdminKey> {
        let key = self.to_admin_key()?;
        if account.account_id != key.account_id() {
            return Err(AdminAuthorityError::AdminAccountMismatch);
        }

        match self {
            Self::Signer(_) => {
                if !account.is_authorized {
                    return Err(AdminAuthorityError::MissingSignature);
                }
            }
            Self::Pda { .. } => {
                if account.account == Account::default()
                    || account.account.program_owner == DEFAULT_PROGRAM_ID
                {
                    return Err(AdminAuthorityError::UndeployedPda);
                }
            }
        }

        Ok(key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AdminAuthority {
    pub admin: Option<AdminKey>,
    pub revoked: bool,
}

impl AdminAuthority {
    pub fn new(admin: AdminKey) -> Result<Self> {
        Ok(Self {
            admin: Some(admin.validate()?),
            revoked: false,
        })
    }

    pub fn transfer(
        &mut self,
        current_admin: &AccountWithMetadata,
        new_admin: AdminCandidate,
        new_admin_account: &AccountWithMetadata,
    ) -> Result<()> {
        self.assert_admin(current_admin)?;
        self.admin = Some(new_admin.validate_with_account(new_admin_account)?);
        Ok(())
    }

    pub fn revoke(&mut self, current_admin: &AccountWithMetadata) -> Result<()> {
        self.assert_admin(current_admin)?;
        self.admin = None;
        self.revoked = true;
        Ok(())
    }

    pub fn assert_admin(&self, authority: &AccountWithMetadata) -> Result<()> {
        if self.revoked {
            return Err(AdminAuthorityError::Revoked);
        }

        let expected = self.admin.ok_or(AdminAuthorityError::MissingAdmin)?;
        if authority.account_id != expected.account_id() {
            return Err(AdminAuthorityError::NotAdmin);
        }
        if !authority.is_authorized {
            return Err(AdminAuthorityError::MissingSignature);
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Data> {
        Data::try_from(borsh::to_vec(self).map_err(|_| AdminAuthorityError::Encode)?)
            .map_err(|_| AdminAuthorityError::Encode)
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        Self::try_from_slice(data).map_err(|e| AdminAuthorityError::Decode(e.to_string()))
    }

    pub fn from_account(account: &AccountWithMetadata) -> Result<Self> {
        Self::decode(account.account.data.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{Account, Nonce};
    use nssa_core::program::DEFAULT_PROGRAM_ID;

    fn id(byte: u8) -> AccountId {
        AccountId::new([byte; 32])
    }

    fn account(id: AccountId, authorized: bool) -> AccountWithMetadata {
        AccountWithMetadata {
            account: Account {
                program_owner: DEFAULT_PROGRAM_ID,
                balance: 0,
                data: Data::default(),
                nonce: Nonce(0),
            },
            is_authorized: authorized,
            account_id: id,
        }
    }

    fn initialized_account(id: AccountId, authorized: bool) -> AccountWithMetadata {
        AccountWithMetadata {
            account: Account {
                program_owner: [42; 8],
                balance: 0,
                data: Data::default(),
                nonce: Nonce(0),
            },
            is_authorized: authorized,
            account_id: id,
        }
    }

    #[test]
    fn initialization_stores_exactly_one_admin() {
        let authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(1))));
        assert!(!authority.revoked);
    }

    #[test]
    fn transfer_succeeds_for_current_admin() {
        let current = account(id(1), true);
        let new_admin = account(id(2), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        authority
            .transfer(&current, AdminCandidate::Signer(id(2)), &new_admin)
            .unwrap();
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(2))));
    }

    #[test]
    fn transfer_rejects_non_admin() {
        let current = account(id(9), true);
        let new_admin = account(id(2), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        let err = authority
            .transfer(&current, AdminCandidate::Signer(id(2)), &new_admin)
            .unwrap_err();
        assert_eq!(err, AdminAuthorityError::NotAdmin);
    }

    #[test]
    fn transfer_rejects_invalid_new_admin() {
        let current = account(id(1), true);
        let new_admin = account(AccountId::default(), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        let err = authority
            .transfer(
                &current,
                AdminCandidate::Signer(AccountId::default()),
                &new_admin,
            )
            .unwrap_err();
        assert_eq!(err, AdminAuthorityError::InvalidAdminKey);
    }

    #[test]
    fn revoke_blocks_future_privileged_calls() {
        let current = account(id(1), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        authority.revoke(&current).unwrap();
        assert!(authority.revoked);
        assert_eq!(authority.admin, None);
        assert_eq!(
            authority.assert_admin(&current).unwrap_err(),
            AdminAuthorityError::Revoked
        );
    }

    #[test]
    fn assert_admin_accepts_pda_variant() {
        let pda_id = id(7);
        let pda_account = account(pda_id, true);
        let authority = AdminAuthority::new(AdminKey::Pda(pda_id)).unwrap();
        assert!(authority.assert_admin(&pda_account).is_ok());
    }

    #[test]
    fn assert_admin_rejects_pda_without_authorization() {
        let pda_id = id(7);
        let pda_account = account(pda_id, false);
        let authority = AdminAuthority::new(AdminKey::Pda(pda_id)).unwrap();
        assert_eq!(
            authority.assert_admin(&pda_account).unwrap_err(),
            AdminAuthorityError::MissingSignature
        );
    }

    #[test]
    fn transfer_to_pda_variant_succeeds() {
        let current = account(id(1), true);
        let program_id = [42; 8];
        let seed = [5; 32];
        let pda_id = AccountId::for_public_pda(&program_id, &PdaSeed::new(seed));
        let pda_account = initialized_account(pda_id, false);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        authority
            .transfer(
                &current,
                AdminCandidate::Pda { program_id, seed },
                &pda_account,
            )
            .unwrap();
        assert_eq!(authority.admin, Some(AdminKey::Pda(pda_id)));
    }

    #[test]
    fn checked_transfer_to_signer_requires_new_admin_authorization() {
        let current = account(id(1), true);
        let new_admin = account(id(2), false);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();

        let err = authority
            .transfer(&current, AdminCandidate::Signer(id(2)), &new_admin)
            .unwrap_err();

        assert_eq!(err, AdminAuthorityError::MissingSignature);
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(1))));
    }

    #[test]
    fn checked_transfer_rejects_mismatched_candidate_account() {
        let current = account(id(1), true);
        let new_admin = account(id(9), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();

        let err = authority
            .transfer(&current, AdminCandidate::Signer(id(2)), &new_admin)
            .unwrap_err();

        assert_eq!(err, AdminAuthorityError::AdminAccountMismatch);
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(1))));
    }

    #[test]
    fn checked_transfer_to_pda_requires_derived_initialized_account() {
        let current = account(id(1), true);
        let program_id = [42; 8];
        let seed = [7; 32];
        let pda_id = AccountId::for_public_pda(&program_id, &PdaSeed::new(seed));
        let pda_account = initialized_account(pda_id, false);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();

        authority
            .transfer(
                &current,
                AdminCandidate::Pda { program_id, seed },
                &pda_account,
            )
            .unwrap();

        assert_eq!(authority.admin, Some(AdminKey::Pda(pda_id)));
    }

    #[test]
    fn checked_transfer_rejects_undeployed_pda() {
        let current = account(id(1), true);
        let program_id = [42; 8];
        let seed = [7; 32];
        let pda_id = AccountId::for_public_pda(&program_id, &PdaSeed::new(seed));
        let pda_account = account(pda_id, false);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();

        let err = authority
            .transfer(
                &current,
                AdminCandidate::Pda { program_id, seed },
                &pda_account,
            )
            .unwrap_err();

        assert_eq!(err, AdminAuthorityError::UndeployedPda);
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(1))));
    }

    #[test]
    fn encode_decode_round_trips() {
        let authority = AdminAuthority::new(AdminKey::Pda(id(7))).unwrap();
        let data = authority.encode().unwrap();
        assert_eq!(AdminAuthority::decode(data.as_ref()).unwrap(), authority);
    }
}
