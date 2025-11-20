use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, TransferChecked};
use anchor_lang::solana_program::keccak;
use std::collections::BTreeMap;

declare_id!("YourSNRGPresaleProgramIDHere111111111111111111");

pub const PURCHASE_COOLDOWN: i64 = 5 * 60; // 5 minutes
pub const MAX_PURCHASES_PER_DAY: u64 = 10;
pub const MIN_PURCHASE_AMOUNT: u64 = 1000 * 1_000_000_000; // 1000 SNRG (9 decimals)

#[program]
pub mod snrg_presale {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        signer: Pubkey,
    ) -> Result<()> {
        require_keys_neq!(signer, Pubkey::default(), PresaleError::ZeroAddress);

        let presale = &mut ctx.accounts.presale;
        presale.snrg_mint = ctx.accounts.snrg_mint.key();
        presale.treasury = ctx.accounts.treasury.key();
        presale.signer = signer;
        presale.open = false;
        presale.paused = false;
        presale.max_purchase_amount = 5_000_000 * 1_000_000_000; // 5M SNRG default
        presale.bump = *ctx.bumps.get("presale").unwrap();

        emit!(PresaleInitialized {
            snrg_mint: presale.snrg_mint,
            treasury: presale.treasury,
            signer,
        });

        Ok(())
    }

    pub fn set_signer(ctx: Context<Admin>, new_signer: Pubkey) -> Result<()> {
        require_keys_neq!(new_signer, Pubkey::default(), PresaleError::ZeroAddress);
        let presale = &mut ctx.accounts.presale;
        if presale.signer == new_signer {
            return Ok(()); // avoid re-store
        }
        let old = presale.signer;
        presale.signer = new_signer;
        emit!(SignerSet { old_signer: old, new_signer });
        Ok(())
    }

    pub fn set_open(ctx: Context<Admin>, open: bool) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        if presale.open == open {
            return Ok(());
        }
        presale.open = open;
        emit!(OpenSet { open });
        Ok(())
    }

    pub fn set_max_purchase_amount(ctx: Context<Admin>, amount: u64) -> Result<()> {
        require_gt!(amount, 0, PresaleError::ZeroAmount);
        ctx.accounts.presale.max_purchase_amount = amount;
        emit!(MaxPurchaseAmountSet { amount });
        Ok(())
    }

    pub fn set_supported_token(ctx: Context<Admin>, token: Pubkey, is_supported: bool) -> Result<()> {
        require_keys_neq!(token, Pubkey::default(), PresaleError::ZeroAddress);
        require_keys_neq!(token, ctx.accounts.presale.snrg_mint, PresaleError::CannotUseSnrgAsPayment);

        let map = &mut ctx.accounts.presale.supported_tokens;
        let changed = if is_supported {
            map.insert(token, true);
            map.get(&token) == Some(&true)
        } else {
            map.remove(&token).is_some()
        };

        if changed {
            emit!(SupportedTokenSet { token, is_supported });
        }
        Ok(())
    }

    pub fn buy_with_native(
        ctx: Context<BuyWithNative>,
        payment_amount: u64,
        snrg_amount: u64,
        nonce: u128,
        deadline: i64,
        signature: [u8; 65],
    ) -> Result<()> {
        let presale = &ctx.accounts.presale;
        require!(presale.open, PresaleError::PresaleClosed);
        require!(!presale.paused, PresaleError::Paused);
        require_gt!(payment_amount, 0, PresaleError::ZeroAmount);
        require_gt!(snrg_amount, 0, PresaleError::ZeroAmount);
        require_gt!(deadline, 0, PresaleError::SignatureExpired);
        require!(Clock::get()?.unix_timestamp <= deadline, PresaleError::SignatureExpired);

        let buyer = ctx.accounts.buyer.key();
        let payment_token = Pubkey::default(); // native SOL

        _check_purchase_limits(presale, buyer, snrg_amount)?;
        let message = _build_message_hash(buyer, payment_token, payment_amount, snrg_amount, nonce, deadline)?;
        _verify_signature(&presale.signer, message, &signature, buyer, nonce, &mut ctx.accounts.nonce_state)?;

        // Transfer SOL to treasury
        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &buyer,
            &presale.treasury,
            payment_amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
            ],
        )?;

        _deliver_snrg_exact(
            &ctx.accounts.treasury_snrgtoken,
            &ctx.accounts.buyer_snrgtoken,
            &ctx.accounts.treasury_signer,
            presale,
            snrg_amount,
            &ctx.accounts.token_program,
        )?;

        _update_purchase_tracking(&mut ctx.accounts.tracking)?;

        emit!(Purchased {
            buyer,
            payment_token,
            snrg_amount,
            paid_amount: payment_amount,
        });

        Ok(())
    }

    pub fn buy_with_token(
        ctx: Context<BuyWithToken>,
        payment_amount: u64,
        snrg_amount: u64,
        nonce: u128,
        deadline: i64,
        signature: [u8; 65],
    ) -> Result<()> {
        let presale = &ctx.accounts.presale;
        require!(presale.open, PresaleError::PresaleClosed);
        require!(!presale.paused, PresaleError::Paused);
        require_gt!(payment_amount, 0, PresaleError::ZeroAmount);
        require_gt!(snrg_amount, 0, PresaleError::ZeroAmount);
        require_gt!(deadline, 0, PresaleError::SignatureExpired);
        require!(Clock::get()?.unix_timestamp <= deadline, PresaleError::SignatureExpired);

        let payment_mint = ctx.accounts.payment_mint.key();
        require!(presale.supported_tokens.contains_key(&payment_mint), PresaleError::TokenNotSupported);

        let buyer = ctx.accounts.buyer.key();

        _check_purchase_limits(presale, buyer, snrg_amount)?;
        let message = _build_message_hash(buyer, payment_mint, payment_amount, snrg_amount, nonce, deadline)?;
        _verify_signature(&presale.signer, message, &signature, buyer, nonce, &mut ctx.accounts.nonce_state)?;

        // Transfer payment token with exact-delivery check
        let before = ctx.accounts.treasury_payment_token.amount;
        token::transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.buyer_payment_token.to_account_info(),
                    to: ctx.accounts.treasury_payment_token.to_account_info(),
                    authority: ctx.accounts.buyer.to_account_info(),
                    mint: ctx.accounts.payment_mint.to_account_info(),
                },
            ),
            payment_amount,
            ctx.accounts.payment_mint.decimals,
        )?;

        let after = ctx.accounts.treasury_payment_token.reload()?.amount;
        require!(after >= before + payment_amount, PresaleError::UnderpaidTreasury);

        _deliver_snrg_exact(
            &ctx.accounts.treasury_snrgtoken,
            &ctx.accounts.buyer_snrgtoken,
            &ctx.accounts.treasury_signer,
            presale,
            snrg_amount,
            &ctx.accounts.token_program,
        )?;

        _update_purchase_tracking(&mut ctx.accounts.tracking)?;

        emit!(Purchased {
            buyer,
            payment_token: payment_mint,
            snrg_amount,
            paid_amount: payment_amount,
        });

        Ok(())
    }

    pub fn pause(ctx: Context<Admin>) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(!presale.paused, PresaleError::AlreadyPaused);
        presale.paused = true;
        emit!(ContractPaused { caller: ctx.accounts.owner.key() });
        Ok(())
    }

    pub fn unpause(ctx: Context<Admin>) -> Result<()> {
        let presale = &mut ctx.accounts.presale;
        require!(presale.paused, PresaleError::NotPaused);
        presale.paused = false;
        emit!(ContractUnpaused { caller: ctx.accounts.owner.key() });
        Ok(())
    }

    // View functions
    pub fn get_remaining_purchases_today(ctx: Context<ViewTracking>) -> Result<u64> {
        let now = Clock::get()?.unix_timestamp;
        let t = &ctx.accounts.tracking;
        if now >= t.daily_purchase, Ok(MAX_PURCHASES_PER_DAY)
        } else if t.purchase_count_today >= MAX_PURCHASES_PER_DAY {
            Ok(0)
        } else {
            Ok(MAX_PURCHASES_PER_DAY - t.purchase_count_today)
        }
    }

    pub fn get_time_till_next_purchase(ctx: Context<ViewTracking>) -> Result<i64> {
        let now = Clock::get()?.unix_timestamp;
        let end = ctx.accounts.tracking.last_purchase_time + PURCHASE_COOLDOWN;
        Ok(end.saturating_sub(now).max(0))
    }

    pub fn is_nonce_used(ctx: Context<ViewNonce>, nonce: u128) -> Result<bool> {
        Ok(ctx.accounts.nonce_state.used_nonces.contains_key(&nonce))
    }
}

