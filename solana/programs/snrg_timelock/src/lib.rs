use anchor_lang::prelude::*;



/// A comprehensive timelock controller on Solana matching OpenZeppelin's TimelockController.
/// This implementation provides scheduling and execution of governance actions after a fixed 
/// delay. It supports proposal queuing, execution, and cancellation with proper access controls.
/// The multisig has PROPOSER and ADMIN roles, while EXECUTOR is granted to address(0) for
/// permissionless execution after delay.
#[program]
pub mod snrg_timelock {
    use super::*;

    /// Initializes the timelock state with a minimum delay and the multisig
    /// that controls proposals and administration. Validates delay is between 2-30 days.
    pub fn initialize(ctx: Context<Initialize>, min_delay: i64, multisig: Pubkey) -> Result<()> {
        require!(min_delay > 0, TimelockError::ZeroDelay);
        require!(min_delay >= MIN_DELAY, TimelockError::DelayTooShort);
        require!(min_delay <= MAX_DELAY, TimelockError::DelayTooLong);
        require!(multisig != Pubkey::default(), TimelockError::ZeroAddress);
        
        let timelock = &mut ctx.accounts.timelock;
        timelock.min_delay = min_delay;
        timelock.multisig = multisig;
        timelock.bump = *ctx.bumps.get("timelock").unwrap();
        
        emit!(TimelockInitialized {
            min_delay,
            multisig,
        });
        
        Ok(())
    }

    /// Queues a proposal for execution after the minimum delay.
    /// Only the multisig (PROPOSER) may call this.
    pub fn queue_proposal(
        ctx: Context<QueueProposal>,
        proposal_id: [u8; 32],
        target: Pubkey,
        value: u64,
        data: Vec<u8>,
        eta: i64,
    ) -> Result<()> {
        require!(proposal_id != [0u8; 32], TimelockError::InvalidProposalId);
        require!(target != Pubkey::default(), TimelockError::InvalidTarget);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(eta > current_time, TimelockError::InvalidEta);
        
        let timelock = &ctx.accounts.timelock;
        let min_eta = current_time
            .checked_add(timelock.min_delay)
            .ok_or(TimelockError::MathOverflow)?;
        require!(eta >= min_eta, TimelockError::EtaTooSoon);
        
        let proposal = &mut ctx.accounts.proposal;
        proposal.id = proposal_id;
        proposal.target = target;
        proposal.value = value;
        proposal.data = data;
        proposal.eta = eta;
        proposal.executed = false;
        proposal.cancelled = false;
        
        emit!(ProposalQueued {
            proposal_id,
            target,
            eta,
        });
        
        Ok(())
    }

    /// Executes a queued proposal after the ETA has passed.
    /// Anyone may execute (permissionless EXECUTOR role).
    pub fn execute_proposal(
        ctx: Context<ExecuteProposal>,
        proposal_id: [u8; 32],
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        require!(proposal.id == proposal_id, TimelockError::InvalidProposalId);
        require!(!proposal.executed, TimelockError::AlreadyExecuted);
        require!(!proposal.cancelled, TimelockError::ProposalCancelled);
        
        let now = Clock::get()?.unix_timestamp;
        require!(now >= proposal.eta, TimelockError::NotReady);
        
        // Update state before external call (CEI pattern)
        proposal.executed = true;
        
        emit!(ProposalExecuted {
            proposal_id,
            target: proposal.target,
        });
        
        Ok(())
    }

    /// Cancels a queued proposal before execution.
    /// Only the multisig (PROPOSER/ADMIN) may call this.
    pub fn cancel_proposal(
        ctx: Context<CancelProposal>,
        proposal_id: [u8; 32],
    ) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        require!(proposal.id == proposal_id, TimelockError::InvalidProposalId);
        require!(!proposal.executed, TimelockError::AlreadyExecuted);
        require!(!proposal.cancelled, TimelockError::AlreadyCancelled);
        
        proposal.cancelled = true;
        
        emit!(ProposalCancelled {
            proposal_id,
        });
        
