use anchor_lang::prelude::*;

declare_id!("SnRGTimeLock1111111111111111111111111111");

/// A placeholder implementation of a timelock controller on Solana.  The
/// Ethereum version relies on OpenZeppelin's `TimelockController` which
/// schedules and executes governance actions after a fixed delay.  This
/// simplified version merely stores the delay and multisig authority.  It
/// could be extended to queue and execute arbitrary instructions after the
/// delay has expired.
#[program]
pub mod snrg_timelock {
    use super::*;

    /// Initializes the timelock state with a minimum delay and the multisig
    /// that controls proposals and administration.  The deployer becomes
    /// temporary admin until the multisig takes ownership.
    pub fn initialize(ctx: Context<Initialize>, min_delay: i64, multisig: Pubkey) -> Result<()> {
        require!(min_delay >= 0, TimelockError::InvalidDelay);
        let timelock = &mut ctx.accounts.timelock;
        timelock.min_delay = min_delay;
        timelock.multisig = multisig;
        timelock.bump = *ctx.bumps.get("timelock").unwrap();
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Accounts and state

/// Minimal timelock state storing the minimum execution delay and the
/// multisig that controls proposals.  More complex scheduling logic would
/// require additional accounts and instructions.
#[account]
pub struct Timelock {
    pub min_delay: i64,
    pub multisig: Pubkey,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 8 + 32 + 1, seeds = [b"timelock"], bump)]
    pub timelock: Account<'info, Timelock>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum TimelockError {
    #[msg("Invalid minimum delay")] InvalidDelay,
}