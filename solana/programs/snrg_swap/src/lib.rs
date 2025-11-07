use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Burn};


/// A Solana implementation of the SNRG token swap contract matching the Solidity version.
/// Users may burn SNRG tokens in exchange for an off-chain receipt to be redeemed later.
/// The program tracks the total amount each user has burned and exposes a `finalize` 
/// instruction to record a Merkle root of the burn distribution. After finalization no 
/// further burns are accepted. Includes pausable functionality and ReentrancyGuard equivalent.
#[program]
pub mod snrg_swap {
    use super::*;

    /// Initializes the swap state. Records the SNRG mint and treasury.
    /// Must be called once by the deployer.
    pub fn initialize(ctx: Context<Initialize>, snrg_mint: Pubkey, treasury: Pubkey) -> Result<()> {
        require!(snrg_mint != Pubkey::default(), SwapError::ZeroAddress);
        require!(treasury != Pubkey::default(), SwapError::ZeroAddress);
        
        let swap = &mut ctx.accounts.swap;
        swap.snrg_mint = snrg_mint;
        swap.treasury = treasury;
        swap.finalized = false;
        swap.paused = false;
        swap.merkle_root = [0u8; 32];
        swap.bump = *ctx.bumps.get("swap").unwrap();
        
        emit!(SwapInitialized {
            snrg_mint,
        });
        
        Ok(())
    }

    /// Burns `amount` of SNRG from the caller and records the burn in a
    /// per-user `UserBurn` account. Burns are only allowed before the
    /// contract is finalized and when not paused.
    pub fn burn_for_receipt(ctx: Context<BurnForReceipt>, amount: u64) -> Result<()> {
        let swap = &ctx.accounts.swap;
        
        require!(!swap.paused, SwapError::ContractPaused);
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        require!(amount > 0, SwapError::ZeroAmount);
        
        // Validate token accounts
        require!(ctx.accounts.user_snrgtoken.mint == swap.snrg_mint, SwapError::InvalidMint);
        require!(ctx.accounts.snrg_mint.key() == swap.snrg_mint, SwapError::InvalidMint);
        require!(ctx.accounts.user_snrgtoken.owner == ctx.accounts.user.key(), SwapError::InvalidOwner);
        require!(ctx.accounts.user_snrgtoken.amount >= amount, SwapError::InsufficientBalance);
        
        // Update state before external call (CEI pattern / reentrancy guard equivalent)
        let user_burn = &mut ctx.accounts.user_burn;
        user_burn.user = ctx.accounts.user.key();
        user_burn.amount = user_burn
            .amount
            .checked_add(amount)
            .ok_or(SwapError::Overflow)?;
        
        // Burn tokens from user
        let cpi_accounts = Burn {
            mint: ctx.accounts.snrg_mint.to_account_info(),
            from: ctx.accounts.user_snrgtoken.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::burn(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        
        emit!(TokenBurned {
            user: ctx.accounts.user.key(),
            amount,
            total_burned: user_burn.amount,
        });
        
        Ok(())
    }

    /// Finalizes the swap and records a Merkle root representing the burn
    /// distribution. After finalization no further burns are permitted.
    /// Only the owner (treasury) may call this.
    pub fn finalize(ctx: Context<Finalize>, merkle_root: [u8; 32]) -> Result<()> {
        require!(merkle_root != [0u8; 32], SwapError::ZeroMerkleRoot);
        
        let swap = &mut ctx.accounts.swap;
        require!(!swap.finalized, SwapError::AlreadyFinalized);
        
        swap.finalized = true;
        swap.merkle_root = merkle_root;
        
        emit!(SwapFinalized {
            merkle_root,
        });
        
        Ok(())
    }

    /// Pauses the contract. Only the owner (treasury) may call this.
    pub fn pause(ctx: Context<ManageContract>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(!swap.paused, SwapError::AlreadyPaused);
        swap.paused = true;
        
        emit!(ContractPaused {});
        Ok(())
    }

    /// Unpauses the contract. Only the owner (treasury) may call this.
    pub fn unpause(ctx: Context<ManageContract>) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(swap.paused, SwapError::NotPaused);
        swap.paused = false;
        
        emit!(ContractUnpaused {});
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

#[account]
pub struct Swap {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub finalized: bool,
    pub paused: bool,
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
    #[account(
        init, 
        payer = payer, 
        space = 8 + 32 + 32 + 1 + 1 + 32 + 1, 
        seeds = [b"swap"], 
        bump
    )]
    pub swap: Account<'info, Swap>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BurnForReceipt<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub snrg_mint: Account<'info, Mint>,
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
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = treasury @ SwapError::UnauthorizedCaller,
        constraint = authority.key() == swap.treasury @ SwapError::UnauthorizedCaller
    )]
    pub swap: Account<'info, Swap>,
    /// CHECK: Treasury authority
    pub treasury: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ManageContract<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = treasury @ SwapError::UnauthorizedCaller,
        constraint = authority.key() == swap.treasury @ SwapError::UnauthorizedCaller
    )]
    pub swap: Account<'info, Swap>,
    /// CHECK: Treasury authority
    pub treasury: UncheckedAccount<'info>,
}

// -----------------------------------------------------------------------------
// Events

#[event]
pub struct SwapInitialized {
    pub snrg_mint: Pubkey,
}

#[event]
pub struct TokenBurned {
    pub user: Pubkey,
    pub amount: u64,
    pub total_burned: u64,
}

#[event]
pub struct SwapFinalized {
    pub merkle_root: [u8; 32],
}

#[event]
pub struct ContractPaused {}

#[event]
pub struct ContractUnpaused {}

// -----------------------------------------------------------------------------
// Errors

#[error_code]
pub enum SwapError {
    #[msg("Zero address not allowed")]
    ZeroAddress,
    
    #[msg("Invalid mint address")]
    InvalidMint,
    
    #[msg("Zero amount not allowed")]
    ZeroAmount,
    
    #[msg("Swap already finalized")]
    AlreadyFinalized,
    
    #[msg("Math overflow")]
    Overflow,
    
    #[msg("Invalid owner")]
    InvalidOwner,
    
    #[msg("Insufficient balance")]
    InsufficientBalance,
    
    #[msg("Zero merkle root not allowed")]
    ZeroMerkleRoot,
    
    #[msg("Unauthorized caller - only owner allowed")]
    UnauthorizedCaller,
    
    #[msg("Contract is paused")]
    ContractPaused,
    
    #[msg("Contract already paused")]
    AlreadyPaused,
    
    #[msg("Contract not paused")]
    NotPaused,
}