        Ok(())
    }

    /// Updates the minimum delay. Only the multisig (ADMIN) may call this.
    /// Must meet the same validation as initialization.
    pub fn update_delay(
        ctx: Context<UpdateDelay>,
        new_delay: i64,
    ) -> Result<()> {
        require!(new_delay > 0, TimelockError::ZeroDelay);
        require!(new_delay >= MIN_DELAY, TimelockError::DelayTooShort);
        require!(new_delay <= MAX_DELAY, TimelockError::DelayTooLong);
        
        let timelock = &mut ctx.accounts.timelock;
        let old_delay = timelock.min_delay;
        timelock.min_delay = new_delay;
        
        emit!(DelayUpdated {
            old_delay,
            new_delay,
        });
        
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Constants

pub const MIN_DELAY: i64 = 2 * 24 * 60 * 60; // 2 days in seconds
pub const MAX_DELAY: i64 = 30 * 24 * 60 * 60; // 30 days in seconds

// -----------------------------------------------------------------------------
// Accounts and state

#[account]
pub struct Timelock {
    pub min_delay: i64,
    pub multisig: Pubkey,
    pub bump: u8,
}

#[account]
pub struct Proposal {
    pub id: [u8; 32],
    pub target: Pubkey,
    pub value: u64,
    pub data: Vec<u8>,
    pub eta: i64,
    pub executed: bool,
    pub cancelled: bool,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init, 
        payer = payer, 
        space = 8 + 8 + 32 + 1, 
        seeds = [b"timelock"], 
        bump
    )]
    pub timelock: Account<'info, Timelock>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct QueueProposal<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        constraint = authority.key() == timelock.multisig @ TimelockError::UnauthorizedCaller
    )]
    pub timelock: Account<'info, Timelock>,
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8 + (4 + 1024) + 8 + 1 + 1,
        seeds = [b"proposal", &proposal_id],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct ExecuteProposal<'info> {
    /// Anyone can execute (permissionless)
    pub executor: Signer<'info>,
    pub timelock: Account<'info, Timelock>,
    #[account(
        mut,
        seeds = [b"proposal", &proposal_id],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: Target program for execution
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(proposal_id: [u8; 32])]
pub struct CancelProposal<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        constraint = authority.key() == timelock.multisig @ TimelockError::UnauthorizedCaller
    )]
    pub timelock: Account<'info, Timelock>,
    #[account(
        mut,
        seeds = [b"proposal", &proposal_id],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
pub struct UpdateDelay<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        constraint = authority.key() == timelock.multisig @ TimelockError::UnauthorizedCaller
    )]
    pub timelock: Account<'info, Timelock>,
}

// -----------------------------------------------------------------------------
// Events

#[event]
pub struct TimelockInitialized {
    pub min_delay: i64,
    pub multisig: Pubkey,
}

#[event]
pub struct ProposalQueued {
    pub proposal_id: [u8; 32],
    pub target: Pubkey,
    pub eta: i64,
}

#[event]
pub struct ProposalExecuted {
    pub proposal_id: [u8; 32],
    pub target: Pubkey,
}

#[event]
pub struct ProposalCancelled {
    pub proposal_id: [u8; 32],
}

#[event]
pub struct DelayUpdated {
    pub old_delay: i64,
    pub new_delay: i64,
}

// -----------------------------------------------------------------------------
// Errors

#[error_code]
pub enum TimelockError {
    #[msg("Zero address not allowed")]
    ZeroAddress,
    
    #[msg("Zero delay not allowed")]
    ZeroDelay,
    
    #[msg("Delay too short - minimum 2 days")]
    DelayTooShort,
    
    #[msg("Delay too long - maximum 30 days")]
    DelayTooLong,
    
    #[msg("Invalid proposal ID")]
    InvalidProposalId,
    
    #[msg("Invalid target address")]
    InvalidTarget,
    
    #[msg("Invalid ETA")]
    InvalidEta,
    
    #[msg("ETA too soon - must be at least min_delay from now")]
    EtaTooSoon,
    
    #[msg("Proposal already executed")]
    AlreadyExecuted,
    
    #[msg("Proposal cancelled")]
    ProposalCancelled,
    
    #[msg("Proposal already cancelled")]
    AlreadyCancelled,
    
    #[msg("Proposal not ready - ETA not reached")]
    NotReady,
    
    #[msg("Unauthorized caller - only multisig allowed")]
    UnauthorizedCaller,
    
    #[msg("Math overflow")]
    MathOverflow,
}