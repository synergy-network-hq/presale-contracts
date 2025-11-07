use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};


/// Enhanced Solana port of the SelfRescueRegistry contract matching the
/// Solidity version. This is a SELF-RESCUE mechanism requiring user opt-in
/// via token approval. Includes cooldowns, max rescue amounts, and pausability.
#[program]
pub mod self_rescue_registry {
    use super::*;

    /// Initializes the registry.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.token_mint = Pubkey::default();
        registry.executors = vec![ctx.program_id];
        registry.owner = ctx.accounts.owner.key();
        registry.max_rescue_amount = 0;
        registry.paused = false;
        registry.bump = *ctx.bumps.get("registry").unwrap();
        
        emit!(RegistryInitialized {
            owner: registry.owner,
        });
        
        Ok(())
    }

    /// Registers a recovery plan for the caller.
    pub fn register_plan(ctx: Context<RegisterPlan>, recovery: Pubkey, delay: i64) -> Result<()> {
        require!(recovery != Pubkey::default(), RegistryError::InvalidRecovery);
        require!(recovery != ctx.accounts.user.key(), RegistryError::InvalidRecovery);
        require!(delay >= MINIMUM_RESCUE_DELAY, RegistryError::DelayTooShort);
        require!(delay <= MAXIMUM_RESCUE_DELAY, RegistryError::DelayTooLong);
        
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);
        
        let plan = &mut ctx.accounts.plan;
        plan.owner = ctx.accounts.user.key();
        plan.recovery = recovery;
        plan.delay = delay;
        plan.eta = 0;
        plan.last_rescue_time = 0;
        
        emit!(PlanRegistered {
            user: ctx.accounts.user.key(),
            recovery,
            delay,
        });
        
        Ok(())
    }

    /// Initiates the rescue timer.
    pub fn initiate_rescue(ctx: Context<InitiateRescue>) -> Result<()> {
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);
        
        let plan = &mut ctx.accounts.plan;
        require!(plan.recovery != Pubkey::default(), RegistryError::NoPlan);
        require!(plan.eta == 0, RegistryError::RescueAlreadyActive);
        
        let now = Clock::get()?.unix_timestamp;
        
        // Check cooldown period
        require!(
            now >= plan.last_rescue_time + RESCUE_COOLDOWN,
            RegistryError::CooldownActive
        );
        
        plan.last_rescue_time = now;
        plan.eta = now
            .checked_add(plan.delay)
            .ok_or(RegistryError::MathOverflow)?;
        
        emit!(RescueInitiated {
            user: ctx.accounts.user.key(),
            eta: plan.eta,
        });
        
        Ok(())
    }

    /// Cancels an initiated rescue (FIX L-04: resets cooldown).
    pub fn cancel_rescue(ctx: Context<CancelRescue>) -> Result<()> {
        let plan = &mut ctx.accounts.plan;
        require!(plan.eta != 0, RegistryError::NoActive);
        
        plan.eta = 0;
        // FIX L-04: Reset cooldown on cancel to allow re-initiation
        plan.last_rescue_time = 0;
        
        emit!(RescueCancelled {
            user: ctx.accounts.user.key(),
        });
        
        Ok(())
    }

    /// Executes the rescue by transferring tokens from victim to recovery address.
    /// FIX M-01: This requires the victim to have approved this program for the
    /// rescue amount. This is a SELF-RESCUE mechanism, NOT forced recovery.
    pub fn execute_rescue(ctx: Context<ExecuteRescue>, amount: u64) -> Result<()> {
        require!(amount > 0, RegistryError::InvalidAmount);
        
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);
        
        let plan = &mut ctx.accounts.plan;
        let now = Clock::get()?.unix_timestamp;
        
        require!(plan.eta != 0 && now >= plan.eta, RegistryError::NotMatured);
        require!(plan.recovery != Pubkey::default(), RegistryError::NoPlan);
        
        // Check executor authorization
        let caller = ctx.accounts.caller.key();
        let is_authorized = registry.executors.contains(&caller) 
            || caller == ctx.accounts.victim.key() 
            || caller == plan.recovery;
        require!(is_authorized, RegistryError::NotExecutor);
        
        // Check max rescue amount if set
        if registry.max_rescue_amount > 0 {
            require!(amount <= registry.max_rescue_amount, RegistryError::ExceedsMaxRescue);
        }
        
        // Validate token accounts
        require!(ctx.accounts.victim_token.mint == registry.token_mint, RegistryError::InvalidMint);
        require!(ctx.accounts.recovery_token.mint == registry.token_mint, RegistryError::InvalidMint);
        require!(ctx.accounts.victim_token.owner == ctx.accounts.victim.key(), RegistryError::InvalidOwner);
        
        // Check sufficient balance
        require!(ctx.accounts.victim_token.amount >= amount, RegistryError::InsufficientBalance);
        
        // FIX M-01: Check if victim has approved this program for the amount
        // This is the key requirement for self-rescue - user must grant allowance
        let victim_delegate = ctx.accounts.victim_token.delegate;
        let victim_delegated_amount = ctx.accounts.victim_token.delegated_amount;
        
        require!(
            victim_delegate == COption::Some(ctx.program_id) && victim_delegated_amount >= amount,
            RegistryError::InsufficientAllowance
        );
        
        // Reset eta before external call to prevent reentrancy
        plan.eta = 0;
        
        // Transfer tokens using delegated authority
        let cpi_accounts = Transfer {
            from: ctx.accounts.victim_token.to_account_info(),
            to: ctx.accounts.recovery_token.to_account_info(),
            authority: ctx.accounts.registry_signer.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"registry", &[registry.bump]]];
        let signer = &[&seeds[..]];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ), amount)?;
        
        emit!(RescueExecuted {
            victim: ctx.accounts.victim.key(),
            recovery: plan.recovery,
            amount,
        });
        
        Ok(())
    }

    /// Adds or removes an executor.
    pub fn set_executor(ctx: Context<SetExecutor>, exec: Pubkey, enabled: bool) -> Result<()> {
        require!(exec != Pubkey::default(), RegistryError::InvalidExecutor);
        
        let registry = &mut ctx.accounts.registry;
        if enabled {
            require!(!registry.executors.contains(&exec), RegistryError::ExecutorAlreadyExists);
            require!(registry.executors.len() < MAX_EXECUTORS, RegistryError::TooManyExecutors);
            registry.executors.push(exec);
        } else {
            require!(registry.executors.contains(&exec), RegistryError::ExecutorNotFound);
            registry.executors.retain(|e| *e != exec);
        }
        
        emit!(ExecutorSet {
            executor: exec,
            enabled,
        });
        
        Ok(())
    }

    /// Sets the token mint used for rescues.
    pub fn set_token(ctx: Context<SetToken>, token_mint: Pubkey) -> Result<()> {
        require!(token_mint != Pubkey::default(), RegistryError::InvalidToken);
        
        let registry = &mut ctx.accounts.registry;
        require!(registry.token_mint == Pubkey::default(), RegistryError::TokenAlreadySet);
        registry.token_mint = token_mint;
        
        emit!(TokenSet {
            token_mint,
        });
        
        Ok(())
    }

    /// Sets the maximum rescue amount.
    pub fn set_max_rescue_amount(ctx: Context<SetToken>, max_amount: u64) -> Result<()> {
        require!(max_amount > 0, RegistryError::InvalidAmount);
        
        let registry = &mut ctx.accounts.registry;
        registry.max_rescue_amount = max_amount;
        
        emit!(MaxRescueAmountSet {
            amount: max_amount,
        });
        
        Ok(())
    }

    /// Pause the registry.
    pub fn pause(ctx: Context<SetToken>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(!registry.paused, RegistryError::AlreadyPaused);
        registry.paused = true;
        
        emit!(Paused {});
        Ok(())
    }

    /// Unpause the registry.
    pub fn unpause(ctx: Context<SetToken>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(registry.paused, RegistryError::NotPaused);
        registry.paused = false;
        
        emit!(Unpaused {});
        Ok(())
    }

    /// View: Check if a rescue can be executed.
    pub fn can_execute_rescue(ctx: Context<ViewPlan>) -> Result<bool> {
        let plan = &ctx.accounts.plan;
        let now = Clock::get()?.unix_timestamp;
        Ok(plan.eta != 0 && now >= plan.eta)
    }

    /// View: Check if caller is an executor.
    pub fn is_rescue_executor(ctx: Context<ViewRegistry>, caller: Pubkey) -> Result<bool> {
        let registry = &ctx.accounts.registry;
        Ok(registry.executors.contains(&caller))
    }
}

