use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("SelfRescue111111111111111111111111111111");

/// A Solana port of the SelfRescueRegistry contract.  Users can register a
/// recovery address and delay, initiate a rescue to start the timer, cancel
/// rescues before the delay elapses and execute rescues after the delay
/// passes.  The owner can designate executors and set the SNRG token mint.
#[program]
pub mod self_rescue_registry {
    use super::*;

    /// Initializes the registry.  Marks the registry itself as an executor.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.token_mint = Pubkey::default();
        registry.executors = vec![ctx.program_id];
        registry.owner = ctx.accounts.owner.key();
        registry.bump = *ctx.bumps.get("registry").unwrap();
        Ok(())
    }

    /// Registers a recovery plan for the caller.  Requires a minimum delay.
    pub fn register_plan(ctx: Context<RegisterPlan>, recovery: Pubkey, delay: i64) -> Result<()> {
        require!(recovery != Pubkey::default(), RegistryError::InvalidRecovery);
        require!(delay >= MINIMUM_RESCUE_DELAY, RegistryError::DelayTooShort);
        let plan = &mut ctx.accounts.plan;
        plan.owner = ctx.accounts.user.key();
        plan.recovery = recovery;
        plan.delay = delay;
        plan.eta = 0;
        Ok(())
    }

    /// Initiates the rescue timer.  Sets the ETA based on the registered
    /// delay.  The caller must have previously registered a plan.
    pub fn initiate_rescue(ctx: Context<InitiateRescue>) -> Result<()> {
        let plan = &mut ctx.accounts.plan;
        require!(plan.recovery != Pubkey::default(), RegistryError::NoPlan);
        let now = Clock::get()?.unix_timestamp;
        plan.eta = now + plan.delay;
        Ok(())
    }

    /// Cancels an initiated rescue by resetting the ETA to zero.
    pub fn cancel_rescue(ctx: Context<CancelRescue>) -> Result<()> {
        let plan = &mut ctx.accounts.plan;
        require!(plan.eta != 0, RegistryError::NoActive);
        plan.eta = 0;
        Ok(())
    }

    /// Executes the rescue by transferring `amount` of tokens from the victim
    /// to the registered recovery address.  Anyone may call this after the
    /// ETA has passed.  Only designated executors are allowed to invoke this
    /// instruction if executors have been set.
    pub fn execute_rescue(ctx: Context<ExecuteRescue>, amount: u64) -> Result<()> {
        require!(amount > 0, RegistryError::InvalidAmount);
        let registry = &ctx.accounts.registry;
        let plan = &mut ctx.accounts.plan;
        let now = Clock::get()?.unix_timestamp;
        require!(plan.eta != 0 && now >= plan.eta, RegistryError::NotMatured);
        // check executor
        let caller = ctx.accounts.caller.key();
        require!(registry.executors.contains(&caller), RegistryError::NotExecutor);
        // reset eta to prevent reentrancy
        plan.eta = 0;
        // transfer tokens
        let cpi_accounts = Transfer {
            from: ctx.accounts.victim_token.to_account_info(),
            to: ctx.accounts.recovery_token.to_account_info(),
            authority: ctx.accounts.victim.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        Ok(())
    }

    /// Adds or removes an executor.  Only the owner may call this.
    pub fn set_executor(ctx: Context<SetExecutor>, exec: Pubkey, enabled: bool) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        if enabled {
            if !registry.executors.contains(&exec) {
                registry.executors.push(exec);
            }
        } else {
            registry.executors.retain(|e| *e != exec);
        }
        Ok(())
    }

    /// Sets the token mint used for rescues.  Only the owner may call this.
    pub fn set_token(ctx: Context<SetToken>, token_mint: Pubkey) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(registry.token_mint == Pubkey::default(), RegistryError::TokenAlreadySet);
        require!(token_mint != Pubkey::default(), RegistryError::InvalidToken);
        registry.token_mint = token_mint;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

pub const MINIMUM_RESCUE_DELAY: i64 = 86_400; // 1 day in seconds

#[account]
pub struct Registry {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub executors: Vec<Pubkey>,
    pub bump: u8,
}

#[account]
pub struct Plan {
    pub owner: Pubkey,
    pub recovery: Pubkey,
    pub delay: i64,
    pub eta: i64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 32 + 32 + (4 + 32 * 5) + 1, seeds = [b"registry"], bump)]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterPlan<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init_if_needed, payer = user, space = 8 + 32 + 32 + 8 + 8, seeds = [b"plan", user.key().as_ref()], bump)]
    pub plan: Account<'info, Plan>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitiateRescue<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"plan", user.key().as_ref()], bump)]
    pub plan: Account<'info, Plan>,
}

#[derive(Accounts)]
pub struct CancelRescue<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"plan", user.key().as_ref()], bump)]
    pub plan: Account<'info, Plan>,
}

#[derive(Accounts)]
pub struct ExecuteRescue<'info> {
    /// The caller that triggers the rescue.  Must be an approved executor.
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut)]
    pub registry: Account<'info, Registry>,
    #[account(mut, seeds = [b"plan", victim.key().as_ref()], bump)]
    pub plan: Account<'info, Plan>,
    #[account(mut)]
    pub victim: Signer<'info>,
    #[account(mut)]
    pub victim_token: Account<'info, TokenAccount>,
    /// CHECK: Recovery account is not owned by the program
    pub recovery: UncheckedAccount<'info>,
    #[account(mut)]
    pub recovery_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetExecutor<'info> {
    #[account(mut, has_one = owner)]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetToken<'info> {
    #[account(mut, has_one = owner)]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
}

#[error_code]
pub enum RegistryError {
    #[msg("Invalid recovery address")] InvalidRecovery,
    #[msg("Delay too short")] DelayTooShort,
    #[msg("No plan registered")]
    NoPlan,
    #[msg("No active rescue")] NoActive,
    #[msg("Rescue not yet matured")] NotMatured,
    #[msg("Caller is not an executor")] NotExecutor,
    #[msg("Invalid amount")] InvalidAmount,
    #[msg("Token mint already set")] TokenAlreadySet,
    #[msg("Invalid token mint")] InvalidToken,
}