use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

/// Enhanced Solana implementation of the SNRG staking contract matching the
/// Solidity version. Includes reserve tracking, emergency withdrawals, and
/// pausability for enhanced security and monitoring.
#[program]
pub mod snrg_staking {
    use super::*;

    /// Initializes the staking state with treasury and reward rates.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        staking.treasury = ctx.accounts.treasury.key();
        staking.snrg_mint = Pubkey::default();
        staking.is_funded = false;
        staking.paused = false;
        staking.reward_reserve = 0;
        staking.promised_rewards = 0;
        staking.reward_rates = vec![
            RewardRate { duration: 30, bps: 125 },
            RewardRate { duration: 60, bps: 250 },
            RewardRate { duration: 90, bps: 375 },
            RewardRate { duration: 180, bps: 500 },
        ];
        staking.bump = *ctx.bumps.get("staking").unwrap();
        
        emit!(StakingInitialized {
            treasury: staking.treasury,
        });
        
        Ok(())
    }

    /// Sets the SNRG mint (can only be called once).
    pub fn set_snrg_mint(ctx: Context<SetSnrgMint>, snrg_mint: Pubkey) -> Result<()> {
        require!(snrg_mint != Pubkey::default(), StakingError::InvalidMint);
        
        let staking = &mut ctx.accounts.staking;
        require!(staking.snrg_mint == Pubkey::default(), StakingError::SnrgAlreadySet);
        staking.snrg_mint = snrg_mint;
        
        emit!(SnrgMintSet { snrg_mint });
        Ok(())
    }

    /// Funds the staking contract (can only be called once).
    pub fn fund_contract(ctx: Context<FundContract>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        
        let staking = &mut ctx.accounts.staking;
        require!(!staking.is_funded, StakingError::AlreadyFunded);
        require!(staking.snrg_mint != Pubkey::default(), StakingError::SnrgNotSet);
        
        // Validate token accounts
        require!(ctx.accounts.treasury_snrgtoken.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.staking_vault.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.treasury_snrgtoken.amount >= amount, StakingError::InsufficientBalance);
        
        staking.is_funded = true;
        staking.reward_reserve = amount;
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_snrgtoken.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.treasury_authority.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        
        emit!(ContractFunded { amount });
        Ok(())
    }

    /// Top up reward reserves (FIX H-03).
    pub fn top_up_reserves(ctx: Context<FundContract>, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        
        let staking = &mut ctx.accounts.staking;
        require!(staking.snrg_mint != Pubkey::default(), StakingError::SnrgNotSet);
        
        // Validate token accounts
        require!(ctx.accounts.treasury_snrgtoken.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.staking_vault.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.treasury_snrgtoken.amount >= amount, StakingError::InsufficientBalance);
        
        staking.reward_reserve = staking.reward_reserve
            .checked_add(amount)
            .ok_or(StakingError::MathOverflow)?;
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_snrgtoken.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.treasury_authority.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        
        emit!(ReserveToppedUp { amount });
        Ok(())
    }

    /// Stakes SNRG for a fixed duration with reserve validation.
    pub fn stake(ctx: Context<Stake>, amount: u64, duration: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        require!(duration > 0, StakingError::InvalidDuration);
        
        let staking = &mut ctx.accounts.staking;
        require!(staking.is_funded, StakingError::NotFunded);
        require!(!staking.paused, StakingError::Paused);
        require!(staking.snrg_mint != Pubkey::default(), StakingError::SnrgNotSet);
        
        // Validate token accounts
        require!(ctx.accounts.user_snrgtoken.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.staking_vault.mint == staking.snrg_mint, StakingError::InvalidMint);
        require!(ctx.accounts.user_snrgtoken.owner == ctx.accounts.user.key(), StakingError::InvalidOwner);
        require!(ctx.accounts.user_snrgtoken.amount >= amount, StakingError::InsufficientBalance);
        
        // Look up reward basis points
        let mut reward_bps = None;
        for rate in staking.reward_rates.iter() {
            if rate.duration == duration {
                reward_bps = Some(rate.bps);
                break;
            }
        }
        require!(reward_bps.is_some(), StakingError::InvalidDuration);
        let reward_bps = reward_bps.unwrap();
        
        // Compute reward with overflow protection
        let reward = amount
            .checked_mul(reward_bps as u64)
            .ok_or(StakingError::MathOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(StakingError::MathOverflow)?;
        
        // FIX H-03: Check if we have sufficient reserves for this reward
        let new_promised = staking.promised_rewards
            .checked_add(reward)
            .ok_or(StakingError::MathOverflow)?;
        require!(staking.reward_reserve >= new_promised, StakingError::InsufficientReserves);
        
        // Transfer tokens from user to staking vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_snrgtoken.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        
        let current_time = Clock::get()?.unix_timestamp;
        let end_time = current_time
            .checked_add(duration as i64 * SECONDS_PER_DAY)
            .ok_or(StakingError::MathOverflow)?;
        
        // Update state
        staking.promised_rewards = new_promised;
        
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.owner = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.reward = reward;
        stake_account.end_time = end_time;
        stake_account.withdrawn = false;
        
        emit!(Staked {
            user: ctx.accounts.user.key(),
            amount,
            reward,
            end_time,
        });
        
        Ok(())
    }

    /// Withdraws a matured stake with principal + reward.
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        require!(!stake_account.withdrawn, StakingError::AlreadyWithdrawn);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time >= stake_account.end_time, StakingError::NotMatured);
        
        // Update state before external call (CEI pattern)
        stake_account.withdrawn = true;
        
        let total = stake_account
            .amount
            .checked_add(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        
        // Validate sufficient balance
        require!(ctx.accounts.staking_vault.amount >= total, StakingError::InsufficientBalance);
        
        // FIX H-03: Decrease promised rewards AND reward reserve when rewards are paid
        let staking = &mut ctx.accounts.staking;
        staking.promised_rewards = staking.promised_rewards
            .checked_sub(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        staking.reward_reserve = staking.reward_reserve
            .checked_sub(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        
        // Transfer tokens from staking vault to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"staking", ctx.accounts.staking.treasury.as_ref(), &[ctx.accounts.staking.bump]]];
        let signer = &seeds[..];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ), total)?;
        
        emit!(Withdrawn {
            user: ctx.accounts.user.key(),
            amount: stake_account.amount,
            reward: stake_account.reward,
        });
        
        Ok(())
    }

    /// Early withdrawal with 5% fee (rewards forfeited).
    pub fn withdraw_early(ctx: Context<WithdrawEarly>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        require!(!stake_account.withdrawn, StakingError::AlreadyWithdrawn);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time < stake_account.end_time, StakingError::AlreadyMatured);
        
        // Update state before external calls (CEI pattern)
        stake_account.withdrawn = true;
        
        // Compute fee = amount * 5% (500 bps) / 10000
        let fee = stake_account
            .amount
            .checked_mul(EARLY_WITHDRAWAL_FEE_BPS as u64)
            .ok_or(StakingError::MathOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(StakingError::MathOverflow)?;
        
        let return_amount = stake_account
            .amount
            .checked_sub(fee)
            .ok_or(StakingError::MathOverflow)?;
        
        // Validate sufficient balance
        require!(ctx.accounts.staking_vault.amount >= stake_account.amount, StakingError::InsufficientBalance);
        
        // FIX H-03: Update promised rewards (early withdrawal forfeits rewards)
        // Do NOT decrease reward_reserve since rewards weren't paid out
        let staking = &mut ctx.accounts.staking;
        staking.promised_rewards = staking.promised_rewards
            .checked_sub(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        
        let seeds: &[&[&[u8]]] = &[&[b"staking", ctx.accounts.staking.treasury.as_ref(), &[ctx.accounts.staking.bump]]];
        let signer = &seeds[..];
        
        // Transfer fee to treasury
        let cpi_accounts_fee = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.treasury_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_fee,
            signer,
        ), fee)?;
        
        // Transfer remainder to user
        let cpi_accounts_return = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_return,
            signer,
        ), return_amount)?;
        
        emit!(WithdrawnEarly {
            user: ctx.accounts.user.key(),
            amount: return_amount,
            fee,
        });
        
        Ok(())
    }

    /// Emergency withdrawal with 10% fee (rewards forfeited).
    pub fn emergency_withdraw(ctx: Context<WithdrawEarly>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        require!(!stake_account.withdrawn, StakingError::AlreadyWithdrawn);
        
        // Update state before external calls (CEI pattern)
        stake_account.withdrawn = true;
        
        // Compute fee = amount * 10% (1000 bps) / 10000
        let fee = stake_account
            .amount
            .checked_mul(EMERGENCY_FEE_BPS as u64)
            .ok_or(StakingError::MathOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(StakingError::MathOverflow)?;
        
        let return_amount = stake_account
            .amount
            .checked_sub(fee)
            .ok_or(StakingError::MathOverflow)?;
        
        // Validate sufficient balance
        require!(ctx.accounts.staking_vault.amount >= stake_account.amount, StakingError::InsufficientBalance);
        
        // FIX H-03: Update promised rewards (emergency withdrawal forfeits rewards)
        // Do NOT decrease reward_reserve since rewards weren't paid out
        let staking = &mut ctx.accounts.staking;
        staking.promised_rewards = staking.promised_rewards
            .checked_sub(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        
        let seeds: &[&[&[u8]]] = &[&[b"staking", ctx.accounts.staking.treasury.as_ref(), &[ctx.accounts.staking.bump]]];
        let signer = &seeds[..];
        
        // Transfer fee to treasury
        let cpi_accounts_fee = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.treasury_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_fee,
            signer,
        ), fee)?;
        
        // Transfer remainder to user
        let cpi_accounts_return = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_return,
            signer,
        ), return_amount)?;
        
        emit!(EmergencyWithdrawal {
            user: ctx.accounts.user.key(),
            amount: return_amount,
            fee,
        });
        
        Ok(())
    }

    /// Pause staking operations.
    pub fn pause(ctx: Context<AdminAction>) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        require!(!staking.paused, StakingError::AlreadyPaused);
        staking.paused = true;
        
        emit!(Paused {});
        Ok(())
    }

    /// Unpause staking operations.
    pub fn unpause(ctx: Context<AdminAction>) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        require!(staking.paused, StakingError::NotPaused);
        staking.paused = false;
        
        emit!(Unpaused {});
        Ok(())
    }

    /// View: Check if contract has sufficient reserves for all promised rewards.
    pub fn is_solvent(ctx: Context<ViewStaking>) -> Result<bool> {
        let staking = &ctx.accounts.staking;
        Ok(staking.reward_reserve >= staking.promised_rewards)
    }

    /// View: Get stake count for user.
    pub fn get_stake_count(ctx: Context<ViewUserStakes>) -> Result<u64> {
        // In this implementation, each user can have multiple stake accounts
        // This would need to be tracked separately in production
        Ok(1)
    }
}

