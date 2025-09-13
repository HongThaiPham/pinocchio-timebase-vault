use core::mem::transmute;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_token::state::Mint;

use crate::{
    errors::TimeBaseVaultError,
    states::Vault,
    utils::{load_acc_mut_unchecked, DataLen},
};

pub struct InitializeSplVaultAccounts<'info> {
    pub signer: &'info AccountInfo,
    pub vault: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub user_ata: &'info AccountInfo,
    pub vault_ata: &'info AccountInfo,
    pub token_program: &'info AccountInfo,
    pub associated_token_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for InitializeSplVaultAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [signer, vault, mint, user_ata, vault_ata, token_program, associated_token_program, system_program] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !signer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // verify vault account
        if !vault.is_writable() {
            return Err(ProgramError::InvalidAccountData);
        }
        if !vault.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        Ok(InitializeSplVaultAccounts {
            vault,
            signer,
            mint,
            user_ata,
            vault_ata,
            token_program,
            associated_token_program,
            system_program,
        })
    }
}

#[repr(C, packed)]
pub struct InitializeSplVaultInstructionData {
    pub amount: u64,
    pub unlock_timestamp: i64,
    pub bump: u8,
}

impl DataLen for InitializeSplVaultInstructionData {
    const LEN: usize = core::mem::size_of::<InitializeSplVaultInstructionData>();
}

impl<'info> TryFrom<&'info [u8]> for InitializeSplVaultInstructionData {
    type Error = ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(unsafe {
            transmute(
                TryInto::<[u8; Self::LEN]>::try_into(data)
                    .map_err(|_| ProgramError::InvalidInstructionData)?,
            )
        })
    }
}

pub struct InitializeSplVault<'info> {
    pub accounts: InitializeSplVaultAccounts<'info>,
    pub instruction_data: InitializeSplVaultInstructionData,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for InitializeSplVault<'info> {
    type Error = ProgramError;

    fn try_from(
        (data, accounts): (&'info [u8], &'info [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeSplVaultAccounts::try_from(accounts)?;
        let instruction_data = InitializeSplVaultInstructionData::try_from(data)?;

        Ok(InitializeSplVault {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> InitializeSplVault<'info> {
    pub const DISCRIMINATOR: &'info u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let current_timestamp = Clock::get()?.unix_timestamp;
        let unlock_timestamp = self.instruction_data.unlock_timestamp;
        let amount = self.instruction_data.amount;
        if unlock_timestamp.lt(&current_timestamp) {
            return Err(TimeBaseVaultError::UnlockTimestampMustBeInFuture.into());
        }
        if amount.eq(&0) {
            return Err(TimeBaseVaultError::AmountMustBeGreaterThanZero.into());
        }

        Vault::validate_pda(
            self.accounts.vault.key(),
            self.accounts.signer.key(),
            self.instruction_data.amount,
            self.instruction_data.unlock_timestamp,
            self.instruction_data.bump,
            Some(*self.accounts.mint.key()),
        )?;

        {
            // create and init vault account
            let bump_binding = [self.instruction_data.bump];
            let amount_bytes = amount.to_le_bytes();
            let unlock_timestamp_bytes = unlock_timestamp.to_le_bytes();
            let seed = [
                Seed::from(Vault::SEED),
                Seed::from(self.accounts.signer.key()),
                Seed::from(self.accounts.mint.key()),
                Seed::from(&amount_bytes),
                Seed::from(&unlock_timestamp_bytes),
                Seed::from(&bump_binding),
            ];
            let signer_seeds = Signer::from(&seed);

            pinocchio_system::instructions::CreateAccount {
                from: self.accounts.signer,
                to: self.accounts.vault,
                space: Vault::LEN as u64,
                lamports: Rent::get()?.minimum_balance(Vault::LEN),
                owner: &crate::ID,
            }
            .invoke_signed(&[signer_seeds])?;

            let mut data: pinocchio::account_info::RefMut<'_, [u8]> =
                self.accounts.vault.try_borrow_mut_data()?;
            let vault = unsafe { load_acc_mut_unchecked::<Vault>(&mut data) }?;

            vault.mint = Some(*self.accounts.mint.key());
            vault.unlock_timestamp = self.instruction_data.unlock_timestamp.to_le_bytes();
            vault.amount = self.instruction_data.amount.to_le_bytes();
            vault.bump = [self.instruction_data.bump];
        }

        {
            // create associated token account for vault
            pinocchio_associated_token_account::instructions::Create {
                account: self.accounts.vault_ata,
                mint: self.accounts.mint,
                funding_account: self.accounts.signer,
                system_program: self.accounts.system_program,
                token_program: self.accounts.token_program,
                wallet: self.accounts.vault,
            }
            .invoke()?;

            // transfer spl token to vault
            pinocchio_token::instructions::TransferChecked {
                mint: self.accounts.mint,
                from: self.accounts.user_ata,
                to: self.accounts.vault_ata,
                amount: self.instruction_data.amount,
                authority: self.accounts.signer,
                decimals: Mint::from_account_info(self.accounts.mint)?.decimals(),
            }
            .invoke()?;
        }
        Ok(())
    }
}
