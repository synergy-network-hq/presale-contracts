use anchor_lang::prelude::*;
use anchor_spl::token::{Burn, Mint, Token, TokenAccount};

declare_id!("ReplaceWithYourActualProgramID11111111111111111111");

pub const FINALIZE_DELAY: i64 = 48 * 60 * 60;      // 48 hours
pub const REOPEN_COOLDOWN: i64 = 7 * 24 * 60 * 60; // 7 days

#[program]
pub mod snrg_swap {
    use super::*;

    #[access_control(ctx.accounts.validate_initialize())]
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        swap.snrg_mint = ctx.accounts.snrg_mint.key();
        swap.treasury = ctx.accounts.treasury.key();
        swap.finalized = false;
        swap.paused = false;
        swap.total_burned = 0;
        swap.merkle_root = [0u8; 32];
        swap.proposed_root = [0u8; 32];
        swap.proposed_at = 0;
        swap.burn_commitment = [0u8; 32];
        swap.last_finalized_at = 0;
        swap.bump = ctx.bumps.swap;

        emit!(SwapInitialized {
            snrg_mint: swap.snrg_mint,
            treasury: swap.treasury,
        });

        Ok(())
    }

    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        let swap = &ctx.accounts.swap;
        require!(!swap.paused, SwapError::Paused);
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        require!(amount > 0, SwapError::ZeroAmount);

        let old_balance = ctx.accounts.user_token.amount;

        // Burn tokens
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.snrg_mint.to_account_info(),
                    from: ctx.accounts.user_token.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Defend against fee-on-transfer / rebasing tokens
        ctx.accounts.user_token.reload()?;
        let actual_burned = old_balance.checked_sub(ctx.accounts.user_token.amount).ok_or(SwapError::MathError)?;
        require_eq!(actual_burned, amount, SwapError::InexactBurn);

        // Credit user exactly the requested amount
        let user_burn = &mut ctx.accounts.user_burn;
        user_burn.amount = user_burn.amount.checked_add(amount).ok_or(SwapError::Overflow)?;
        let swap_mut = &mut ctx.accounts.swap;
        swap_mut.total_burned = swap_mut.total_burned.checked_add(amount).ok_or(SwapError::Overflow)?;

        emit!(Burned {
            user: ctx.accounts.user.key(),
            amount,
            total_for_user: user_burn.amount,
        });

        Ok(())
    }

    pub fn propose_root(ctx: Context<AdminAction>, root: [u8; 32]) -> Result<()> {
        require_neq!(root, [0u8; 32], SwapError::ZeroMerkleRoot);

        let swap = &mut ctx.accounts.swap;
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        require_eq!(swap.proposed_root, [0u8; 32], SwapError::PendingRootExists);

        swap.proposed_root = root;
        swap.proposed_at = Clock::get()?.unix_timestamp;

        emit!(RootProposed {
            root,
            proposed_at: swap.proposed_at,
        });

        Ok(())
    }

    pub fn cancel_proposed_root(ctx: Context<AdminAction>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require_neq!(swap.proposed_root, [0u8; 32], SwapError::NoPendingRoot);

        let old = swap.proposed_root;
        swap.proposed_root = [0u8; 32];
        swap.proposed_at = 0;

        emit!(RootCanceled { root: old });
        Ok(())
    }

    pub fn finalize(ctx: Context<AdminAction>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        require_neq!(swap.proposed_root, [0u8; 32], SwapError::ZeroMerkleRoot);
        require!(swap.total_burned > 0, SwapError::ZeroAmount);

        let now = Clock::get()?.unix_timestamp;
        require!(now >= swap.proposed_at + FINALIZE_DELAY, SwapError::TimelockNotExpired);

        // FIX M001: Cryptographic commitment
        swap.burn_commitment = solana_program::keccak::hashv(&[
            &swap.proposed_root,
            &swap.total_burned.to_le_bytes(),
            &now.to_le_bytes(),
            &crate::ID.to_bytes(),
        ])
        .0;

        swap.finalized = true;
        swap.merkle_root = swap.proposed_root;
        swap.last_finalized_at = now;
        swap.proposed_root = [0u8; 32];
        swap.proposed_at = 0;

        emit!(Finalized {
            merkle_root: swap.merkle_root,
            commitment: swap.burn_commitment,
        });

        Ok(())
    }

    pub fn reopen_finalization(ctx: Context<AdminAction>, new_root: [u8; 32]) -> Result<()> {
        require_neq!(new_root, [0u8; 32], SwapError::ZeroMerkleRoot);

        let swap = &mut ctx.accounts.swap;
        require!(swap.finalized, SwapError::NotFinalized);

        let now = Clock::get()?.unix_timestamp;
        require!(now >= swap.last_finalized_at + REOPEN_COOLDOWN, SwapError::ReopenCooldownActive);

        let previous = swap.merkle_root;
        swap.finalized = false;
        swap.merkle_root = [0u8; 32];
        swap.burn_commitment = [0u8; 32];

        swap.proposed_root = new_root;
        swap.proposed_at = now;

        emit!(FinalizationReopened {
            previous_root: previous,
            new_root,
            proposed_at: now,
        });

        Ok(())
    }

    pub fn pause(ctx: Context<AdminAction>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(!swap.paused, SwapError::AlreadyPaused);
        swap.paused = true;
        emit!(ContractPaused);
        Ok(())
    }

    pub fn unpause(ctx: Context<AdminAction>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(swap.paused, SwapError::NotPaused);
        swap.paused = false;
        emit!(ContractUnpaused);
        Ok(())
    }

    // View
    pub fn get_burned_amount(ctx: Context<ViewBurn>, _user: Pubkey) -> Result<u64> {
        Ok(ctx.accounts.user_burn.amount)
    }
}