// -----------------------------------------------------------------------------
// State definitions

pub const MINIMUM_RESCUE_DELAY: i64 = 7 * 86_400; // 7 days
pub const MAXIMUM_RESCUE_DELAY: i64 = 365 * 86_400; // 365 days
pub const RESCUE_COOLDOWN: i64 = 90 * 86_400; // 90 days
pub const MAX_EXECUTORS: usize = 10;

#[account]
pub struct Registry {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub max_rescue_amount: u64,
    pub paused: bool,
    pub executors: Vec<Pubkey>,
    pub bump: u8,
}

#[account]
pub struct Plan {
    pub owner: Pubkey,
    pub recovery: Pubkey,
    pub delay: i64,
    pub eta: i64,
    pub last_rescue_time: i64,
}

// -----------------------------------------------------------------------------
// Account contexts

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8 + 1 + (4 + 32 * 10) + 1,
        seeds = [b"registry"],
        bump
    )]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterPlan<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub registry: Account<'info, Registry>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 32 + 8 + 8 + 8,
        seeds = [b"plan", user.key().as_ref()],
        bump
    )]
    pub plan: Account<'info, Plan>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitiateRescue<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub registry: Account<'info, Registry>,
    #[account(
        mut,
        seeds = [b"plan", user.key().as_ref()],
        bump,
        constraint = plan.owner == user.key()
    )]
    pub plan: Account<'info, Plan>,
}

