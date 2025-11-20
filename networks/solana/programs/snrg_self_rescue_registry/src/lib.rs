use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, TransferChecked};

declare_id!("YourProgramIDHere11111111111111111111111111111");

pub const MINIMUM_RESCUE_DELAY: i64 = 7 * 86_400; // 7 days
pub const MAXIMUM_RESCUE_DELAY: i64 = 365 * 86_400; // 1 year max
pub const RESCUE_COOLDOWN: i64 = 90 * 86_400; // 90 days

#[program]
pub mod self_rescue_registry {
    use super::*;

    /// One-time initialization (used with upgradeable proxy or factory)
    pub fn initialize(ctx: Context<Initialize>, token_mint: Pubkey) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(!registry.initialized, RegistryError::AlreadyInitialized);

        registry.owner = ctx.accounts.owner.key();
        registry.token_mint = token_mint;
        registry.max_rescue_amount = 0;
        registry.paused = false;
        registry.bump = *ctx.bumps.get("registry").unwrap();

        // Contract itself is always an executor (for CPI safety)
        registry.executors.insert(ctx.program_id);

        registry.initialized = true;

        emit!(Initialized {
            initializer: ctx.accounts.owner.key()
        });

        Ok(())
    }

    /// User registers their self-rescue plan
    pub fn register_plan(ctx: Context<RegisterPlan>, recovery: Pubkey, delay: i64) -> Result<()> {
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);
        require_keys_neq!(recovery, Pubkey::default(), RegistryError::ZeroAddress);
        require_keys_neq!(recovery, ctx.accounts.user.key(), RegistryError::InvalidRecovery);
        require_gte!(delay, MINIMUM_RESCUE_DELAY, RegistryError::DelayTooShort);
        require_lte!(delay, MAXIMUM_RESCUE_DELAY, RegistryError::DelayTooLong);

        let plan = &mut ctx.accounts.plan;
        plan.owner = ctx.accounts.user.key();
        plan.recovery = recovery;
        plan.delay = delay;
        plan.eta = 0;

        emit!(PlanRegistered {
            user: ctx.accounts.user.key(),
            recovery,
            delay,
        });

        Ok(())
    }

    /// User starts the time-delayed rescue process
    pub fn initiate_rescue(ctx: Context<InitiateRescue>) -> Result<()> {
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);

        let plan = &mut ctx.accounts.plan;
        require_neq!(plan.recovery, Pubkey::default(), RegistryError::NoPlanRegistered);
        require_eq!(plan.eta, 0, RegistryError::RescueAlreadyActive);

        let now = Clock::get()?.unix_timestamp;
        require!(
            now >= plan.last_rescue_time + RESCUE_COOLDOWN,
            RegistryError::CooldownActive
        );

        plan.last_rescue_time = now;
        plan.eta = now.checked_add(plan.delay).ok_or(RegistryError::MathOverflow)?;

        emit!(RescueInitiated {
            user: ctx.accounts.user.key(),
            eta: plan.eta
        });

        Ok(())
    }

    /// User or anyone can cancel an active rescue
    pub fn cancel_rescue(ctx: Context<CancelRescue>) -> Result<()> {
        let plan = &mut ctx.accounts.plan;
        require_neq!(plan.eta, 0, RegistryError::NoActiveRescue);

        plan.eta = 0;
        plan.last_rescue_time = 0; // FIX L-04 equivalent

        emit!(RescueCanceled {
            user: ctx.accounts.user.key()
        });

        Ok(())
    }

    /// Anyone (executor, victim, or recovery addr) can execute after delay
    pub fn execute_rescue(ctx: Context<ExecuteRescue>, amount: u64) -> Result<()> {
        let registry = &ctx.accounts.registry;
        require!(!registry.paused, RegistryError::Paused);
        require_gt!(amount, 0, RegistryError::ZeroAmount);

        let plan = &mut ctx.accounts.plan;
        let now = Clock::get()?.unix_timestamp;

        require_neq!(plan.eta, 0, RegistryError::NoActiveRescue);
        require!(now >= plan.eta, RegistryError::NotMatured);

        // Authorization: executor OR victim OR recovery address
        let caller = ctx.accounts.caller.key();
        let is_executor = registry.executors.contains(&caller);
        let is_victim = caller == plan.owner;
        let is_recovery = caller == plan.recovery;
        require!(is_executor || is_victim || is_recovery, RegistryError::UnauthorizedCaller);

        if registry.max_rescue_amount > 0 {
            require!(amount <= registry.max_rescue_amount, RegistryError::ExceedsMaxRescue);
        }

        let victim_token = &ctx.accounts.victim_token;
        require_eq!(victim_token.mint, registry.token_mint, RegistryError::InvalidMint);
        require_eq!(victim_token.owner, plan.owner, RegistryError::InvalidOwner);
        require_gte!(victim_token.amount, amount, RegistryError::InsufficientBalance);

        // Critical FIX M-01: Must have delegated (approved) this program
        let delegate = victim_token.delegate.ok_or(RegistryError::InsufficientAllowance)?;
        let delegated_amount = victim_token.delegated_amount;
        require_eq!(delegate, ctx.accounts.registry_signer.key(), RegistryError::InsufficientAllowance);
        require_gte!(delegated_amount, amount, RegistryError::InsufficientAllowance);

        // Clear ETA before external interaction (reentrancy protection)
        plan.eta = 0;

        // Transfer using program as delegate
        let seeds = &[b"registry".as_ref(), &[registry.bump]];
        let signer_seeds = &[&seeds[..]];

        token::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.victim_token.to_account_info(),
                    to: ctx.accounts.recovery_token.to_account_info(),
                    authority: ctx.accounts.registry_signer.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            ctx.accounts.mint.decimals,
        )?;

        emit!(RescueExecuted {
            user: plan.owner,
            recovery: plan.recovery,
            amount,
        });

        Ok(())
    }

    // Owner-only admin functions below

    pub fn set_executor(ctx: Context<Admin>, exec: Pubkey, enabled: bool) -> Result<()> {
        require_keys_neq!(exec, Pubkey::default(), RegistryError::ZeroAddress);
        let executors = &mut ctx.accounts.registry.executors;

        if enabled {
            require!(!executors.contains(&exec), RegistryError::ExecutorAlreadySet);
            executors.insert(exec);
        } else {
            executors.remove(&exec);
        }

        emit!(ExecutorSet { executor: exec, enabled });
        Ok(())
    }

    pub fn set_max_rescue_amount(ctx: Context<Admin>, amount: u64) -> Result<()> {
        require_gt!(amount, 0, RegistryError::ZeroAmount);
        ctx.accounts.registry.max_rescue_amount = amount;
        emit!(MaxRescueAmountSet { amount });
        Ok(())
    }

    pub fn pause(ctx: Context<Admin>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(!registry.paused, RegistryError::AlreadyPaused);
        registry.paused = true;
        emit!(Paused {});
        Ok(())
    }

    pub fn unpause(ctx: Context<Admin>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        require!(registry.paused, RegistryError::NotPaused);
        registry.paused = false;
        emit!(Unpaused {});
        Ok(())
    }

    // View functions
    pub fn can_execute_rescue(ctx: Context<ViewPlan>) -> Result<bool> {
        let now = Clock::get()?.unix_timestamp;
        Ok(ctx.accounts.plan.eta != 0 && now >= ctx.accounts.plan.eta)
    }

    pub fn is_rescue_executor(ctx: Context<ViewRegistry>, caller: Pubkey) -> Result<bool> {
        Ok(ctx.accounts.registry.executors.contains(&caller))
    }
}

