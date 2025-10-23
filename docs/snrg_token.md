# Anchor API

## snrg_token Program

snrg_token : solana/programs/snrg_token

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>, total_supply: u64) -> Result<()> 
```

#### set_endpoints

```rust
fn set_endpoints(ctx: Context<SetEndpoints>, staking: Pubkey, swap: Pubkey, rescue_registry: Pubkey) -> Result<()> 
```

#### transfer_restricted

```rust
fn transfer_restricted(ctx: Context<TransferRestricted>, amount: u64) -> Result<()> 
```