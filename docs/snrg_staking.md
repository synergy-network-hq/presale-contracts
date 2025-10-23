# Anchor API

## snrg_staking Program

snrg_staking : solana/programs/snrg_staking

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>) -> Result<()> 
```

#### set_snrg_mint

```rust
fn set_snrg_mint(ctx: Context<SetSnrgMint>, snrg_mint: Pubkey) -> Result<()> 
```

#### fund_contract

```rust
fn fund_contract(ctx: Context<FundContract>, amount: u64) -> Result<()> 
```

#### stake

```rust
fn stake(ctx: Context<Stake>, amount: u64, duration: u64) -> Result<()> 
```

#### withdraw

```rust
fn withdraw(ctx: Context<Withdraw>) -> Result<()> 
```

#### withdraw_early

```rust
fn withdraw_early(ctx: Context<WithdrawEarly>) -> Result<()> 
```