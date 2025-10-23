use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("SnRGStaking111111111111111111111111111111");

/// A simplified Solana implementation of the SNRG staking contract.  Users may
/// stake SNRG for fixed durations and earn predetermined rewards.  Stakes are
/// tracked in per–user stake accounts.  Early withdrawal is allowed but
/// assessed a fee that is forwarded to the treasury.  The contract must be
/// funded by the owner before stakes can be withdrawn.
#[program]
pub mod snrg_staking {
    use super::*;

    /// Initializes the staking state.  The owner defines the treasury that
    /// collects early withdrawal fees.  Reward rates for common durations are
    /// hard–coded and match the Ethereum version of the contract.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        staking.treasury = ctx.accounts.treasury.key();
        staking.snrg_mint = Pubkey::default();
        staking.is_funded = false;
        staking.reward_rates = vec![
            RewardRate { duration: 30, bps: 125 },
            RewardRate { duration: 60, bps: 250 },
            RewardRate { duration: 90, bps: 375 },
            RewardRate { duration: 180, bps: 500 },
        ];
        staking.bump = *ctx.bumps.get("staking").unwrap();
        Ok(())
    }

    /// Sets the SNRG mint.  This can only be called once and must be invoked
    /// before any stakes are created.  The owner must provide the mint and
    /// ensure that the staking PDA has authority to transfer tokens from the
    /// treasury.
    pub fn set_snrg_mint(ctx: Context<SetSnrgMint>, snrg_mint: Pubkey) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        require!(staking.snrg_mint == Pubkey::default(), StakingError::SnrgAlreadySet);
        require!(snrg_mint != Pubkey::default(), StakingError::InvalidMint);
        staking.snrg_mint = snrg_mint;
        Ok(())
    }

    /// Funds the staking contract by transferring rewards from the treasury
    /// token account into the staking vault.  This function may only be
    /// executed once.  The treasury must approve the transfer ahead of time.
    pub fn fund_contract(ctx: Context<FundContract>, amount: u64) -> Result<()> {
        let staking = &mut ctx.accounts.staking;
        require!(!staking.is_funded, StakingError::AlreadyFunded);
        require!(amount > 0, StakingError::InvalidAmount);
        staking.is_funded = true;
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_snrgtoken.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.treasury_authority.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        Ok(())
    }

    /// Stakes `amount` of SNRG for `duration` days.  A new `StakeAccount` is
    /// created for each stake.  The reward is calculated using the configured
    /// basis points for the given duration.  The user must transfer the
    /// principal amount to the staking vault.
    pub fn stake(ctx: Context<Stake>, amount: u64, duration: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        let staking = &ctx.accounts.staking;
        // look up reward basis points
        let mut reward_bps = None;
        for rate in staking.reward_rates.iter() {
            if rate.duration == duration {
                reward_bps = Some(rate.bps);
                break;
            }
        }
        require!(reward_bps.is_some(), StakingError::InvalidDuration);
        let reward_bps = reward_bps.unwrap();
        // transfer tokens from user to staking vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_snrgtoken.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        // compute reward and end_time
        let reward = amount
            .checked_mul(reward_bps as u64)
            .ok_or(StakingError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(StakingError::MathOverflow)?;
        let end_time = Clock::get()?.unix_timestamp + (duration as i64 * 86_400);
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.owner = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.reward = reward;
        stake_account.end_time = end_time;
        stake_account.withdrawn = false;
        Ok(())
    }

    /// Withdraws a matured stake.  Transfers principal + reward from the
    /// staking vault to the user.  The stake must not have been withdrawn
    /// previously and the current time must exceed the recorded end_time.
    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        require!(!stake_account.withdrawn, StakingError::AlreadyWithdrawn);
        require!(Clock::get()?.unix_timestamp >= stake_account.end_time, StakingError::NotMatured);
        stake_account.withdrawn = true;
        let total = stake_account
            .amount
            .checked_add(stake_account.reward)
            .ok_or(StakingError::MathOverflow)?;
        // transfer tokens from staking vault to user
        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"staking", ctx.accounts.staking.treasury.as_ref(), &[ctx.accounts.staking.bump]]];
        let signer = &[&seeds[..]];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ), total)?;
        Ok(())
    }

    /// Performs an early withdrawal of principal.  A fee is assessed and sent to
    /// the treasury.  Rewards are forfeited.  The stake must not have
    /// matured.  After this instruction the stake is marked as withdrawn.
    pub fn withdraw_early(ctx: Context<WithdrawEarly>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        require!(!stake_account.withdrawn, StakingError::AlreadyWithdrawn);
        require!(Clock::get()?.unix_timestamp < stake_account.end_time, StakingError::AlreadyMatured);
        stake_account.withdrawn = true;
        // compute fee = amount * 5% (500 bps) / 10000
        let fee = stake_account
            .amount
            .checked_mul(EARLY_WITHDRAWAL_FEE_BPS as u64)
            .ok_or(StakingError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(StakingError::MathOverflow)?;
        let return_amount = stake_account.amount - fee;
        // transfer fee to treasury
        let cpi_accounts_fee = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.treasury_snrgtoken.to_account_info(),
            authority: ctx.accounts.staking_signer.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"staking", ctx.accounts.staking.treasury.as_ref(), &[ctx.accounts.staking.bump]]];
        let signer = &[&seeds[..]];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_fee,
            signer,
        ), fee)?;
        // transfer remainder to user
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
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state definitions

