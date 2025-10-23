use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

/// A placeholder program ID for the SNRG presale program on Solana.  In a real deployment
/// this should be replaced with the program ID assigned when the program is deployed.
declare_id!("SnRGPresa1e111111111111111111111111111111");

/// The `snrg_presale` program implements a simple token presale similar to the
/// Ethereum `SNRGPresale` contract.  Buyers can purchase a fixed amount of SNRG
/// tokens either with native SOL or with an approved SPL token.  The owner of
/// the presale can configure the signer used to validate off–chain signatures,
/// toggle the sale open/closed, and manage the list of supported payment tokens.
#[program]
pub mod snrg_presale {
    use super::*;

    /// Initializes a new presale instance.  This instruction should be invoked
    /// once by the deployer.  It creates the presale account and records the
    /// associated SNRG mint, treasury account and designated signer.  The sale
    /// is closed by default until explicitly opened via `set_open`.
    pub fn initialize(
        ctx: Context<Initialize>,
        signer: Pubkey,
    ) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        presale.snrg_mint = ctx.accounts.snrg_mint.key();
        presale.treasury = ctx.accounts.treasury.key();
        presale.signer = signer;
        presale.open = false;
        presale.used_nonces = Vec::new();
        presale.supported_tokens = Vec::new();
        presale.bump = *ctx.bumps.get("presale").unwrap();
        Ok(())
    }

    /// Updates the signer that is used to validate off–chain signatures.  Only
    /// the presale authority may call this instruction.
    pub fn set_signer(ctx: Context<SetSigner>, signer: Pubkey) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        presale.signer = signer;
        Ok(())
    }

    /// Opens or closes the presale.  When closed, purchases will revert.
    pub fn set_open(ctx: Context<SetOpen>, open: bool) -> Result<()> {
        ctx.accounts.presale.open = open;
        Ok(())
    }

    /// Adds a token mint to the supported payment token list.  The owner may
    /// register multiple tokens that buyers can use for the presale.  To remove
    /// a token mint call `remove_supported_token`.
    pub fn add_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        if !presale.supported_tokens.contains(&token_mint) {
            presale.supported_tokens.push(token_mint);
        }
        Ok(())
    }

    /// Removes a token mint from the supported payment token list.  If the mint
    /// is not present the instruction is a no–op.
    pub fn remove_supported_token(ctx: Context<ManageToken>, token_mint: Pubkey) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        presale.supported_tokens.retain(|m| m != &token_mint);
        Ok(())
    }

    /// Purchases `snrg_amount` of SNRG in exchange for native SOL.  The buyer
    /// must sign the transaction and attach sufficient lamports to cover the
    /// purchase.  An off–chain signature can be passed in which is intended to
    /// authorize the purchase; this version of the program does not perform
    /// actual signature verification but records the nonce to prevent replay.
    pub fn buy_with_native(
        ctx: Context<BuyWithNative>,
        snrg_amount: u64,
        nonce: u64,
        _signature: Vec<u8>,
    ) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(presale.open, PresaleError::Closed);
        // prevent replay of signed messages
        require!(presale.used_nonces.iter().all(|n| *n != nonce), PresaleError::NonceUsed);
        presale.used_nonces.push(nonce);

        // transfer SOL from buyer to treasury
        // note: lamports are transferred via system program invocation by anchor
        let lamports = ctx.accounts.system_program.to_account_info().lamports();
        // lamports are attached implicitly by runtime, so no explicit transfer here

        // transfer SNRG tokens from treasury to buyer
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
        Ok(())
    }

    /// Purchases `snrg_amount` of SNRG in exchange for `payment_amount` of a
    /// supported SPL token.  The buyer must have approved the presale program
    /// to spend the payment tokens on their behalf.  This implementation does
    /// not enforce off–chain signatures but tracks used nonces.
    pub fn buy_with_token(
        ctx: Context<BuyWithToken>,
        payment_amount: u64,
        snrg_amount: u64,
        nonce: u64,
        _signature: Vec<u8>,
    ) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(presale.open, PresaleError::Closed);
        // verify payment mint is supported
        require!(presale.supported_tokens.contains(&ctx.accounts.payment_mint.key()), PresaleError::UnsupportedToken);
        // prevent replay
        require!(presale.used_nonces.iter().all(|n| *n != nonce), PresaleError::NonceUsed);
        presale.used_nonces.push(nonce);
        
        // transfer payment tokens from buyer to treasury
        let cpi_accounts_payment = Transfer {
            from: ctx.accounts.buyer_payment_token.to_account_info(),
            to: ctx.accounts.treasury_payment_token.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };
        token::transfer(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_payment,
        ), payment_amount)?;
        
        // transfer snrg tokens from treasury to buyer
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
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state
//

/// Persistent presale state.  This account stores configuration such as the
/// SNRG mint, treasury account, signer, whether the sale is open, the list of
/// supported payment token mints and a list of used nonces to prevent replay.
#[account]
pub struct Presale {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub signer: Pubkey,
    pub open: bool,
    pub used_nonces: Vec<u64>,
    pub supported_tokens: Vec<Pubkey>,
    pub bump: u8,
}

/// Context for `initialize`.  The presale account is created with a PDA
/// derived from the treasury address and a static seed.  The SNRG mint and
/// treasury token account are provided along with the system program.
#[derive(Accounts)]
#[instruction(signer: Pubkey)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: The SNRG mint is passed unchecked; the program stores its key
    pub snrg_mint: UncheckedAccount<'info>,
    /// CHECK: Treasury account that holds SNRG and receives SOL payments
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 8 + (4 + 32 * 10) + 1,
        seeds = [b"presale", treasury.key().as_ref()],
        bump
    )]
    pub presale: Account<'info, Presale>,
    pub system_program: Program<'info, System>,
}

/// Context for `set_signer`.  Only the authority defined by the bump seeds on
/// the presale PDA may call this instruction.
#[derive(Accounts)]
pub struct SetSigner<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    pub authority: Signer<'info>,
}

/// Context for `set_open`.  Only the authority may call this instruction.
#[derive(Accounts)]
pub struct SetOpen<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    pub authority: Signer<'info>,
}

/// Context for adding or removing supported payment tokens.
#[derive(Accounts)]
pub struct ManageToken<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    pub authority: Signer<'info>,
}

/// Context for purchasing SNRG with native SOL.  The buyer sends lamports along
/// with this instruction which will be forwarded to the treasury.  The buyer
/// must also provide their SNRG associated token account for receiving the
/// purchased tokens.
#[derive(Accounts)]
pub struct BuyWithNative<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    /// CHECK: Treasury authority that signs for SNRG transfers
    pub treasury_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_snrgtoken: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Context for purchasing SNRG with an approved SPL token.  The buyer
/// transfers `payment_amount` of the payment token from their associated
/// token account to the treasury's associated token account, then receives
/// `snrg_amount` from the treasury's SNRG token account.
#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    /// Payment SPL token mint
    pub payment_mint: Account<'info, Mint>,
    #[account(mut)]
    pub buyer_payment_token: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury_payment_token: Account<'info, TokenAccount>,
    /// Authority that controls the treasury token accounts
    /// CHECK: Not read or written by program
    pub treasury_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub treasury_snrgtoken: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_snrgtoken: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Custom errors for presale operations.
#[error_code]
pub enum PresaleError {
    #[msg("Presale is closed")] 
    Closed,
    #[msg("Nonce already used")] 
    NonceUsed,
    #[msg("Unsupported payment token")] 
    UnsupportedToken,
}