// -----------------------------------------------------------------------------
// State definitions

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct RewardRate {
    pub duration: u64,
    pub bps: u64,
}

#[account]
pub struct Staking {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub is_funded: bool,
    pub paused: bool,
    pub reward_reserve: u64,
    pub promised_rewards: u64,
    pub reward_rates: Vec<RewardRate>,
    pub bump: u8,
}

#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub amount: u64,
    pub reward: u64,
    pub end_time: i64,
    pub withdrawn: bool,
}

// -----------------------------------------------------------------------------
// Account contexts

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: The treasury is arbitrary; its key is stored in the staking
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1 + 1 + 8 + 8 + (4 + 16 * 4) + 1,
        seeds = [b"staking", treasury.key().as_ref()],
        bump
    )]
    pub staking: Account<'info, Staking>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetSnrgMint<'info> {
    #[account(mut)]
    pub staking: Account<'info, Staking>,
    pub treasury: UncheckedAccount<'info>,
    #[account(
        constraint = authority.key() == staking.treasury
    )]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct FundContract<'info> {
    #[account(mut)]
    pub staking: Account<'info, Staking>,
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    /// CHECK: Authority for the treasury token account
    pub treasury_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub staking: Account<'info, Staking>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 8 + 1,
        seeds = [b"stake", user.key().as_ref(), &Clock::get()?.unix_timestamp.to_le_bytes()],
        bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(mut)]
    pub user_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, has_one = treasury)]
    pub staking: Account<'info, Staking>,
    #[account(mut, close = user, constraint = stake_account.owner == user.key())]
    pub stake_account: Account<'info, StakeAccount>,
    /// CHECK: Derived staking signer PDA
    pub staking_signer: UncheckedAccount<'info>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawEarly<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub staking: Account<'info, Staking>,
    #[account(mut, close = user, constraint = stake_account.owner == user.key())]
    pub stake_account: Account<'info, StakeAccount>,
    /// CHECK: Derived staking signer PDA
    pub staking_signer: UncheckedAccount<'info>,
    #[account(mut)]
    pub staking_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(mut)]
    pub staking: Account<'info, Staking>,
    #[account(
        constraint = authority.key() == staking.treasury
    )]
    pub authority: Signer<'info>,
    pub treasury: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ViewStaking<'info> {
    pub staking: Account<'info, Staking>,
}