// =============================================================================
// Accounts & Events
// =============================================================================

#[account]
pub struct Registry {
    pub owner: Pubkey,
    pub token_mint: Pubkey,
    pub max_rescue_amount: u64,
    pub paused: bool,
    pub executors: BTreeSet<Pubkey>, // efficient contains/remove
    pub initialized: bool,
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

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 8 + 1 + 200 + 1 + 1, // generous for BTreeSet
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
    #[account(signer)]
    pub caller: Signer<'info>,
    pub registry: Account<'info, Registry>,
    #[account(
        mut,
        seeds = [b"plan", victim.key().as_ref()],
        bump,
        constraint = plan.owner == victim.key()
    )]
    pub plan: Account<'info, Plan>,
    /// CHECK: victim wallet
    pub victim: UncheckedAccount<'info>,
    #[account(mut)]
    pub victim_token: Account<'info, TokenAccount>,
    /// CHECK: recovery wallet
    pub recovery: UncheckedAccount<'info>,
    #[account(mut)]
    pub recovery_token: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        seeds = [b"registry"],
        bump = registry.bump,
    )]
    /// CHECK: PDA signer
    pub registry_signer: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Admin<'info> {
    #[account(
        mut,
        has_one = owner @ RegistryError::Unauthorized,
    )]
    pub registry: Account<'info, Registry>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ViewPlan<'info> {
    pub plan: Account<'info, Plan>,
}

#[derive(Accounts)]
pub struct ViewRegistry<'info> {
    pub registry: Account<'info, Registry>,
}

// =============================================================================
// Events (exact match to Solidity)
// =============================================================================

#[event]
pub struct Initialized { pub initializer: Pubkey }

#[event]
pub struct PlanRegistered { pub user: Pubkey, pub recovery: Pubkey, pub delay: i64 }

#[event]
pub struct RescueInitiated { pub user: Pubkey, pub eta: i64 }

#[event]
pub struct RescueCanceled { pub user: Pubkey }

#[event]
pub struct RescueExecuted { pub user: Pubkey, pub recovery: Pubkey, pub amount: u64 }

#[event]
pub struct ExecutorSet { pub executor: Pubkey, pub enabled: bool }

#[event]
pub struct MaxRescueAmountSet { pub amount: u64 }

#[event]
pub struct Paused {}

#[event]
pub struct Unpaused {}

// =============================================================================
// Errors (exact match to Solidity)
// =============================================================================

#[error_code]
pub enum RegistryError {
    #[msg("Zero address")]
    ZeroAddress,
    #[msg("Invalid recovery address")]
    InvalidRecovery,
    #[msg("Delay too short")]
    DelayTooShort,
    #[msg("Delay too long")]
    DelayTooLong,
    #[msg("No plan registered")]
    NoPlanRegistered,
    #[msg("Rescue already active")]
    RescueAlreadyActive,
    #[msg("Cooldown active")]
    CooldownActive,
    #[msg("No active rescue")]
    NoActiveRescue,
    #[msg("Rescue not matured")]
    NotMatured,
    #[msg("Unauthorized caller")]
    UnauthorizedCaller,
    #[msg("Exceeds max rescue amount")]
    ExceedsMaxRescue,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Insufficient allowance")]
    InsufficientAllowance,
    #[msg("Zero amount")]
    ZeroAmount,
    #[msg("Already initialized")]
    AlreadyInitialized,
    #[msg("Paused")]
    Paused,
    #[msg("Not paused")]
    NotPaused,
    #[msg("Already paused")]
    AlreadyPaused,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Unauthorized")]
    Unauthorized,
}