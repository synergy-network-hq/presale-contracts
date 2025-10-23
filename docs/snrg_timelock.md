# Anchor API

## snrg_timelock Program

snrg_timelock : solana/programs/snrg_timelock

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>, min_delay: i64, multisig: Pubkey) -> Result<()> 
```