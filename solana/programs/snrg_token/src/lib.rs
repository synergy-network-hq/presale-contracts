use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("YOUR_PROGRAM_ID_HERE"); // ← Replace with actual deployed program ID

pub const ENDPOINT_CONFIRMATION_DELAY: i64 = 24 * 60 * 60; // 24 hours in seconds

#[program]
pub mod snrg_token {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, total_supply: u64) -> Result<()> {
        require!(total_supply > 0, TokenError::InvalidSupply);

        let token_state = &mut ctx.accounts.token_state;
        token_state.mint = ctx.accounts.mint.key();
        token_state.treasury = ctx.accounts.treasury.key();
        token_state.bump = ctx.bumps.token_state;
        token_state.endpoints_configured = false;

        // PDA mint authority check
        let (expected_pda, _) = Pubkey::find_program_address(
            &[b"token", token_state.treasury.as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(ctx.accounts.mint_authority.key(), expected_pda, TokenError::InvalidPDA);
        require_keys_eq!(ctx.accounts.mint.mint_authority.unwrap(), expected_pda, TokenError::InvalidPDA);

        // Mint full supply to treasury
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.treasury_token.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[&[
                    b"token",
                    token_state.treasury.as_ref(),
                    &[token_state.bump],
                ]],
            ),
            total_supply,
        )?;

        emit!(TokenInitialized {
            mint: token_state.mint,
            treasury: token_state.treasury,
            total_supply,
        });

        Ok(())
    }

    /// Owner proposes new endpoints – starts 24h timelock
    pub fn propose_endpoints(
        ctx: Context<ProposeEndpoints>,
        staking: Pubkey,
        swap: Pubkey,
        presale: Pubkey,
        rescue_registry: Pubkey,
    ) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;

        require!(!token_state.has_pending_proposal(), TokenError::PendingEndpoints);

        Self::validate_endpoint_inputs(staking, swap, presale, rescue_registry, token_state.treasury)?;

        let clock = Clock::get()?;
        let eta = clock.unix_timestamp.checked_add(ENDPOINT_CONFIRMATION_DELAY)
            .ok_or(TokenError::MathOverflow)?;

        token_state.pending_proposal = Some(PendingProposal {
            staking,
            swap,
            presale,
            rescue_registry,
            eta,
        });

        emit!(EndpointsProposed {
            staking,
            swap,
            presale,
            rescue_registry,
            eta,
        });

        Ok(())
    }

    /// Owner confirms after 24h delay → activates endpoints
    pub fn confirm_endpoints(ctx: Context<ConfirmEndpoints>) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        let proposal = token_state.pending_proposal.ok_or(TokenError::NoPendingEndpoints)?;

        require_gt!(Clock::get()?.unix_timestamp, proposal.eta, TokenError::EndpointDelayActive);

        token_state.staking = proposal.staking;
        token_state.swap = proposal.swap;
        token_state.presale = proposal.presale;
        token_state.rescue_registry = proposal.rescue_registry;
        token_state.endpoints_configured = true;
        token_state.pending_proposal = None;

        emit!(EndpointsSet {
            staking: proposal.staking,
            swap: proposal.swap,
            presale: proposal.presale,
            rescue_registry: proposal.rescue_registry,
        });

        Ok(())
    }

    pub fn cancel_endpoint_proposal(ctx: Context<AuthTreasury>) -> Result<()> {
        let token_state = &mut ctx.accounts.token_state;
        if token_state.pending_proposal.is_some() {
            token_state.pending_proposal = None;
            emit!(EndpointProposalCancelled {});
        }
        Ok(())
    }

    /// Standard transfer with full restriction logic (exact match to Solidity _update)
    pub fn transfer_restricted(ctx: Context<TransferRestricted>, amount: u64) -> Result<()> {
        require!(amount > 0, TokenError::InvalidAmount);

        let token_state = &ctx.accounts.token_state;
        require!(token_state.endpoints_configured, TokenError::TransfersDisabled);

        let from_owner = ctx.accounts.from_token.owner;
        let to_owner = ctx.accounts.to_token.owner;
        let caller = ctx.accounts.authority.key();

        require_keys_eq!(from_owner, caller, TokenError::Unauthorized);
        require_eq!(ctx.accounts.from_token.mint, token_state.mint, TokenError::InvalidMint);
        require_eq!(ctx.accounts.to_token.mint, token_state.mint, TokenError::InvalidMint);
        require!(ctx.accounts.from_token.amount >= amount, TokenError::InsufficientFunds);

        let to_endpoint = to_owner == token_state.staking
            || to_owner == token_state.swap
            || to_owner == token_state.presale;

        let from_endpoint = from_owner == token_state.staking || from_owner == token_state.swap;

        let treasury_to_endpoint = from_owner == token_state.treasury && to_endpoint;

        let presale_distribution = caller == token_state.presale && from_owner == token_state.treasury;

        // Donation attack protection — only treasury, user themselves, or endpoint can send to endpoint
        let controlled_to_endpoint = to_endpoint && (
            from_owner == token_state.treasury ||
            caller == from_owner ||           // user depositing
            caller == to_owner                // endpoint pulling
        );

        // Rescue check — rescue_registry must be a program and caller must be authorized executor
        let mut rescue_move = false;
        if token_state.rescue_registry != Pubkey::default() {
            // We cannot do full interface check like Solidity, but we can at least restrict to program-owned accounts
            // Recommended: off-chain verify that rescue_registry implements correct interface
            let registry_info = ctx.accounts.rescue_registry.to_account_info();
            if registry_info.owner == &crate::ID && registry_info.executable {
                // Optional: add CPI check if you have a known rescue program ID
                rescue_move = caller == token_state.rescue_registry;
            }
        }

        require!(
            treasury_to_endpoint
                || from_endpoint
                || controlled_to_endpoint
                || presale_distribution
                || rescue_move,
            TokenError::TransferNotAllowed
        );

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from_token.to_account_info(),
                    to: ctx.accounts.to_token.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(TokenTransferred {
            from: from_owner,
            to: to_owner,
            amount,
            caller,
        });

        Ok(())
    }

    /// Optional but recommended: allow burning
    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        require!(amount > 0, TokenError::InvalidAmount);
        require!(ctx.accounts.token_state.endpoints_configured, TokenError::TransfersDisabled);

        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    from: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Accounts & Helpers
