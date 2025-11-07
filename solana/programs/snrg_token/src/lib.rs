use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer, MintTo};



/// A Solana implementation of the SNRG presale token matching the ERC20 version.
/// Mints a fixed supply to the treasury and restricts transfers such that only 
/// designated staking, swap, and presale programs may send or receive tokens.
/// A rescue registry may be set to allow special transfers in emergency scenarios.
#[program]
pub mod snrg_token {
    use super::*;

    /// Initializes the token state. Mints the full supply to the treasury
    /// using a PDA as the mint authority. The mint must be created ahead of
    /// time with this PDA as its mint authority.
    pub fn initialize(ctx: Context<Initialize>, total_supply: u64) -> Result<()> {
        require!(total_supply > 0, TokenError::InvalidSupply);
        
        let token_state = &mut ctx.accounts.token_state;
        token_state.mint = ctx.accounts.mint.key();
        token_state.treasury = ctx.accounts.treasury.key();
        token_state.staking = Pubkey::default();
        token_state.swap = Pubkey::default();
        token_state.presale = Pubkey::default();
        token_state.rescue_registry = Pubkey::default();
        token_state.endpoints_set = false;
        token_state.bump = *ctx.bumps.get("token_state").unwrap();
        
        // Validate PDA derivation
        let (expected_pda, expected_bump) = Pubkey::find_program_address(
            &[b"token", ctx.accounts.treasury.key().as_ref()],
            ctx.program_id
        );
        require!(ctx.accounts.mint_authority.key() == expected_pda, TokenError::InvalidPDA);
        require!(token_state.bump == expected_bump, TokenError::InvalidBump);
        
        // Mint total supply to treasury (6 billion tokens with 9 decimals)
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
        
        emit!(TokenInitialized {
            mint: token_state.mint,
            treasury: token_state.treasury,
            total_supply,
        });
        
        Ok(())
    }

    /// Sets the addresses of the staking program, swap program, presale program,
    /// and rescue registry. Only the authority (treasury) may call this, and it
    /// can only be called once.
    pub fn set_endpoints(
        ctx: Context<SetEndpoints>, 
        staking: Pubkey, 
        swap: Pubkey, 
        presale: Pubkey,
        rescue_registry: Pubkey
    ) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        
        // Check if endpoints are already set (can only set once)
        require!(!token_state.endpoints_set, TokenError::EndpointsAlreadySet);
        
        // Validate all addresses are not default
        require!(staking != Pubkey::default(), TokenError::ZeroAddress);
        require!(swap != Pubkey::default(), TokenError::ZeroAddress);
        require!(presale != Pubkey::default(), TokenError::ZeroAddress);
        require!(rescue_registry != Pubkey::default(), TokenError::ZeroAddress);
        
        // Prevent setting treasury as endpoint for security
        require!(staking != token_state.treasury, TokenError::InvalidEndpoint);
        require!(swap != token_state.treasury, TokenError::InvalidEndpoint);
        require!(presale != token_state.treasury, TokenError::InvalidEndpoint);
        require!(rescue_registry != token_state.treasury, TokenError::InvalidEndpoint);
        
        // Validate addresses are different from each other
        require!(staking != swap, TokenError::DuplicateEndpoints);
        require!(staking != presale, TokenError::DuplicateEndpoints);
        require!(staking != rescue_registry, TokenError::DuplicateEndpoints);
        require!(swap != presale, TokenError::DuplicateEndpoints);
        require!(swap != rescue_registry, TokenError::DuplicateEndpoints);
        require!(presale != rescue_registry, TokenError::DuplicateEndpoints);
        
        token_state.staking = staking;
        token_state.swap = swap;
        token_state.presale = presale;
        token_state.rescue_registry = rescue_registry;
        token_state.endpoints_set = true;
        
        emit!(EndpointsSet {
            staking,
            swap,
            presale,
            rescue_registry,
        });
        
