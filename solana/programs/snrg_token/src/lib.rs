use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer, MintTo};

declare_id!("SnRGToken111111111111111111111111111111");

/// A simplified Solana analogue of the SNRG ERC20 token.  It mints a fixed
/// supply to the treasury and restricts transfers such that only designated
/// staking and swap programs may send or receive tokens.  A rescue registry
/// may be set to allow special transfers in emergency scenarios.  Note that
/// this program does not replace the SPL Token program but wraps around it
/// with additional checks.
#[program]
pub mod snrg_token {
    use super::*;

    /// Initializes the token state.  Mints the full supply to the treasury
    /// using a PDA as the mint authority.  The mint must be created ahead of
    /// time with this PDA as its mint authority.
    pub fn initialize(ctx: Context<Initialize>, total_supply: u64) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        token_state.mint = ctx.accounts.mint.key();
        token_state.treasury = ctx.accounts.treasury.key();
        token_state.staking = Pubkey::default();
        token_state.swap = Pubkey::default();
        token_state.rescue_registry = Pubkey::default();
        token_state.bump = *ctx.bumps.get("token_state").unwrap();
        // mint total supply to treasury
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.treasury_token.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"token", token_state.treasury.as_ref(), &[token_state.bump]]];
        let signer = &[&seeds[..]];
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ), total_supply)?;
        Ok(())
    }

    /// Sets the addresses of the staking program, swap program and rescue
    /// registry.  Only the authority (treasury) may call this.
    pub fn set_endpoints(ctx: Context<SetEndpoints>, staking: Pubkey, swap: Pubkey, rescue_registry: Pubkey) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        token_state.staking = staking;
        token_state.swap = swap;
        token_state.rescue_registry = rescue_registry;
        Ok(())
    }

    /// Transfers tokens subject to restriction rules.  Only permitted transfers
    /// are allowed: tokens may move from the treasury, staking or swap
    /// accounts, or to the staking or swap accounts.  Transfers initiated by
    /// the rescue registry are also permitted.
    pub fn transfer_restricted(ctx: Context<TransferRestricted>, amount: u64) -> Result<()> {
        let token_state = &ctx.accounts.token_state;
        let from_owner = ctx.accounts.from_token.owner;
        let to_owner = ctx.accounts.to_token.owner;
        // Determine allowed directions
        let from_allowed = from_owner == token_state.staking || from_owner == token_state.swap || from_owner == token_state.treasury;
        let to_allowed = to_owner == token_state.staking || to_owner == token_state.swap;
        let mut rescue_move = false;
        if token_state.rescue_registry != Pubkey::default() {
            // If the caller is the rescue registry we allow transfer
            rescue_move = ctx.accounts.authority.key() == token_state.rescue_registry;
        }
        require!(from_allowed || to_allowed || rescue_move, TokenError::TransferNotAllowed);
        // perform transfer via SPL token program
        let cpi_accounts = Transfer {
            from: ctx.accounts.from_token.to_account_info(),
            to: ctx.accounts.to_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

#[account]
pub struct TokenState {
    pub mint: Pubkey,
    pub treasury: Pubkey,
    pub staking: Pubkey,
    pub swap: Pubkey,
    pub rescue_registry: Pubkey,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub treasury_token: Account<'info, TokenAccount>,
    /// CHECK: This PDA must match the mint's mint authority
    #[account(seeds = [b"token", treasury.key().as_ref()], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 1,
        seeds = [b"token", treasury.key().as_ref()],
        bump
    )]
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Treasury that receives the initial supply
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetEndpoints<'info> {
    #[account(mut, has_one = treasury)]
    pub token_state: Account<'info, TokenState>,
    /// CHECK: Only the treasury may call this
    pub treasury: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferRestricted<'info> {
    #[account(mut)]
    pub from_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_token: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_state: Account<'info, TokenState>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum TokenError {
    #[msg("Transfer not allowed between accounts")] TransferNotAllowed,
}