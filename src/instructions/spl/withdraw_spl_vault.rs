use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use pinocchio_token::state::Mint;

use crate::{errors::TimeBaseVaultError, states::Vault, utils::load_acc_unchecked};

pub struct WithdrawSplVaultAccounts<'info> {
    pub signer: &'info AccountInfo,
    pub vault: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub user_ata: &'info AccountInfo,
    pub vault_ata: &'info AccountInfo,
    pub token_program: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for WithdrawSplVaultAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [signer, vault, mint, user_ata, vault_ata, token_program, _] = accounts else {
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

        Ok(WithdrawSplVaultAccounts {
            vault,
            signer,
            mint,
            user_ata,
            vault_ata,
            token_program,
        })
    }
}

pub struct WithdrawSplVault<'info> {
    pub accounts: WithdrawSplVaultAccounts<'info>,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for WithdrawSplVault<'info> {
    type Error = ProgramError;

    fn try_from((_, accounts): (&'info [u8], &'info [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = WithdrawSplVaultAccounts::try_from(accounts)?;

        Ok(WithdrawSplVault { accounts })
    }
}

impl<'info> WithdrawSplVault<'info> {
    pub const DISCRIMINATOR: &'info u8 = &3;

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

        {
            let amount_bytes = vault.amount;
            let unlock_timestamp_bytes = vault.unlock_timestamp;
            let bump_binding = vault.bump;
            let seed = [
                Seed::from(Vault::SEED),
                Seed::from(self.accounts.signer.key()),
                Seed::from(self.accounts.mint.key()),
                Seed::from(&amount_bytes),
                Seed::from(&unlock_timestamp_bytes),
                Seed::from(&bump_binding),
            ];
            let signer_seeds = Signer::from(&seed);
            // transfer spl token to user
            pinocchio_token::instructions::TransferChecked {
                mint: self.accounts.mint,
                from: self.accounts.vault_ata,
                to: self.accounts.user_ata,
                amount: u64::from_le_bytes(vault.amount),
                authority: self.accounts.vault,
                decimals: Mint::from_account_info(self.accounts.mint)?.decimals(),
            }
            .invoke_signed(&[signer_seeds.clone()])?;

            pinocchio_token::instructions::CloseAccount {
                account: self.accounts.vault_ata,
                destination: self.accounts.signer,
                authority: self.accounts.vault,
            }
            .invoke_signed(&[signer_seeds])?;
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