/// Reward rate definition.  A simple struct storing the duration in days and
/// the basis points reward for that duration.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct RewardRate {
    pub duration: u64,
    pub bps: u64,
}

/// Global staking state.  Holds the SNRG mint, treasury, funding flag, set of
/// reward rates and bump seed.  Stored as a PDA keyed by the treasury.
#[account]
pub struct Staking {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub is_funded: bool,
    pub reward_rates: Vec<RewardRate>,
    pub bump: u8,
}

/// Per–stake account that records the user, principal amount, reward amount,
/// maturity timestamp and withdrawal status.
#[account]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub amount: u64,
    pub reward: u64,
    pub end_time: i64,
    pub withdrawn: bool,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: The treasury is arbitrary; its key is stored in the staking
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1 + (4 + 32 * 4) + 1,
        seeds = [b"staking", treasury.key().as_ref()],
        bump
    )]
    pub staking: Account<'info, Staking>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetSnrgMint<'info> {
    #[account(mut, has_one = treasury)]
    pub staking: Account<'info, Staking>,
    pub treasury: UncheckedAccount<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct FundContract<'info> {
    #[account(mut, has_one = treasury)]
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
    #[account(mut, has_one = treasury)]
    pub staking: Account<'info, Staking>,
    #[account(
        init,
        payer = user,
        space = 8 + 32 + 8 + 8 + 8 + 1,
        seeds = [b"stake", user.key().as_ref(), &[bump]],
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
    #[account(mut, close = user, has_one = owner)]
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
    #[account(mut, has_one = treasury)]
    pub staking: Account<'info, Staking>,
    #[account(mut, close = user, has_one = owner)]
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

// Utility constant for early withdrawal fee (5%).
pub const EARLY_WITHDRAWAL_FEE_BPS: u64 = 500;

/// Custom errors for staking operations.
#[error_code]
pub enum StakingError {
    #[msg("Invalid amount")] InvalidAmount,
    #[msg("Invalid duration")] InvalidDuration,
    #[msg("Math overflow")] MathOverflow,
    #[msg("Stake already withdrawn")] AlreadyWithdrawn,
    #[msg("Stake has not matured")] NotMatured,
    #[msg("Stake has already matured")] AlreadyMatured,
    #[msg("SNRG mint already set")] SnrgAlreadySet,
    #[msg("Invalid SNRG mint")] InvalidMint,
    #[msg("Staking contract already funded")] AlreadyFunded,
}

// Placeholder function to generate a pseudo stake index for PDA seeds.  In an
// actual implementation you might track stake counts in a separate account.
fn stakes_count(_user: &Signer) -> u8 {
    0
}