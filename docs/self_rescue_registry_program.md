# Anchor API

## self_rescue_registry Program

self_rescue_registry : solana/programs/self_rescue_registry

---

### Instructions:

#### initialize

```rust
fn initialize(ctx: Context<Initialize>) -> Result<()> 
```

#### register_plan

```rust
fn register_plan(ctx: Context<RegisterPlan>, recovery: Pubkey, delay: i64) -> Result<()> 
```

#### initiate_rescue

```rust
fn initiate_rescue(ctx: Context<InitiateRescue>) -> Result<()> 
```

#### cancel_rescue

```rust
fn cancel_rescue(ctx: Context<CancelRescue>) -> Result<()> 
```

#### execute_rescue

```rust
fn execute_rescue(ctx: Context<ExecuteRescue>, amount: u64) -> Result<()> 
```

#### set_executor

```rust
fn set_executor(ctx: Context<SetExecutor>, exec: Pubkey, enabled: bool) -> Result<()> 
```

#### set_token

```rust
fn set_token(ctx: Context<SetToken>, token_mint: Pubkey) -> Result<()> 
```