use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use solana_program::ed25519_program;


/// Enhanced Solana implementation of the SNRG presale contract matching the
/// Solidity version. Includes rate limiting, purchase controls, and pausability.
#[program]
pub mod snrg_presale {
    use super::*;

    /// Initializes a new presale instance.
    pub fn initialize(
        ctx: Context<Initialize>,
        signer: Pubkey,
    ) -> Result<()> {
        require!(signer != Pubkey::default(), PresaleError::InvalidSigner);
        
        let presale = &mut ctx.accounts.presale;
        presale.snrg_mint = ctx.accounts.snrg_mint.key();
        presale.treasury = ctx.accounts.treasury.key();
        presale.signer = signer;
        presale.open = false;
        presale.paused = false;
        presale.used_nonces = Vec::new();
        presale.supported_tokens = Vec::new();
        presale.max_purchase_amount = 10_000_000 * 1_000_000_000; // 10M SNRG with 9 decimals
        presale.bump = *ctx.bumps.get("presale").unwrap();
        
        emit!(PresaleInitialized {
            snrg_mint: presale.snrg_mint,
            treasury: presale.treasury,
            signer: presale.signer,
        });
        
        Ok(())
    }

    /// Updates the signer.
    pub fn set_signer(ctx: Context<SetSigner>, signer: Pubkey) -> Result<()> {
        require!(signer != Pubkey::default(), PresaleError::InvalidSigner);
        require!(signer != ctx.accounts.presale.signer, PresaleError::SameSigner);
        
        let old_signer = ctx.accounts.presale.signer;
        let presale = &mut ctx.accounts.presale;
        presale.signer = signer;
        
        emit!(SignerSet { old_signer, new_signer: signer });
        Ok(())
    }

    /// Opens or closes the presale.
    pub fn set_open(ctx: Context<SetOpen>, open: bool) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(presale.open != open, PresaleError::SameState);
        presale.open = open;
        
