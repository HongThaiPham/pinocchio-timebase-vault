# Pinocchio Timebase Vault

A Solana program for creating time-locked vaults that securely hold SOL or SPL tokens until a specified unlock timestamp. Built with the [Pinocchio](https://github.com/anza-xyz/pinocchio) framework for optimized performance and minimal resource usage.

## Features

- **Time-locked Vaults**: Lock SOL or SPL tokens until a specific future timestamp
- **Dual Token Support**: Supports both native SOL and SPL token vaults
- **Secure Architecture**: Program-derived addresses (PDAs) ensure vault security
- **Gas Optimized**: Built with Pinocchio for minimal compute and memory usage
- **Comprehensive Testing**: Full test coverage with Mollusk SVM for reliable operation

## Program ID

```
Ac9JwB8Wc4JB7WwNkVSAY1SESxNmLw5rxuh1okLjQpX
```

## Instructions

### 1. Initialize SOL Vault

Creates a time-locked vault for native SOL tokens.

**Accounts:**

- `signer` (signer, writable): The user creating the vault
- `vault` (writable): The vault PDA account to be created
- `system_program`: The Solana System Program

**Instruction Data:**

- `amount` (u64): Amount of SOL to lock (in lamports)
- `unlock_timestamp` (i64): Unix timestamp when vault can be unlocked
- `bump` (u8): Bump seed for the vault PDA

**Vault PDA Seeds:**

```
["vault", signer_pubkey, amount_bytes, unlock_timestamp_bytes]
```

### 2. Withdraw SOL Vault

Withdraws all SOL from a time-locked vault after the unlock timestamp.

**Accounts:**

- `signer` (signer, writable): The vault owner
- `vault` (writable): The vault account to withdraw from

**Validation:**

- Must be called by the vault owner
- Current timestamp must be >= unlock timestamp
- Vault must contain the expected amount

### 3. Initialize SPL Vault

Creates a time-locked vault for SPL tokens.

**Accounts:**

- `signer` (signer, writable): The user creating the vault
- `vault` (writable): The vault PDA account to be created
- `mint`: The SPL token mint account
- `user_ata` (writable): User's associated token account
- `vault_ata` (writable): Vault's associated token account (created by instruction)
- `token_program`: The SPL Token Program
- `associated_token_program`: The Associated Token Program
- `system_program`: The Solana System Program

**Instruction Data:**

- `amount` (u64): Amount of tokens to lock (in token units)
- `unlock_timestamp` (i64): Unix timestamp when vault can be unlocked
- `bump` (u8): Bump seed for the vault PDA

**Vault PDA Seeds:**

```
["vault", signer_pubkey, mint_pubkey, amount_bytes, unlock_timestamp_bytes]
```

### 4. Withdraw SPL Vault

Withdraws all SPL tokens from a time-locked vault after the unlock timestamp.

**Accounts:**

- `signer` (signer, writable): The vault owner
- `vault` (writable): The vault account to withdraw from
- `mint`: The SPL token mint account
- `user_ata` (writable): User's associated token account
- `vault_ata` (writable): Vault's associated token account
- `token_program`: The SPL Token Program
- `system_program`: The Solana System Program

## Vault State

```rust
pub struct Vault {
    pub owner: Pubkey,           // The vault owner
    pub amount: [u8; 8],         // Amount locked (as bytes)
    pub bump: [u8; 1],           // PDA bump seed
    pub unlock_timestamp: [u8; 8], // Unlock timestamp (as bytes)
    pub mint: Option<Pubkey>,    // Token mint (None for SOL vaults)
}
```

## Error Codes

| Code | Error                           | Description                                |
| ---- | ------------------------------- | ------------------------------------------ |
| 0    | `UnlockTimestampMustBeInFuture` | The unlock timestamp must be in the future |
| 1    | `AmountMustBeGreaterThanZero`   | The amount must be greater than zero       |
| 2    | `Unauthorized`                  | Only the vault owner can withdraw          |
| 3    | `VaultLocking`                  | Cannot withdraw before unlock timestamp    |
| 4    | `InvalidVaultMint`              | Invalid mint address for SPL vault         |

## Development

### Prerequisites

- Rust 1.70+
- Solana CLI tools
- Node.js (for client development)

### Building

```bash
cargo build-sbf
```

### Testing

```bash
cargo test
```

The test suite uses [Mollusk SVM](https://github.com/anza-xyz/mollusk) for comprehensive program testing, including:

- SOL vault creation and withdrawal
- SPL token vault creation and withdrawal
- Error condition testing (unauthorized access, early withdrawal)
- Account validation and PDA verification

### Deploying

```bash
solana program deploy target/deploy/pinocchio_timebase_vault.so
```

## Security Considerations

- **Time Validation**: Unlock timestamps must be in the future when creating vaults
- **Owner Verification**: Only vault owners can withdraw funds
- **PDA Security**: Vault addresses are deterministically generated using program-derived addresses
- **Amount Validation**: Vault amounts must be greater than zero
- **Account Validation**: All account ownership and writability requirements are enforced

## Dependencies

- **Pinocchio Framework**: Core Solana program framework
- **Pinocchio Modules**: System, Token, and Associated Token Account programs
- **Mollusk SVM**: Testing framework for Solana programs

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Author

Leo Pham - [GitHub](https://github.com/HongThaiPham)
