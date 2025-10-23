# Anchor API

## snrg_swap Program

snrg_swap : solana/programs/snrg_swap

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>, snrg_mint: Pubkey) -> Result<()> 
```

#### burn_for_receipt

```rust
fn burn_for_receipt(ctx: Context<BurnForReceipt>, amount: u64) -> Result<()> 
```

#### finalize

```rust
fn finalize(ctx: Context<Finalize>, merkle_root: [u8; 32]) -> Result<()> 
```