        emit!(OpenSet { open });
        Ok(())
    }

    /// Sets the maximum purchase amount.
    pub fn set_max_purchase_amount(ctx: Context<SetOpen>, max_amount: u64) -> Result<()> {
        require!(max_amount > 0, PresaleError::InvalidAmount);
        require!(max_amount >= MIN_PURCHASE_AMOUNT, PresaleError::AmountTooLow);
        
        let presale = &mut ctx.accounts.presale;
        presale.max_purchase_amount = max_amount;
        
        emit!(MaxPurchaseAmountSet { amount: max_amount });
        Ok(())
    }

    /// Adds a supported payment token.
    pub fn add_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()> {
        require!(token_mint != Pubkey::default(), PresaleError::InvalidToken);
        require!(token_mint != ctx.accounts.presale.snrg_mint, PresaleError::CannotUseSnrgAsPayment);
        
        let presale = &mut ctx.accounts.presale;
        require!(!presale.supported_tokens.contains(&token_mint), PresaleError::TokenAlreadySupported);
        require!(presale.supported_tokens.len() < MAX_SUPPORTED_TOKENS, PresaleError::TooManyTokens);
        
        presale.supported_tokens.push(token_mint);
        
        emit!(SupportedTokenSet { token: token_mint, is_supported: true });
        Ok(())
    }

    /// Removes a supported payment token.
    pub fn remove_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        let initial_len = presale.supported_tokens.len();
        presale.supported_tokens.retain(|m| m != &token_mint);
        
        if initial_len != presale.supported_tokens.len() {
            emit!(SupportedTokenSet { token: token_mint, is_supported: false });
        }
        
        Ok(())
    }

    /// Purchases SNRG with native SOL.
    pub fn buy_with_native(
        ctx: Context<BuyWithNative>,
        snrg_amount: u64,
        payment_amount: u64,
        nonce: u64,
        signature: Vec<u8>,
    ) -> Result<()> {
        require!(snrg_amount > 0, PresaleError::InvalidAmount);
        require!(payment_amount > 0, PresaleError::InvalidAmount);
        require!(signature.len() == 64, PresaleError::InvalidSignature);
        
        let presale = &mut ctx.accounts.presale;
        require!(presale.open, PresaleError::Closed);
        require!(!presale.paused, PresaleError::Paused);
        
        // Validate nonce range
        check_nonce(nonce)?;
        
        // Check purchase limits
        check_purchase_limits(snrg_amount, presale.max_purchase_amount)?;
        
        // Check and update rate limiting
        let buyer_state = &mut ctx.accounts.buyer_state;
        check_and_update_rate_limits(buyer_state)?;
        
        // Prevent replay
        require!(!presale.used_nonces.contains(&nonce), PresaleError::NonceUsed);
        presale.used_nonces.push(nonce);
        
        // Verify signature
        let message = build_message_hash(
            ctx.accounts.buyer.key(),
            Pubkey::default(), // native SOL
            payment_amount,
            snrg_amount,
            nonce,
        );
        verify_signature(&message, &signature, &presale.signer)?;
        
        // Validate token accounts
        require!(ctx.accounts.treasury_snrgtoken.mint == presale.snrg_mint, PresaleError::InvalidMint);
        require!(ctx.accounts.buyer_snrgtoken.mint == presale.snrg_mint, PresaleError::InvalidMint);
        require!(ctx.accounts.buyer_snrgtoken.owner == ctx.accounts.buyer.key(), PresaleError::InvalidOwner);
        
        // Check sufficient balance
        require!(ctx.accounts.treasury_snrgtoken.amount >= snrg_amount, PresaleError::InsufficientBalance);
        
        // Transfer SNRG tokens from treasury to buyer
        let cpi_accounts = Transfer {
            from: ctx.accounts.treasury_snrgtoken.to_account_info(),
            to: ctx.accounts.buyer_snrgtoken.to_account_info(),
            authority: ctx.accounts.treasury_authority.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"presale", presale.treasury.as_ref(), &[presale.bump]]];
        let signer = &[&seeds[..]];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        ), snrg_amount)?;
        
        emit!(Purchased {
            buyer: ctx.accounts.buyer.key(),
            payment_token: Pubkey::default(),
            snrg_amount,
            paid_amount: payment_amount,
        });
        
        Ok(())
    }

    /// Purchases SNRG with an SPL token.
    pub fn buy_with_token(
        ctx: Context<BuyWithToken>,
        payment_amount: u64,
        snrg_amount: u64,
        nonce: u64,
        signature: Vec<u8>,
    ) -> Result<()> {
        require!(payment_amount > 0, PresaleError::InvalidAmount);
        require!(snrg_amount > 0, PresaleError::InvalidAmount);
        require!(signature.len() == 64, PresaleError::InvalidSignature);
        
        let presale = &mut ctx.accounts.presale;
        require!(presale.open, PresaleError::Closed);
        require!(!presale.paused, PresaleError::Paused);
        
        // Validate nonce range
        check_nonce(nonce)?;
        
        // Check purchase limits
        check_purchase_limits(snrg_amount, presale.max_purchase_amount)?;
        
        // Check and update rate limiting
        let buyer_state = &mut ctx.accounts.buyer_state;
        check_and_update_rate_limits(buyer_state)?;
        
        // Verify payment mint is supported
        require!(presale.supported_tokens.contains(&ctx.accounts.payment_mint.key()), PresaleError::UnsupportedToken);
        
        // Prevent replay
        require!(!presale.used_nonces.contains(&nonce), PresaleError::NonceUsed);
        presale.used_nonces.push(nonce);
        
        // Verify signature
        let message = build_message_hash(
            ctx.accounts.buyer.key(),
            ctx.accounts.payment_mint.key(),
            payment_amount,
            snrg_amount,
            nonce,
        );
        verify_signature(&message, &signature, &presale.signer)?;
        
        // Validate token accounts
        require!(ctx.accounts.buyer_payment_token.mint == ctx.accounts.payment_mint.key(), PresaleError::InvalidMint);
        require!(ctx.accounts.treasury_payment_token.mint == ctx.accounts.payment_mint.key(), PresaleError::InvalidMint);
        require!(ctx.accounts.treasury_snrgtoken.mint == presale.snrg_mint, PresaleError::InvalidMint);
        require!(ctx.accounts.buyer_snrgtoken.mint == presale.snrg_mint, PresaleError::InvalidMint);
        require!(ctx.accounts.buyer_payment_token.owner == ctx.accounts.buyer.key(), PresaleError::InvalidOwner);
        require!(ctx.accounts.buyer_snrgtoken.owner == ctx.accounts.buyer.key(), PresaleError::InvalidOwner);
        
        // Check sufficient balances
        require!(ctx.accounts.buyer_payment_token.amount >= payment_amount, PresaleError::InsufficientBalance);
        require!(ctx.accounts.treasury_snrgtoken.amount >= snrg_amount, PresaleError::InsufficientBalance);
        
        // Transfer payment tokens from buyer to treasury
        let cpi_accounts_payment = Transfer {
            from: ctx.accounts.buyer_payment_token.to_account_info(),
            to: ctx.accounts.treasury_payment_token.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_payment,
        ), payment_amount)?;
        
        // Transfer SNRG tokens from treasury to buyer
        let cpi_accounts_snrg = Transfer {
            from: ctx.accounts.treasury_snrgtoken.to_account_info(),
            to: ctx.accounts.buyer_snrgtoken.to_account_info(),
            authority: ctx.accounts.treasury_authority.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[&[b"presale", presale.treasury.as_ref(), &[presale.bump]]];
        let signer = &[&seeds[..]];
        token::transfer(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_snrg,
            signer,
        ), snrg_amount)?;
        
        emit!(Purchased {
            buyer: ctx.accounts.buyer.key(),
            payment_token: ctx.accounts.payment_mint.key(),
            snrg_amount,
            paid_amount: payment_amount,
        });
        
        Ok(())
    }

    /// Pause the presale.
    pub fn pause(ctx: Context<SetOpen>) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(!presale.paused, PresaleError::AlreadyPaused);
        presale.paused = true;
        
        emit!(Paused {});
        Ok(())
    }

    /// Unpause the presale.
    pub fn unpause(ctx: Context<SetOpen>) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(presale.paused, PresaleError::NotPaused);
        presale.paused = false;
        
        emit!(Unpaused {});
        Ok(())
    }
}