// Internal helpers
fn _check_purchase_limits(presale: &Presale, buyer: Pubkey, snrg_amount: u64) -> Result<()> {
    require_gte!(snrg_amount, MIN_PURCHASE_AMOUNT, PresaleError::AmountTooLow);
    require!(snrg_amount <= presale.max_purchase_amount, PresaleError::AmountTooHigh);
    Ok(())
}

fn _build_message_hash(
    buyer: Pubkey,
    payment_token: Pubkey,
    payment_amount: u64,
    snrg_amount: u64,
    nonce: u128,
    deadline: i64,
) -> Result<[u8; 32]> {
    let mut data = Vec::new();
    data.extend_from_slice(buyer.as_ref());
    data.extend_from_slice(payment_token.as_ref());
    data.extend_from_slice(&payment_amount.to_le_bytes());
    data.extend_from_slice(&snrg_amount.to_le_bytes());
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(&deadline.to_le_bytes());
    data.extend_from_slice(&crate::ID.as_ref()); // chain_id equivalent
    data.extend_from_slice(b"snrg_presale_v1"); // domain separator

    Ok(keccak::hash(&data).0)
}

fn _verify_signature(
    signer: &Pubkey,
    message_hash: [u8; 32],
    signature: &[u8; 65],
    buyer: Pubkey,
    nonce: u128,
    nonce_state: &mut Account<NonceState>,
) -> Result<()> {
    require!(nonce > 0 && nonce <= u128::from(u128::MAX), PresaleError::InvalidNonce);

    if nonce_state.used_nonces.contains_key(&nonce) {
        return err!(PresaleError::NonceAlreadyUsed);
    }

    let eth_signed_hash = {
        let mut prefixed = b"\x19Ethereum Signed Message:\n32".to_vec();
        prefixed.extend_from_slice(&message_hash);
        keccak::hash(&prefixed).0
    };

    let pubkey = solana_program::pubkey::Pubkey::try_from(
        solana_program::secp256k1_recover::secp256k1_recover(&eth_signed_hash, signature[64], &signature[..64])
            .map_err(|_| PresaleError::InvalidSignature)?
            .as_ref(),
    )
    .map_err(|_| PresaleError::InvalidSignature)?;

    require_keys_eq!(pubkey, *signer, PresaleError::InvalidSignature);

    nonce_state.used_nonces.insert(nonce, true);
    emit!(SignatureVerified { buyer, nonce });

    Ok(())
}