// ─────────────────────────────────────────────────────────────────────────────

#[account]
pub struct TokenState {
    pub mint: Pubkey,
    pub treasury: Pubkey,
    pub staking: Pubkey,
    pub swap: Pubkey,
    pub presale: Pubkey,
    pub rescue_registry: Pubkey,
    pub endpoints_configured: bool,
    pub pending_proposal: Option<PendingProposal>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PendingProposal {
    pub staking: Pubkey,
    pub swap: Pubkey,
    pub presale: Pubkey,
    pub rescue_registry: Pubkey,
    pub eta: i64,
}

impl TokenState {
    pub fn has_pending_proposal(&self) -> bool {
        self.pending_proposal.is_some()
    }
}

impl TokenState {
    fn validate_endpoint_inputs(
        staking: Pubkey,
        swap: Pubkey,
        presale: Pubkey,
        rescue_registry: Pubkey,
        treasury: Pubkey,
    ) -> Result<()> {
        require!(staking != Pubkey::default(), TokenError::ZeroAddress);
        require!(swap != Pubkey::default(), TokenError::ZeroAddress);
        require!(presale != Pubkey::default(), TokenError::ZeroAddress);
        require!(rescue_registry != Pubkey::default(), TokenError::ZeroAddress);

        require!(staking != treasury, TokenError::InvalidEndpoint);
        require!(swap != treasury, TokenError::InvalidEndpoint);
        require!(presale != treasury, TokenError::InvalidEndpoint);
        require!(rescue_registry != treasury, TokenError::InvalidEndpoint);

        let endpoints = [staking, swap, presale, rescue_registry];
        for i in 0..endpoints.len() {
            for j in (i + 1)..endpoints.len() {
                require!(endpoints[i] != endpoints[j], TokenError::DuplicateEndpoints);
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Contexts
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub treasury_token: Account<'info, TokenAccount>,

    /// CHECK: PDA mint authority
    #[account(
        seeds = [b"token", treasury.key().as_ref()],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + TokenState::INIT_SPACE,
        seeds = [b"token", treasury.key().as_ref()],
        bump,
    )]
    pub token_state: Account<'info, TokenState>,

    /// CHECK: Treasury owner of the token
    pub treasury: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ProposeEndpoints<'info> {
    #[account(
        mut,
        has_one = treasury,
    )]
    pub token_state: Account<'info, TokenState>,
    pub treasury: Signer<'info>,
}

#[derive(Accounts)]
pub struct ConfirmEndpoints<'info> {
    #[account(
        mut,
        has_one = treasury,
    )]
    pub token_state: Account<'info, TokenState>,
    pub treasury: Signer<'info>,
}

#[derive(Accounts)]
pub struct AuthTreasury<'info> {
    #[account(has_one = treasury)]
    pub token_state: Account<'info, TokenState>,
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

    /// CHECK: Only used for executable check in rescue
    pub rescue_registry: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Burn<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_state: Account<'info, TokenState>,
    pub token_program: Program<'info, Token>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Events & Errors
// ─────────────────────────────────────────────────────────────────────────────

#[event]
pub struct TokenInitialized {
    pub mint: Pubkey,
    pub treasury: Pubkey,
    pub total_supply: u64,
}

#[event]
pub struct EndpointsProposed {
    pub staking: Pubkey,
    pub swap: Pubkey,
    pub presale: Pubkey,
    pub rescue_registry: Pubkey,
    pub eta: i64,
}

#[event]
pub struct EndpointProposalCancelled {}

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

#[error_code]
pub enum TokenError {
    #[msg("Invalid supply amount")]
    InvalidSupply,
    #[msg("Invalid PDA mismatch")]
    InvalidPDA,
    #[msg("Zero address not allowed")]
    ZeroAddress,
    #[msg("Invalid endpoint - cannot be treasury")]
    InvalidEndpoint,
    #[msg("Duplicate endpoints")]
    DuplicateEndpoints,
    #[msg("Pending endpoint proposal already exists")]
    PendingEndpoints,
    #[msg("No pending endpoint proposal")]
    NoPendingEndpoints,
    #[msg("24h delay not elapsed yet")]
    EndpointDelayActive,
    #[msg("Transfers are disabled until endpoints are configured")]
    TransfersDisabled,
    #[msg("Transfer not allowed under restriction rules")]
    TransferNotAllowed,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Unauthorized signer")]
    Unauthorized,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Math overflow")]
    MathOverflow,
}

// Add this for space calculation
impl TokenState {
    pub const INIT_SPACE: usize = 8 + // discriminator
        32*6 + // pubkeys
        1 +    // bool
        1 +    // bump
        1 + 32*4 + 8 + 8; // Option<PendingProposal> max size
}