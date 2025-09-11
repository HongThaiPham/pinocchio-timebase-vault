use pinocchio::{
    program_error::ProgramError,
    pubkey::{self, Pubkey},
};

use crate::utils::DataLen;

#[repr(C)]
pub struct Vault {
    /// The owner of the vault
    pub owner: Pubkey,
    /// The amount of tokens in the vault
    pub amount: [u8; 8], // u64 as bytes
    /// The bump seed for the vault
    pub bump: [u8; 1],
    /// The unlock timestamp for the vault
    pub unlock_timestamp: [u8; 8], //i64 as bytes
    /// The mint address of the token in the vault (if applicable)
    pub mint: Option<Pubkey>,
}

impl DataLen for Vault {
    const LEN: usize = core::mem::size_of::<Vault>();
}

impl Vault {
    pub const SEED: &'static [u8] = b"vault";

    pub fn validate_pda(
        target: &Pubkey,
        signer: &Pubkey,
        amount: u64,
        unlock_timestamp: i64,
        bump: u8,
        mint: Option<Pubkey>,
    ) -> Result<(), ProgramError> {
        match mint {
            Some(mint) => {
                let seed_with_bump = &[
                    Self::SEED,
                    signer.as_ref(),
                    mint.as_ref(),
                    &amount.to_le_bytes(),
                    &unlock_timestamp.to_le_bytes(),
                    &[bump],
                ];
                let expected = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
                if expected != *target {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            None => {
                let seed_with_bump = &[
                    Self::SEED,
                    signer.as_ref(),
                    &amount.to_le_bytes(),
                    &unlock_timestamp.to_le_bytes(),
                    &[bump],
                ];
                let expected = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
                if expected != *target {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
        }

        Ok(())
    }
}