impl Initialize<'_> {
    pub fn validate_initialize(&self) -> Result<()> {
        require_keys_eq!(self.treasury.key(), self.swap.treasury, SwapError::Unauthorized);
        Ok(())
    }
}

// Accounts
#[account]
pub struct Swap {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub finalized: bool,
    pub paused: bool,
    pub total_burned: u64,
    pub merkle_root: [u8; 32],
    pub proposed_root: [u8; 32],
    pub proposed_at: i64,
    pub burn_commitment: [u8; 32],
    pub last_finalized_at: i64,
    pub bump: u8,
}

#[account]
pub struct UserBurn {
    pub amount: u64,
}

// Contexts
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: treasury/owner
    pub treasury: UncheckedAccount<'info>,
    pub snrg_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1 + 1 + 8 + 32 + 32 + 8 + 32 + 8 + 1,
        seeds = [b"swap"],
        bump
    )]
    pub swap: Account<'info, Swap>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub snrg_mint: Account<'info, Mint>,
    pub swap: Account<'info, Swap>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 8,
        seeds = [b"burn", user.key().as_ref()],
        bump
    )]
    pub user_burn: Account<'info, UserBurn>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(mut, constraint = authority.key() == swap.treasury)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub swap: Account<'info, Swap>,
}

#[derive(Accounts)]
pub struct ViewBurn<'info> {
    pub user_burn: Account<'info, UserBurn>,
}

// Events — exact match to Solidity
#[event]
pub struct SwapInitialized { pub snrg_mint: Pubkey, pub treasury: Pubkey }
#[event]
pub struct Burned { pub user: Pubkey, pub amount: u64, pub total_for_user: u64 }
#[event]
pub struct RootProposed { pub root: [u8; 32], pub proposed_at: i64 }
#[event]
pub struct RootCanceled { pub root: [u8; 32] }
#[event]
pub struct Finalized { pub merkle_root: [u8; 32], pub commitment: [u8; 32] }
#[event]
pub struct FinalizationReopened { pub previous_root: [u8; 32], pub new_root: [u8; 32], pub proposed_at: i64 }
#[event]
pub struct ContractPaused;
#[event]
pub struct ContractUnpaused;

// Errors — exact match
#[error_code]
pub enum SwapError {
    ZeroAmount,
    ZeroMerkleRoot,
    AlreadyFinalized,
    NotFinalized,
    PendingRootExists,
    NoPendingRoot,
    TimelockNotExpired,
    ReopenCooldownActive,
    Paused,
    AlreadyPaused,
    NotPaused,
    InexactBurn,
    Overflow,
    MathError,
    Unauthorized,
}
