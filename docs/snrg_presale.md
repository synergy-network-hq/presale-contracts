# Anchor API

## snrg_presale Program

snrg_presale : solana/programs/snrg_presale

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>, signer: Pubkey) -> Result<()>
```

#### set_signer

```rust
fn set_signer(ctx: Context<SetSigner>, signer: Pubkey) -> Result<()>
```

#### set_open

```rust
fn set_open(ctx: Context<SetOpen>, open: bool) -> Result<()>
```

#### add_supported_token

```rust
fn add_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()>
```

#### remove_supported_token

```rust
fn remove_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()>
```

#### buy_with_native

```rust
fn buy_with_native(ctx: Context<BuyWithNative>, snrg_amount: u64, nonce: u64, signature: Vec<u8>) -> Result<()>
```

#### buy_with_token

```rust
fn buy_with_token(ctx: Context<BuyWithToken>, payment_amount: u64, snrg_amount: u64, nonce: u64, signature: Vec<u8>) -> Result<()>
```