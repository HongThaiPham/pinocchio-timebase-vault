use core::mem::transmute;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    ProgramResult,
};

use crate::{
    errors::TimeBaseVaultError,
    states::Vault,
    utils::{load_acc_mut_unchecked, DataLen},
};

pub struct InitializeSolVaultAccounts<'info> {
    pub signer: &'info AccountInfo,
    pub vault: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for InitializeSolVaultAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [signer, vault, _] = accounts else {
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

        Ok(InitializeSolVaultAccounts {
            vault: vault,
            signer: signer,
        })
    }
}

#[repr(C, packed)]
pub struct InitializeSolVaultInstructionData {
    pub amount: u64,
    pub unlock_timestamp: i64,
    pub bump: u8,
}

impl DataLen for InitializeSolVaultInstructionData {
    const LEN: usize = core::mem::size_of::<InitializeSolVaultInstructionData>();
}

impl<'info> TryFrom<&'info [u8]> for InitializeSolVaultInstructionData {
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

pub struct InitializeSolVault<'info> {
    pub accounts: InitializeSolVaultAccounts<'info>,
    pub instruction_data: InitializeSolVaultInstructionData,
}

impl<'info> TryFrom<(&'info [u8], &'info [AccountInfo])> for InitializeSolVault<'info> {
    type Error = ProgramError;

    fn try_from(
        (data, accounts): (&'info [u8], &'info [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeSolVaultAccounts::try_from(accounts)?;
        let instruction_data = InitializeSolVaultInstructionData::try_from(data)?;

        Ok(InitializeSolVault {
            accounts,
            instruction_data,
        })
    }
}

impl<'info> InitializeSolVault<'info> {
    pub const DISCRIMINATOR: &'info u8 = &0;

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
            None,
        )?;

        {
            // create and init vault account
            let bump_binding = [self.instruction_data.bump];
            let amount_bytes = amount.to_le_bytes();
            let unlock_timestamp_bytes = unlock_timestamp.to_le_bytes();
            let seed = [
                Seed::from(Vault::SEED),
                Seed::from(self.accounts.signer.key()),
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

            vault.mint = None;
            vault.unlock_timestamp = self.instruction_data.unlock_timestamp.to_le_bytes();
            vault.amount = self.instruction_data.amount.to_le_bytes();
            vault.bump = [self.instruction_data.bump];
        }

        {
            // transfer sol to vault
            pinocchio_system::instructions::Transfer {
                from: self.accounts.signer,
                to: self.accounts.vault,
                lamports: self.instruction_data.amount,
            }
            .invoke()?;
        }
        Ok(())
    }
}
