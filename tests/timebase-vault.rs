#[cfg(test)]
mod tests_timebase_vault {
    use mollusk_svm::{
        result::{Check, ProgramResult},
        Mollusk,
    };

    use pinocchio_timebase_vault::{
        instructions::{InitializeSolVault, InitializeSolVaultInstructionData},
        states::Vault,
        utils::{to_bytes, DataLen},
        ID,
    };
    use solana_sdk::{
        account::{Account, AccountSharedData, ReadableAccount},
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        program_error::ProgramError,
        pubkey::Pubkey,
    };

    pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(ID);

    fn get_mollusk() -> Mollusk {
        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/pinocchio_timebase_vault");
        mollusk
    }

    #[test]
    fn init_sol_vault() {
        let mollusk = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let amount = 1 * LAMPORTS_PER_SOL;
        let unlock_timestamp = mollusk.sysvars.clock.unix_timestamp + 3600;

        println!("unlock_timestamp: {}", unlock_timestamp);

        let (vault_address, bump) = Pubkey::find_program_address(
            &[
                Vault::SEED,
                maker.as_ref(),
                &amount.to_le_bytes(),
                &unlock_timestamp.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );

        let vault_account = Account::new(0, 0, &system_program);

        let ix_data = InitializeSolVaultInstructionData {
            amount,
            unlock_timestamp,
            bump,
        };

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(Vault::LEN);

        let mut data = vec![InitializeSolVault::DISCRIMINATOR.clone()];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(maker, true),
                AccountMeta::new(vault_address, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let result: mollusk_svm::result::InstructionResult = mollusk
            .process_and_validate_instruction(
                &instruction,
                &[
                    (maker, maker_account),
                    (vault_address, vault_account),
                    (system_program, system_account),
                ],
                &[
                    Check::success(),
                    Check::account(&vault_address).owner(&PROGRAM_ID).build(),
                    Check::account(&vault_address)
                        .lamports(amount + lamport_for_rent)
                        .build(),
                ],
            );
    }
}
