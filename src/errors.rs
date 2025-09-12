use pinocchio::program_error::ProgramError;

#[derive(Clone, PartialEq)]
pub enum TimeBaseVaultError {
    UnlockTimestampMustBeInFuture,
    AmountMustBeGreaterThanZero,
    Unauthorized,
    VaultLocking,
    InvalidVaultMint,
}

impl From<TimeBaseVaultError> for ProgramError {
    fn from(e: TimeBaseVaultError) -> Self {
        Self::Custom(e as u32)
    }
}
