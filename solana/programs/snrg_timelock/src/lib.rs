use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;

declare_id!("ReplaceWithYourActualProgramID11111111111111111111");

pub const MIN_DELAY: i64 = 2 * 24 * 60 * 60;  // 2 days
pub const MAX_DELAY: i64 = 30 * 24 * 60 * 60; // 30 days

#[program]
pub mod snrg_timelock {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, min_delay: i64) -> Result<()>  {
        require!(min_delay >= MIN_DELAY, TimelockError::DelayTooShort);
        require!(min_delay <= MAX_DELAY, TimelockError::DelayTooLong);

        let timelock = &mut ctx.accounts.timelock;
        timelock.multisig = ctx.accounts.multisig.key();
        timelock.min_delay = min_delay;
        timelock.bump = ctx.bumps.timelock;

        emit!(TimelockDeployed {
            multisig: timelock.multisig,
            min_delay,
        });

        Ok(())
    }

    /// PROPOSER (multisig only) — schedule operation
    pub fn schedule(
        ctx: Context<Schedule>,
        proposal_id: [u8; 32],
        target: Pubkey,
        value: u64,
        data: Vec<u8>,
        predecessor: [u8; 32],
        delay: i64,
    ) -> Result<()> {
        require_neq!(proposal_id, [0u8; 32], TimelockError::InvalidProposalId);

        let now = Clock::get()?.unix_timestamp;
        let eta = now.checked_add(delay).ok_or(TimelockError::MathOverflow)?;
        require!(delay >= ctx.accounts.timelock.min_delay, TimelockError::InsufficientDelay);

        // Optional predecessor check (OpenZeppelin allows dependency chain)
        if predecessor != [0u8; 32] {
            let pred = ctx.accounts.predecessor.as_ref();
            if let Some(p) = pred {
                require!(p.executed, TimelockError::PredecessorNotExecuted);
            }
        }

        let proposal = &mut ctx.accounts.proposal;
        require!(!proposal.scheduled, TimelockError::AlreadyScheduled);

        proposal.proposer = ctx.accounts.authority.key();
        proposal.target = target;
        proposal.value = value;
        proposal.data = data;
        proposal.eta = eta;
        proposal.scheduled = true;
        proposal.executed = false;
        proposal.cancelled = false;

        emit!(CallScheduled {
            id: proposal_id,
            index: 0, // Solana doesn't use index
            target,
            value,
            data: proposal.data.clone(),
            predecessor,
            delay,
        });

        Ok(())
    }

    /// Anyone (EXECUTOR = address(0))
    pub fn execute(
        ctx: Context<Execute>,
        proposal_id: [u8; 32],
        predecessor: [u8; 32],
        _unused: Vec<u8>, // kept for signature compatibility
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        require!(proposal.scheduled, TimelockError::NotScheduled);
        require!(!proposal.executed, TimelockError::AlreadyExecuted);
        require!(!proposal.cancelled, TimelockError::Cancelled);

        let now = Clock::get()?.unix_timestamp;
        require!(now >= proposal.eta, TimelockError::TimelockNotFinished);

        proposal.executed = true;

        let ix = Instruction {
            program_id: proposal.target,
            accounts: ctx.remaining_accounts.to_vec(),
            data: proposal.data.clone(),
        };

        solana_program::program::invoke_signed(
            &ix,
            &[ctx.accounts.target_program.to_account_info()],
            &[],
        )?;

        emit!(CallExecuted {
            id: proposal_id,
            index: 0,
            target: proposal.target,
            value: proposal.value,
            data: proposal.data.clone(),
        });

        Ok(())
    }

    /// PROPOSER or ADMIN
    pub fn cancel(ctx: Context<Cancel>, proposal_id: [u8; 32]) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        require!(proposal.scheduled, TimelockError::NotScheduled);
        require!(!proposal.cancelled, TimelockError::AlreadyCancelled);

        proposal.cancelled = true;

        emit!(Cancelled { id: proposal_id });
        Ok(())
    }

    /// ADMIN only — update min delay
    pub fn update_delay(ctx: Context<UpdateDelay>, new_delay: i64) -> Result<()> {
        require!(new_delay >= MIN_DELAY, TimelockError::DelayTooShort);
        require!(new_delay <= MAX_DELAY, TimelockError::DelayTooLong);

        let timelock = &mut ctx.accounts.timelock;
        let old = timelock.min_delay;
        timelock.min_delay = new_delay;

        emit!(MinDelayChange { old_delay: old, new_delay });
        Ok(())
    }
}

// Accounts
#[account]
pub struct Timelock {
    pub multisig: Pubkey,
    pub min_delay: i64,
    pub bump: u8,
}

#[account]
pub struct Proposal {
    pub proposer: Pubkey,
    pub target: Pubkey,
    pub value: u64,
    pub data: Vec<u8>,
    pub eta: i64,
    pub scheduled: bool,
    pub executed: bool,
    pub cancelled: bool,
}

// Contexts
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: multisig (granted PROPOSER + ADMIN)
    pub multisig: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 8 + 1,
        seeds = [b"timelock"],
        bump
    )]
    pub timelock: Account<'info, Timelock>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct Schedule<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(constraint = authority.key() == timelock.multisig)]
    pub timelock: Account<'info, Timelock>,
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + 32 + 32 + 8 + (4 + 1024) + 8 + 1 + 1 + 1,
        seeds = [b"proposal", proposal_id.as_ref()],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    /// Optional predecessor
    pub predecessor: Option<Account<'info, Proposal>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct Execute<'info> {
    pub executor: Signer<'info>, // anyone
    pub timelock: Account<'info, Timelock>,
    #[account(mut, seeds = [b"proposal", proposal_id.as_ref()], bump)]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: target program
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct Cancel<'info> {
    #[account()]
    pub authority: Signer<'info>,
    #[account(constraint = authority.key() == timelock.multisig)]
    pub timelock: Account<'info, Timelock>,
    #[account(mut, seeds = [b"proposal", proposal_id.as_ref()], bump)]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
pub struct UpdateDelay<'info> {
    #[account()]
    pub authority: Signer<'info>,
    #[account(mut, constraint = authority.key() == timelock.multisig)]
    pub timelock: Account<'info, Timelock>,
}

// Events — 1:1 with OpenZeppelin
#[event]
pub struct TimelockDeployed { pub multisig: Pubkey, pub min_delay: i64 }

#[event]
pub struct CallScheduled {
    pub id: [u8; 32],
    pub index: u64,
    pub target: Pubkey,
    pub value: u64,
    pub data: Vec<u8>,
    pub predecessor: [u8; 32],
    pub delay: i64,
}

#[event]
pub struct CallExecuted {
    pub id: [u8; 32],
    pub index: u64,
    pub target: Pubkey,
    pub value: u64,
    pub data: Vec<u8>,
}

#[event]
pub struct Cancelled { pub id: [u8; 32] }

#[event]
pub struct MinDelayChange { pub old_delay: i64, pub new_delay: i64 }

// Errors — exact match
#[error_code]
pub enum TimelockError {
    DelayTooShort,
    DelayTooLong,
    InvalidProposalId,
    InsufficientDelay,
    AlreadyScheduled,
    AlreadyExecuted,
    Cancelled,
    NotScheduled,
    TimelockNotFinished,
    PredecessorNotExecuted,
    MathOverflow,
}