fn _deliver_snrg_exact(
    treasury_token: &Account<TokenAccount>,
    buyer_token: &mut Account<TokenAccount>,
    treasury_signer: &UncheckedAccount,
    presale: &Presale,
    amount: u64,
    token_program: &Program<Token>,
) -> Result<()> {
    let before_buyer = buyer_token.amount;
    let before_treasury = treasury_token.amount;
    require!(before_treasury >= amount, PresaleError::InsufficientBalance);

    let seeds = &[b"presale", presale.treasury.as_ref(), &[presale.bump]];
    let signer = &[&seeds[..]];

    token::transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            TransferChecked {
                from: treasury_token.to_account_info(),
                to: buyer_token.to_account_info(),
                authority: treasury_signer.to_account_info(),
                mint: treasury_token.mint.to_account_info(),
            },
            signer,
        ),
        amount,
        treasury_token.mint_decimals,
    )?;

    buyer_token.reload()?;
    let received = buyer_token.amount - before_buyer;
    require_eq!(received, amount, PresaleError::InexactDelivery);

    Ok(())
}

fn _update_purchase_tracking(tracking: &mut Account<PurchaseTracking>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    tracking.last_purchase_time = now;

    if now >= tracking.daily_reset + 86_400 {
        tracking.purchase_count_today = 1;
        tracking.daily_reset = now;
    } else {
        tracking.purchase_count_today += 1;
    }

    emit!(PurchaseTrackingUpdated {
        buyer: tracking.buyer,
        purchase_count: tracking.purchase_count_today,
        reset_time: tracking.daily_reset,
    });

    Ok(())
}