#[derive(Accounts)]
pub struct CancelRescue<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"plan", user.key().as_ref()],
        bump,
        constraint = plan.owner == user.key()
    )]
    pub plan: Account<'info, Plan>,
}

#[derive(Accounts)]
pub struct ExecuteRescue<'info> {
    /// The caller that triggers the rescue (must be executor, victim, or recovery address)
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut)]
    pub registry: Account<'info, Registry>,
    #[account(
        mut,
        seeds = [b"plan", victim.key().as_ref()],
        bump,
        constraint = plan.owner == victim.key()
    )]
    pub plan: Account<'info, Plan>,
    /// CHECK: Victim account (does not need to sign for self-rescue)
    pub victim: UncheckedAccount<'info>,
    #[account(mut)]
    pub victim_token: Account<'info, TokenAccount>,
    /// CHECK: Recovery account
    pub recovery: UncheckedAccount<'info>,
    #[account(mut)]
    pub recovery_token: Account<'info, TokenAccount>,
    /// CHECK: Registry PDA signer
    #[account(
        seeds = [b"registry"],
        bump = registry.bump
    )]
    pub registry_signer: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetExecutor<'info> {
    #[account(
        mut,
        has_one = owner,
        constraint = registry.owner == owner.key()
    )]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetToken<'info> {
    #[account(
        mut,
        has_one = owner,
        constraint = registry.owner == owner.key()
    )]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ViewRegistry<'info> {
    pub registry: Account<'info, Registry>,
}

#[derive(Accounts)]
pub struct ViewPlan<'info> {
    pub plan: Account<'info, Plan>,
}

// -----------------------------------------------------------------------------
// Events

#[event]
pub struct RegistryInitialized {
    pub owner: Pubkey,
}

#[event]
pub struct PlanRegistered {
    pub user: Pubkey,
    pub recovery: Pubkey,
    pub delay: i64,
}

#[event]
pub struct RescueInitiated {
    pub user: Pubkey,
    pub eta: i64,
}

#[event]
pub struct RescueCancelled {
    pub user: Pubkey,
}

#[event]
pub struct RescueExecuted {
    pub victim: Pubkey,
    pub recovery: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ExecutorSet {
    pub executor: Pubkey,
    pub enabled: bool,
}

#[event]
pub struct TokenSet {
    pub token_mint: Pubkey,
}

#[event]
pub struct MaxRescueAmountSet {
    pub amount: u64,
}

#[event]
pub struct Paused {}

#[event]
pub struct Unpaused {}

// -----------------------------------------------------------------------------
// Errors

#[error_code]
pub enum RegistryError {
    #[msg("Invalid recovery address")]
    InvalidRecovery,
    #[msg("Delay too short")]
    DelayTooShort,
    #[msg("Delay too long")]
    DelayTooLong,
    #[msg("No plan registered")]
    NoPlan,
    #[msg("Rescue already active")]
    RescueAlreadyActive,
    #[msg("Cooldown active")]
    CooldownActive,
    #[msg("No active rescue")]
    NoActive,
    #[msg("Rescue not yet matured")]
    NotMatured,
    #[msg("Caller is not an executor")]
    NotExecutor,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Token mint already set")]
    TokenAlreadySet,
    #[msg("Invalid token mint")]
    InvalidToken,
    #[msg("Invalid executor")]
    InvalidExecutor,
    #[msg("Executor already exists")]
    ExecutorAlreadyExists,
    #[msg("Too many executors")]
    TooManyExecutors,
    #[msg("Executor not found")]
    ExecutorNotFound,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Insufficient allowance")]
    InsufficientAllowance,
    #[msg("Exceeds max rescue amount")]
    ExceedsMaxRescue,
    #[msg("Contract is paused")]
    Paused,
    #[msg("Contract is not paused")]
    NotPaused,
    #[msg("Contract already paused")]
    AlreadyPaused,
}