        Ok(())
    }

    /// Transfers tokens subject to restriction rules. Allowed transfers:
    /// - Treasury → endpoints (staking/swap/presale) for distribution
    /// - Endpoints (staking/swap) → any address (claims/unstaking/distribution)
    /// - Any address → endpoints (deposits/staking)
    /// - Presale distribution: Treasury → buyer when called by presale contract
    /// - Transfers initiated by the rescue registry
    pub fn transfer_restricted(ctx: Context<TransferRestricted>, amount: u64) -> Result<()> {
        require!(amount > 0, TokenError::InvalidAmount);
        
        let token_state = &ctx.accounts.token_state;
        let from_owner = ctx.accounts.from_token.owner;
        let to_owner = ctx.accounts.to_token.owner;
        let caller = ctx.accounts.authority.key();
        
        // Validate token accounts belong to the same mint
        require!(ctx.accounts.from_token.mint == token_state.mint, TokenError::InvalidMint);
        require!(ctx.accounts.to_token.mint == token_state.mint, TokenError::InvalidMint);
        
        // Validate authority matches the from token account owner
        require!(caller == from_owner, TokenError::UnauthorizedAuthority);
        
        // Check sufficient balance
        require!(ctx.accounts.from_token.amount >= amount, TokenError::InsufficientBalance);
        
        // Define endpoint checks
        let to_endpoint = to_owner == token_state.staking || 
                         to_owner == token_state.swap || 
                         to_owner == token_state.presale;
        
        let from_endpoint = from_owner == token_state.staking || 
                           from_owner == token_state.swap;
        
        let treasury_to_endpoint = from_owner == token_state.treasury && to_endpoint;
        
        // Special case: presale distribution (Treasury → buyer via presale contract)
        let presale_distribution = caller == token_state.presale && from_owner == token_state.treasury;
        
        // Check for rescue operations
        let mut rescue_move = false;
        if token_state.rescue_registry != Pubkey::default() {
            rescue_move = caller == token_state.rescue_registry;
        }
        
        // Allow transfer if any of these conditions are met:
        // 1. Treasury → endpoint (distribution)
        // 2. Endpoint → any (claims/unstaking)
        // 3. Any → endpoint (deposits/staking)
        // 4. Presale distribution (Treasury → buyer via presale)
        // 5. Rescue operation
        require!(
            treasury_to_endpoint || from_endpoint || to_endpoint || presale_distribution || rescue_move,
            TokenError::TransferNotAllowed
        );
        
        // Perform transfer via SPL token program
        let cpi_accounts = Transfer {
            from: ctx.accounts.from_token.to_account_info(),
            to: ctx.accounts.to_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        ), amount)?;
        
        emit!(TokenTransferred {
            from: from_owner,
            to: to_owner,
            amount,
            caller,
        });
        
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

#[account]
pub struct TokenState {
    pub mint: Pubkey,              // 32
    pub treasury: Pubkey,          // 32
    pub staking: Pubkey,           // 32
    pub swap: Pubkey,              // 32
    pub presale: Pubkey,           // 32
    pub rescue_registry: Pubkey,   // 32
    pub endpoints_set: bool,       // 1
    pub bump: u8,                  // 1
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
    #[account(
        seeds = [b"token", treasury.key().as_ref()], 
        bump,
        constraint = mint_authority.key() == mint.mint_authority.unwrap() @ TokenError::InvalidPDA
    )]
    pub mint_authority: UncheckedAccount<'info>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 32 + 32 + 32 + 1 + 1,
        seeds = [b"token", treasury.key().as_ref()],
        bump
    )]
    pub token_state: Account<'info, TokenState>,
    
    /// CHECK: Treasury that receives the initial supply
    #[account(
        constraint = treasury.key() == treasury_token.owner @ TokenError::InvalidTreasury
    )]
    pub treasury: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetEndpoints<'info> {
    #[account(
        mut, 
        has_one = treasury @ TokenError::UnauthorizedAuthority,
        constraint = !token_state.endpoints_set @ TokenError::EndpointsAlreadySet
    )]
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
    
    #[account(
        constraint = authority.key() == from_token.owner @ TokenError::UnauthorizedAuthority
    )]
    pub authority: Signer<'info>,
    
    #[account(
        constraint = token_state.mint == from_token.mint @ TokenError::InvalidMint,
        constraint = token_state.mint == to_token.mint @ TokenError::InvalidMint
    )]
    pub token_state: Account<'info, TokenState>,
    
    pub token_program: Program<'info, Token>,
}

// -----------------------------------------------------------------------------
// Events

#[event]
pub struct TokenInitialized {
    pub mint: Pubkey,
    pub treasury: Pubkey,
    pub total_supply: u64,
}

#[event]
pub struct EndpointsSet {
    pub staking: Pubkey,
    pub swap: Pubkey,
    pub presale: Pubkey,
    pub rescue_registry: Pubkey,
}

#[event]
pub struct TokenTransferred {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub caller: Pubkey,
}

// -----------------------------------------------------------------------------
// Errors

#[error_code]
pub enum TokenError {
    #[msg("Invalid supply amount")]
    InvalidSupply,
    
    #[msg("Invalid PDA derivation")]
    InvalidPDA,
    
    #[msg("Invalid bump seed")]
    InvalidBump,
    
    #[msg("Zero address not allowed")]
    ZeroAddress,
    
    #[msg("Invalid endpoint - cannot be treasury")]
    InvalidEndpoint,
    
    #[msg("Duplicate endpoints not allowed")]
    DuplicateEndpoints,
    
    #[msg("Endpoints already set - can only be set once")]
    EndpointsAlreadySet,
    
    #[msg("Invalid amount - must be greater than 0")]
    InvalidAmount,
    
    #[msg("Invalid mint - token accounts must use correct mint")]
    InvalidMint,
    
    #[msg("Unauthorized authority - caller must own the from account")]
    UnauthorizedAuthority,
    
    #[msg("Insufficient balance")]
    InsufficientBalance,
    
    #[msg("Transfer not allowed - restricted transfers only")]
    TransferNotAllowed,
    
    #[msg("Invalid treasury")]
    InvalidTreasury,
}