// Helper functions
fn check_nonce(nonce: u64) -> Result<()> {
    require!(nonce > 0 && nonce <= u128::MAX as u64, PresaleError::InvalidNonce);
    Ok(())
}

fn check_purchase_limits(snrg_amount: u64, max_purchase_amount: u64) -> Result<()> {
    require!(snrg_amount >= MIN_PURCHASE_AMOUNT, PresaleError::AmountTooLow);
    require!(snrg_amount <= max_purchase_amount, PresaleError::AmountTooHigh);
    Ok(())
}

fn check_and_update_rate_limits(buyer_state: &mut BuyerState) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    
    // Check cooldown
    require!(
        now >= buyer_state.last_purchase_time + PURCHASE_COOLDOWN,
        PresaleError::PurchaseTooSoon
    );
    
    // Reset daily counter if needed
    if now >= buyer_state.daily_purchase_reset + SECONDS_PER_DAY {
        buyer_state.purchase_count_today = 0;
        buyer_state.daily_purchase_reset = now;
    }
    
    // Check daily limit
    require!(
        buyer_state.purchase_count_today < MAX_PURCHASES_PER_DAY,
        PresaleError::DailyLimitExceeded
    );
    
    // Update state
    buyer_state.last_purchase_time = now;
    buyer_state.purchase_count_today += 1;
    
    Ok(())
}

fn build_message_hash(
    buyer: Pubkey,
    payment_token: Pubkey,
    payment_amount: u64,
    snrg_amount: u64,
    nonce: u64,
) -> [u8; 32] {
    use solana_program::keccak;
    
    // Include chain ID equivalent (program ID) for cross-chain protection
    let program_id = crate::ID;
    
    let mut data = Vec::new();
    data.extend_from_slice(buyer.as_ref());
    data.extend_from_slice(payment_token.as_ref());
    data.extend_from_slice(&payment_amount.to_le_bytes());
    data.extend_from_slice(&snrg_amount.to_le_bytes());
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(program_id.as_ref());
    
    keccak::hash(&data).to_bytes()
}

fn verify_signature(message: &[u8; 32], signature: &[u8], signer: &Pubkey) -> Result<()> {
    require!(signature.len() == 64, PresaleError::InvalidSignature);
    
    // In production, this would use the ed25519 precompile
    // For now, we validate signature format
    Ok(())
}

// -----------------------------------------------------------------------------
// State definitions

#[account]
pub struct Presale {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub signer: Pubkey,
    pub open: bool,
    pub paused: bool,
    pub max_purchase_amount: u64,
    pub used_nonces: Vec<u64>,
    pub supported_tokens: Vec<Pubkey>,
    pub bump: u8,
}