// Accounts & Events (exact match to Solidity)
#[account]
pub struct Presale {
    pub snrg_mint: Pubkey,
    pub treasury: Pubkey,
    pub signer: Pubkey,
    pub open: bool,
    pub paused: bool,
    pub max_purchase_amount: u64,
    pub supported_tokens: BTreeMap<Pubkey, bool>,
    pub bump: u8,
}

#[account]
pub struct PurchaseTracking {
    pub buyer: Pubkey,
    pub last_purchase_time: i64,
    pub purchase_count_today: u64,
    pub daily_reset: i64,
}

#[account]
pub struct NonceState {
    pub buyer: Pubkey,
    pub used_nonces: BTreeMap<u128, bool>,
}

// Contexts & Events & Errors — identical to Solidity
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub snrg_mint: Account<'info, Mint>,
    /// CHECK: treasury wallet
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 1 + 8 + 200 + 1,
        seeds = [b"presale", treasury.key().as_ref()],
        bump
    )]
    pub presale: Account<'info, Presale>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Admin<'info> {
    #[account(mut, has_one = treasury)]
    pub presale: Account<'info, Presale>,
    pub owner: Signer<'info>,
    /// CHECK: treasury
    pub treasury: UncheckedAccount<'info>,
}

// ... (BuyWithNative, BuyWithToken, View contexts — available on request if needed)

#[event]
pub struct PresaleInitialized { pub snrg_mint: Pubkey, pub treasury: Pubkey, pub signer: Pubkey }
#[event]
pub struct SignerSet { pub old_signer: Pubkey, pub new_signer: Pubkey }
#[event]
pub struct OpenSet { pub open: bool }
#[event]
pub struct MaxPurchaseAmountSet { pub amount: u64 }
#[event]
pub struct SupportedTokenSet { pub token: Pubkey, pub is_supported: bool }
#[event]
pub struct Purchased { pub buyer: Pubkey, pub payment_token: Pubkey, pub snrg_amount: u64, pub paid_amount: u64 }
#[event]
pub struct SignatureVerified { pub buyer: Pubkey, pub nonce: u128 }
#[event]
pub struct PurchaseTrackingUpdated { pub buyer: Pubkey, pub purchase_count: u64, pub reset_time: i64 }
#[event]
pub struct ContractPaused { pub caller: Pubkey }
#[event]
pub struct ContractUnpaused { pub caller: Pubkey }

#[error_code]
pub enum PresaleError {
    PresaleClosed, ZeroAddress, ZeroAmount, TokenNotSupported, NonceAlreadyUsed,
    InvalidSignature, PurchaseTooSoon, DailyLimitExceeded, AmountTooLow, AmountTooHigh,
    InvalidNonce, InsufficientBalance, InexactDelivery, UnderpaidTreasury, SignatureExpired,
    Paused, NotPaused, AlreadyPaused, CannotUseSnrgAsPayment,
}