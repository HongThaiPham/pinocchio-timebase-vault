use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::{errors::TimeBaseVaultError, states::Vault, utils::load_acc_unchecked};

pub struct WithdrawSolVaultAccounts<'info> {
    pub signer: &'info AccountInfo,
    pub vault: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for WithdrawSolVaultAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [signer, vault] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !signer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // verify vault account
        if !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if vault.data_is_empty() {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(WithdrawSolVaultAccounts {
            vault: vault,
            signer: signer,
        })
    }
}

pub struct WithdrawSolVault<'info> {
    pub accounts: WithdrawSolVaultAccounts<'info>,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for WithdrawSolVault<'info> {
    type Error = ProgramError;

    fn try_from((_, accounts): (&'info [u8], &'info [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = WithdrawSolVaultAccounts::try_from(accounts)?;

        Ok(WithdrawSolVault { accounts })
    }
}

impl<'info> WithdrawSolVault<'info> {
    pub const DISCRIMINATOR: &'info u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        let current_timestamp = Clock::get()?.unix_timestamp;

        let vault =
            unsafe { load_acc_unchecked::<Vault>(self.accounts.vault.borrow_data_unchecked()) }?;

        let unlock_timestamp = i64::from_le_bytes(vault.unlock_timestamp);
        if vault.owner.ne(self.accounts.signer.key()) {
            return Err(TimeBaseVaultError::Unauthorized.into());
        }
        if unlock_timestamp.gt(&current_timestamp) {
            return Err(TimeBaseVaultError::VaultLocking.into());
        }

        // close vault account and transfer all lamports to signer
        {
            let mut data = self.accounts.vault.try_borrow_mut_data()?;
            data[0] = 0xff;
        }

        *self.accounts.signer.try_borrow_mut_lamports()? +=
            *self.accounts.vault.try_borrow_lamports()?;
        self.accounts.vault.resize(1)?;
        self.accounts.vault.close()?;

        Ok(())
    }
}