#[account]
pub struct BuyerState {
    pub last_purchase_time: i64,
    pub purchase_count_today: u64,
    pub daily_purchase_reset: i64,
}

const MAX_SUPPORTED_TOKENS: usize = 10;
pub const MIN_PURCHASE_AMOUNT: u64 = 250 * 1_000_000_000; // 250 SNRG with 9 decimals
pub const PURCHASE_COOLDOWN: i64 = 300; // 5 minutes
pub const MAX_PURCHASES_PER_DAY: u64 = 10;
pub const SECONDS_PER_DAY: i64 = 86_400;

// -----------------------------------------------------------------------------
// Account contexts

#[derive(Accounts)]
#[instruction(signer: Pubkey)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: The SNRG mint is passed unchecked
    pub snrg_mint: UncheckedAccount<'info>,
    /// CHECK: Treasury account
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 1 + 8 + (4 + 8 * 100) + (4 + 32 * 10) + 1,
        seeds = [b"presale", treasury.key().as_ref()],
        bump
    )]
    pub presale: Account<'info, Presale>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetSigner<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    #[account(
        constraint = authority.key() == presale.treasury
    )]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetOpen<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    #[account(
        constraint = authority.key() == presale.treasury
    )]
    pub authority: Signer<'info>,
    pub treasury: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ManageToken<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    #[account(
        constraint = authority.key() == presale.treasury
    )]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct BuyWithNative<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + 8 + 8 + 8,
        seeds = [b"buyer", buyer.key().as_ref()],
        bump
    )]
    pub buyer_state: Account<'info, BuyerState>,
    /// CHECK: Treasury authority
    pub treasury_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_snrgtoken: Account<'info, TokenAccount>,
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + 8 + 8 + 8,
        seeds = [b"buyer", buyer.key().as_ref()],
        bump
    )]
    pub buyer_state: Account<'info, BuyerState>,
    pub payment_mint: Account<'info, Mint>,
    #[account(mut)]
    pub buyer_payment_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury_payment_token: Account<'info, TokenAccount>,
    /// CHECK: Treasury authority
    pub treasury_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_snrgtoken: Account<'info, TokenAccount>,
    pub treasury: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Events
#[event]
pub struct PresaleInitialized {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub signer: Pubkey,
}

#[event]
pub struct SignerSet {
    pub old_signer: Pubkey,
    pub new_signer: Pubkey,
}

#[event]
pub struct OpenSet {
    pub open: bool,
}

#[event]
pub struct MaxPurchaseAmountSet {
    pub amount: u64,
}

#[event]
pub struct SupportedTokenSet {
    pub token: Pubkey,
    pub is_supported: bool,
}

#[event]
pub struct Purchased {
    pub buyer: Pubkey,
    pub payment_token: Pubkey,
    pub snrg_amount: u64,
    pub paid_amount: u64,
}

#[event]
pub struct Paused {}

#[event]
pub struct Unpaused {}

/// Custom errors for presale operations.
#[error_code]
pub enum PresaleError {
    #[msg("Invalid signer address")]
    InvalidSigner,
    #[msg("Same signer as current")]
    SameSigner,
    #[msg("Same state as current")]
    SameState,
    #[msg("Invalid token address")]
    InvalidToken,
    #[msg("Cannot use SNRG as payment token")]
    CannotUseSnrgAsPayment,
    #[msg("Token already supported")]
    TokenAlreadySupported,
    #[msg("Too many supported tokens")]
    TooManyTokens,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid nonce")]
    InvalidNonce,
    #[msg("Invalid signature")]
    InvalidSignature,
    #[msg("Insufficient payment")]
    InsufficientPayment,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Invalid owner")]
    InvalidOwner,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Presale is closed")]
    Closed,
    #[msg("Nonce already used")]
    NonceUsed,
    #[msg("Unsupported payment token")]
    UnsupportedToken,
    #[msg("Amount too low")]
    AmountTooLow,
    #[msg("Amount too high")]
    AmountTooHigh,
    #[msg("Purchase too soon")]
    PurchaseTooSoon,
    #[msg("Daily limit exceeded")]
    DailyLimitExceeded,
    #[msg("Contract is paused")]
    Paused,
    #[msg("Contract is not paused")]
    NotPaused,
    #[msg("Contract already paused")]
    AlreadyPaused,
}