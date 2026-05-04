//! Reusable admin authority primitives for SPEL/LEZ programs.

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::{AccountId, AccountWithMetadata, Data};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AdminAuthorityError {
    #[error("admin authority has been revoked")]
    Revoked,
    #[error("admin authority is not set")]
    MissingAdmin,
    #[error("candidate admin key must be non-default")]
    InvalidAdminKey,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
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
        new_admin: AdminKey,
    ) -> Result<()> {
        self.assert_admin(current_admin)?;
        self.admin = Some(new_admin.validate()?);
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

        match self.admin {
            Some(AdminKey::Signer(expected)) => {
                if authority.account_id != expected {
                    return Err(AdminAuthorityError::NotAdmin);
                }
                if !authority.is_authorized {
                    return Err(AdminAuthorityError::MissingSignature);
                }
                Ok(())
            }
            Some(AdminKey::Pda(expected)) => {
                if authority.account_id != expected {
                    return Err(AdminAuthorityError::NotAdmin);
                }
                if !authority.is_authorized {
                    return Err(AdminAuthorityError::MissingSignature);
                }
                Ok(())
            }
            None => Err(AdminAuthorityError::MissingAdmin),
        }
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

    #[test]
    fn initialization_stores_exactly_one_admin() {
        let authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(1))));
        assert!(!authority.revoked);
    }

    #[test]
    fn transfer_succeeds_for_current_admin() {
        let current = account(id(1), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        authority
            .transfer(&current, AdminKey::Signer(id(2)))
            .unwrap();
        assert_eq!(authority.admin, Some(AdminKey::Signer(id(2))));
    }

    #[test]
    fn transfer_rejects_non_admin() {
        let current = account(id(9), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        let err = authority
            .transfer(&current, AdminKey::Signer(id(2)))
            .unwrap_err();
        assert_eq!(err, AdminAuthorityError::NotAdmin);
    }

    #[test]
    fn transfer_rejects_invalid_new_admin() {
        let current = account(id(1), true);
        let mut authority = AdminAuthority::new(AdminKey::Signer(id(1))).unwrap();
        let err = authority
            .transfer(&current, AdminKey::Signer(AccountId::default()))
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
    fn encode_decode_round_trips() {
        let authority = AdminAuthority::new(AdminKey::Pda(id(7))).unwrap();
        let data = authority.encode().unwrap();
        assert_eq!(AdminAuthority::decode(data.as_ref()).unwrap(), authority);
    }
}