#[derive(Accounts)]
pub struct ViewUserStakes<'info> {
    pub user: Signer<'info>,
}

// Constants
pub const EARLY_WITHDRAWAL_FEE_BPS: u64 = 500; // 5%
pub const EMERGENCY_FEE_BPS: u64 = 1000; // 10%
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const SECONDS_PER_DAY: i64 = 86_400;

// Events
#[event]
pub struct StakingInitialized {
    pub treasury: Pubkey,
}

#[event]
pub struct SnrgMintSet {
    pub snrg_mint: Pubkey,
}

#[event]
pub struct ContractFunded {
    pub amount: u64,
}

#[event]
pub struct ReserveToppedUp {
    pub amount: u64,
}

#[event]
pub struct Staked {
    pub user: Pubkey,
    pub amount: u64,
    pub reward: u64,
    pub end_time: i64,
}

#[event]
pub struct Withdrawn {
    pub user: Pubkey,
    pub amount: u64,
    pub reward: u64,
}

#[event]
pub struct WithdrawnEarly {
    pub user: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct EmergencyWithdrawal {
    pub user: Pubkey,
    pub amount: u64,
    pub fee: u64,
}

#[event]
pub struct Paused {}

#[event]
pub struct Unpaused {}

#[event]
pub struct InsufficientReserves {
    pub required: u64,
    pub available: u64,
}

/// Custom errors for staking operations.
#[error_code]
pub enum StakingError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid duration")]
    InvalidDuration,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Stake already withdrawn")]
    AlreadyWithdrawn,
    #[msg("Stake has not matured")]
    NotMatured,
    #[msg("Stake has already matured")]
    AlreadyMatured,
    #[msg("SNRG mint already set")]
    SnrgAlreadySet,
    #[msg("Invalid SNRG mint")]
    InvalidMint,
    #[msg("Staking contract already funded")]
    AlreadyFunded,
    #[msg("SNRG mint not set")]
    SnrgNotSet,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Contract not funded")]
    NotFunded,
    #[msg("Insufficient reserves")]
    InsufficientReserves,
    #[msg("Contract is paused")]
    Paused,
    #[msg("Contract is not paused")]
    NotPaused,
    #[msg("Contract already paused")]
    AlreadyPaused,
}