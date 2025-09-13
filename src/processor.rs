use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::instructions::{
    InitializeSolVault, InitializeSplVault, WithdrawSolVault, WithdrawSplVault,
};

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((InitializeSolVault::DISCRIMINATOR, data)) => {
            InitializeSolVault::try_from((data, accounts))?.process()
        }
        Some((WithdrawSolVault::DISCRIMINATOR, data)) => {
            WithdrawSolVault::try_from((data, accounts))?.process()
        }
        Some((InitializeSplVault::DISCRIMINATOR, data)) => {
            InitializeSplVault::try_from((data, accounts))?.process()
        }
        Some((WithdrawSplVault::DISCRIMINATOR, data)) => {
            WithdrawSplVault::try_from((data, accounts))?.process()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
