use anchor_lang::prelude::*;

declare_id!("2E593Gj2ftX8TNThAokNL4S6hJfn6SUL2cWniie33uN9");

#[program]
pub mod test_workspace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
