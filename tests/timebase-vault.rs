#[cfg(test)]
mod tests_timebase_vault {
    use mollusk_svm::{result::Check, Mollusk};

    use mollusk_svm_programs_token::token::{
        create_account_for_mint, create_account_for_token_account,
    };
    use pinocchio_timebase_vault::{
        instructions::{
            InitializeSolVault, InitializeSolVaultInstructionData, InitializeSplVault,
            InitializeSplVaultInstructionData, WithdrawSolVault, WithdrawSplVault,
        },
        states::Vault,
        utils::{to_bytes, DataLen},
        ID,
    };
    use solana_sdk::{
        account::{Account, AccountSharedData},
        instruction::{AccountMeta, Instruction},
        native_token::LAMPORTS_PER_SOL,
        program_error::ProgramError,
        pubkey::Pubkey,
    };
    use spl_associated_token_account::get_associated_token_address;
    use spl_token::state::{Account as TokenAccount, AccountState, Mint};

    pub const PROGRAM_ID: Pubkey = Pubkey::new_from_array(ID);

    fn get_mollusk() -> (Mollusk, Pubkey, Account) {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/pinocchio_timebase_vault");
        // Add the SPL Token Program
        mollusk_svm_programs_token::token::add_program(&mut mollusk);

        // Add the Token2022 Program
        mollusk_svm_programs_token::token2022::add_program(&mut mollusk);

        // Add the Associated Token Program
        mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);
        mollusk.sysvars.clock.unix_timestamp = 1757633343;

        let mint = Pubkey::new_unique();
        let mint_data = Mint {
            mint_authority: None.into(),
            supply: 10_000_000_000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None.into(),
        };

        let mint_account = create_account_for_mint(mint_data);

