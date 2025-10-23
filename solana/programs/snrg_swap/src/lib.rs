use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Burn, BurnChecked};

declare_id!("SnRGSwap11111111111111111111111111111111");

/// A simplified Solana implementation of the SNRG token swap contract.  Users
/// may burn SNRG tokens in exchange for an off–chain receipt to be redeemed
/// later.  The program tracks the total amount each user has burned and
/// exposes a `finalize` instruction to record a Merkle root of the burn
/// distribution.  After finalization no further burns are accepted.
#[program]
pub mod snrg_swap {
    use super::*;

    /// Initializes the swap state.  Records the SNRG mint.  Must be called
    /// once by the deployer.
    pub fn initialize(ctx: Context<Initialize>, snrg_mint: Pubkey) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        swap.snrg_mint = snrg_mint;
        swap.finalized = false;
        swap.merkle_root = [0u8; 32];
        swap.bump = *ctx.bumps.get("swap").unwrap();
        Ok(())
    }

    /// Burns `amount` of SNRG from the caller and records the burn in a
    /// per–user `UserBurn` account.  Burns are only allowed before the
    /// contract is finalized.
    pub fn burn_for_receipt(ctx: Context<BurnForReceipt>, amount: u64) -> Result<()> {
        require!(amount > 0, SwapError::InvalidAmount);
        require!(!ctx.accounts.swap.finalized, SwapError::AlreadyFinalized);
        // burn tokens from user
        let cpi_accounts = Burn {
            mint: ctx.accounts.snrg_mint.to_account_info(),
            from: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::burn(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        // record burn amount
        let user_burn = &mut ctx.accounts.user_burn;
        user_burn.user = ctx.accounts.user.key();
        user_burn.amount = user_burn
            .amount
            .checked_add(amount)
            .ok_or(SwapError::Overflow)?;
        Ok(())
    }

    /// Finalizes the swap and records a Merkle root representing the burn
    /// distribution.  After finalization no further burns are permitted.
    pub fn finalize(ctx: Context<Finalize>, merkle_root: [u8; 32]) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        swap.finalized = true;
        swap.merkle_root = merkle_root;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

#[account]
pub struct Swap {
    pub snrg_mint: Pubkey,
    pub finalized: bool,
    pub merkle_root: [u8; 32],
    pub bump: u8,
}

#[account]
pub struct UserBurn {
    pub user: Pubkey,
    pub amount: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 32 + 1 + 32 + 1, seeds = [b"swap"], bump)]
    pub swap: Account<'info, Swap>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BurnForReceipt<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_snrgtoken: Account<'info, TokenAccount>,
    pub snrg_mint: Account<'info, Mint>,
    #[account(mut)]
    pub swap: Account<'info, Swap>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 32 + 8,
        seeds = [b"burn", user.key().as_ref()],
        bump
    )]
    pub user_burn: Account<'info, UserBurn>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Finalize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub swap: Account<'info, Swap>,
}

#[error_code]
pub enum SwapError {
    #[msg("Invalid amount")] InvalidAmount,
    #[msg("Swap already finalized")] AlreadyFinalized,
    #[msg("Overflow")] Overflow,
}