        (mollusk, mint, mint_account)
    }

    #[test]
    fn init_sol_vault() {
        let (mollusk, _, _) = get_mollusk();

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

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
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
    #[test]
    fn withdraw_sol_vault_successfully() {
        let (mut mollusk, _, _) = get_mollusk();

        let (system_program, _) = mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let amount = 2 * LAMPORTS_PER_SOL;
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

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(Vault::LEN);

        let vault_account_data = Vault {
            owner: maker.to_bytes(),
            amount: amount.to_le_bytes(),
            unlock_timestamp: unlock_timestamp.to_le_bytes(),
            mint: None,
            bump: [bump],
        };

        let mut vault_account =
            AccountSharedData::new(lamport_for_rent + amount, Vault::LEN, &PROGRAM_ID);

        vault_account.set_data_from_slice(unsafe { to_bytes::<Vault>(&vault_account_data) });

        let data = vec![WithdrawSolVault::DISCRIMINATOR.clone()];

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(maker, true),
                AccountMeta::new(vault_address, false),
            ],
        );

        mollusk.sysvars.clock.unix_timestamp = unlock_timestamp + 100;

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (maker, maker_account),
                (vault_address, vault_account.into()),
            ],
            &[
                Check::success(),
                Check::account(&vault_address).closed().build(),
            ],
        );
    }

    #[test]
    fn withdraw_sol_vault_fail_with_vault_locking() {
        let (mollusk, _, _) = get_mollusk();

        let (system_program, _) = mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let amount = 2 * LAMPORTS_PER_SOL;
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

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(Vault::LEN);

        let vault_account_data = Vault {
            owner: maker.to_bytes(),
            amount: amount.to_le_bytes(),
            unlock_timestamp: unlock_timestamp.to_le_bytes(),
            mint: None,
            bump: [bump],
        };

        let mut vault_account =
            AccountSharedData::new(lamport_for_rent + amount, Vault::LEN, &PROGRAM_ID);

        vault_account.set_data_from_slice(unsafe { to_bytes::<Vault>(&vault_account_data) });

        let data = vec![WithdrawSolVault::DISCRIMINATOR.clone()];

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(maker, true),
                AccountMeta::new(vault_address, false),
            ],
        );

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (maker, maker_account),
                (vault_address, vault_account.into()),
            ],
            &[
                Check::err(ProgramError::Custom(3)), // VaultLocking
                Check::account(&vault_address).owner(&PROGRAM_ID).build(),
                Check::account(&vault_address)
                    .lamports(amount + lamport_for_rent)
                    .build(),
            ],
        );
    }

    #[test]
    fn withdraw_sol_vault_fail_with_unauthorized_user() {
        let (mollusk, _, _) = get_mollusk();

        let (system_program, _) = mollusk_svm::program::keyed_account_for_system_program();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let _ = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let attacker = Pubkey::new_from_array([0x03; 32]);
        let attacker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let amount = 2 * LAMPORTS_PER_SOL;
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

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(Vault::LEN);

        let vault_account_data = Vault {
            owner: maker.to_bytes(),
            amount: amount.to_le_bytes(),
            unlock_timestamp: unlock_timestamp.to_le_bytes(),
            mint: None,
            bump: [bump],
        };

        let mut vault_account =
            AccountSharedData::new(lamport_for_rent + amount, Vault::LEN, &PROGRAM_ID);

        vault_account.set_data_from_slice(unsafe { to_bytes::<Vault>(&vault_account_data) });

        let data = vec![WithdrawSolVault::DISCRIMINATOR.clone()];

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(attacker, true),
                AccountMeta::new(vault_address, false),
            ],
        );

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (attacker, attacker_account),
                (vault_address, vault_account.into()),
            ],
            &[
                Check::err(ProgramError::Custom(2)), // Unauthorized
                Check::account(&vault_address).owner(&PROGRAM_ID).build(),
                Check::account(&vault_address)
                    .lamports(amount + lamport_for_rent)
                    .build(),
            ],
        );
    }

    #[test]
    fn init_spl_vault() {
        let (mollusk, mint, mint_account) = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        // Token Program
        let (token_program, token_program_account) =
            mollusk_svm_programs_token::token::keyed_account();

        // Associated Token Program
        let (associated_token_program, associated_token_program_account) =
            mollusk_svm_programs_token::associated_token::keyed_account();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let token_data = TokenAccount {
            mint,
            owner: maker,
            amount: 10_000_000_000,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };

        let user_ata_account = create_account_for_token_account(token_data);
        let user_ata = get_associated_token_address(&maker, &mint);

        let amount = 1_000_000u64;
        let unlock_timestamp = mollusk.sysvars.clock.unix_timestamp + 3600;

        let (vault_address, bump) = Pubkey::find_program_address(
            &[
                Vault::SEED,
                maker.as_ref(),
                mint.as_ref(),
                &amount.to_le_bytes(),
                &unlock_timestamp.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );

        let vault_account = Account::new(0, 0, &system_program);

        let vault_ata = get_associated_token_address(&vault_address, &mint);
        let vault_ata_account = AccountSharedData::new(0, 0, &system_program);

        let ix_data = InitializeSplVaultInstructionData {
            amount,
            unlock_timestamp,
            bump,
        };

        let mut data = vec![InitializeSplVault::DISCRIMINATOR.clone()];
        data.extend_from_slice(unsafe { to_bytes(&ix_data) });

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(maker, true),
                AccountMeta::new(vault_address, false),
                AccountMeta::new(mint, false),
                AccountMeta::new(user_ata, false),
                AccountMeta::new(vault_ata, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (maker, maker_account),
                (vault_address, vault_account),
                (mint, mint_account),
                (user_ata, user_ata_account),
                (vault_ata, vault_ata_account.into()),
                (token_program, token_program_account),
                (associated_token_program, associated_token_program_account),
                (system_program, system_account),
            ],
            &[
                Check::success(),
                Check::account(&vault_address).owner(&PROGRAM_ID).build(),
            ],
        );
    }

    #[test]
    fn withdraw_spl_vault_successfully() {
        let (mut mollusk, mint, mint_account) = get_mollusk();

        let (system_program, system_account) =
            mollusk_svm::program::keyed_account_for_system_program();

        // Token Program
        let (token_program, token_program_account) =
            mollusk_svm_programs_token::token::keyed_account();

        let maker = Pubkey::new_from_array([0x02; 32]);
        let maker_account = Account::new(10 * LAMPORTS_PER_SOL, 0, &system_program);

        let token_data = TokenAccount {
            mint,
            owner: maker,
            amount: 0,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };

        let user_ata_account = create_account_for_token_account(token_data);
        let user_ata = get_associated_token_address(&maker, &mint);

        let amount = 1_000_000u64;
        let unlock_timestamp = mollusk.sysvars.clock.unix_timestamp + 3600;

        let (vault_address, bump) = Pubkey::find_program_address(
            &[
                Vault::SEED,
                maker.as_ref(),
                mint.as_ref(),
                &amount.to_le_bytes(),
                &unlock_timestamp.to_le_bytes(),
            ],
            &PROGRAM_ID,
        );

        let lamport_for_rent = mollusk.sysvars.rent.minimum_balance(Vault::LEN);

        let vault_account_data = Vault {
            owner: maker.to_bytes(),
            amount: amount.to_le_bytes(),
            unlock_timestamp: unlock_timestamp.to_le_bytes(),
            mint: Some(mint.to_bytes()),
            bump: [bump],
        };

        let mut vault_account = AccountSharedData::new(lamport_for_rent, Vault::LEN, &PROGRAM_ID);

        vault_account.set_data_from_slice(unsafe { to_bytes::<Vault>(&vault_account_data) });

        let vault_token_data = TokenAccount {
            mint,
            owner: vault_address,
            amount: amount,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };

        let vault_ata_account = create_account_for_token_account(vault_token_data);
        let vault_ata = get_associated_token_address(&vault_address, &mint);

        let data = vec![WithdrawSplVault::DISCRIMINATOR.clone()];

        let instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &data,
            vec![
                AccountMeta::new(maker, true),
                AccountMeta::new(vault_address, false),
                AccountMeta::new(mint, false),
                AccountMeta::new(user_ata, false),
                AccountMeta::new(vault_ata, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
            ],
        );

        mollusk.sysvars.clock.unix_timestamp = unlock_timestamp + 100;

        let _: mollusk_svm::result::InstructionResult = mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (maker, maker_account),
                (vault_address, vault_account.into()),
                (mint, mint_account),
                (user_ata, user_ata_account),
                (vault_ata, vault_ata_account.into()),
                (token_program, token_program_account),
                (system_program, system_account),
            ],
            &[
                Check::success(),
                // Check::account(&vault_address).owner(&PROGRAM_ID).build(),
            ],
        );
    }
}
