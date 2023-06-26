use anchor_spl::{
    token,
    token::{Mint, Token, TokenAccount, Transfer},
};

use anchor_lang::{
    prelude::Pubkey,
    prelude::*,
    solana_program::{
        clock::Clock, entrypoint::ProgramResult, hash::hash, program::invoke,
        system_instruction::transfer,
    },
};

declare_id!("4PhKq1ovKkPYhdBxbeMrQWoBKHbpgCJsqzHiTuyGQjrT");
pub const POOL_AUTHORITY_SEED: &[u8] = b"pool_authority";
pub const NAME_MAX_LEN: usize = 32;

#[program]
pub mod swap {

    use super::*;
    pub const CONSTANT_PRICE: u64 = 10;
    // token x: sol, token y: move
    pub fn initialize_pool(
        ctx: Context<Initialize>,
        name_pool: String,
        token_x_amount: u64,
        token_y_amount: u64,
    ) -> Result<()> {
        let exchange = &mut ctx.accounts.exchange;

        exchange.provider = ctx.accounts.owner.key();
        exchange.name_pool = name_pool;

        let balance_x_token = ctx.accounts.user_token_x.amount;
        let balance_y_token = ctx.accounts.user_token_y.amount;

        // check pool not exist

        // check balance
        require!(
            token_x_amount <= balance_x_token,
            ErrorCode::NotEnoughBalance
        );
        require!(
            token_y_amount <= balance_y_token,
            ErrorCode::NotEnoughBalance
        );
        // deposit token x amount to reserve token x
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_x.to_account_info(),
                    to: ctx.accounts.reserve_token_x.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            token_x_amount,
        );
        // deposit token y amount to reserve token y
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_y.to_account_info(),
                    to: ctx.accounts.reserve_token_y.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            token_y_amount,
        );

        Ok(())
    }

    // if route 0: SOL-> MOVE, Route 1: MOVE-> SOL
    pub fn swap(ctx: Context<Swap>, amount_in: u64, route: u64, name_pool: String) -> Result<()> {
        require!(amount_in > 0, ErrorCode::AmountTooLow);
        let exchange = &mut ctx.accounts.exchange;

        let bump = *ctx.bumps.get("pool_authority").unwrap();
        let config_key = ctx.accounts.exchange.key();
        let pda_sign = &[
            POOL_AUTHORITY_SEED.as_ref(),
            name_seed(&name_pool),
            config_key.as_ref(),
            &[bump],
        ];

        // calculate amount out || Type Curve: Constant Price
        let mut amount_out: u64;
        if route == 0 {
            //SOL-> MOVE
            require!(
                amount_in <= ctx.accounts.reserve_token_y.amount,
                ErrorCode::NotEnoughLiquidity
            );
            // Send amount to reserve
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_x.to_account_info(),
                        to: ctx.accounts.reserve_token_x.to_account_info(),
                        authority: ctx.accounts.pool_authority.to_account_info(),
                    },
                ),
                amount_in,
            );

            amount_out = amount_in.checked_mul(CONSTANT_PRICE).unwrap();
            // send reserve to amount
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.reserve_token_y.to_account_info(),
                        to: ctx.accounts.user_token_y.to_account_info(),
                        authority: ctx.accounts.pool_authority.to_account_info(),
                    },
                )
                .with_signer(&[pda_sign]),
                amount_out,
            );
        } else {
            //MOVE-> SOL
            require!(
                amount_in <= ctx.accounts.reserve_token_x.amount,
                ErrorCode::NotEnoughLiquidity
            );
            // Send amount to reserve
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_y.to_account_info(),
                        to: ctx.accounts.reserve_token_y.to_account_info(),
                        authority: ctx.accounts.pool_authority.to_account_info(),
                    },
                ),
                amount_in,
            );

            amount_out = amount_in.checked_div(CONSTANT_PRICE).unwrap();
            // send reserve to amount
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.reserve_token_x.to_account_info(),
                        to: ctx.accounts.user_token_x.to_account_info(),
                        authority: ctx.accounts.pool_authority.to_account_info(),
                    },
                )
                .with_signer(&[pda_sign]),
                amount_out,
            );
        }

        Ok(())
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(signer)]
    pub exchange: Account<'info, Exchange>,

    pub pool_authority: AccountInfo<'info>,

    #[account(mut)]
    pub user_token_x: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_y: Account<'info, TokenAccount>,

    #[account(mut)]
    pub reserve_token_x: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reserve_token_y: Account<'info, TokenAccount>,

    pub token_program: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    pub exchange: Box<Account<'info, Exchange>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub pool_authority: AccountInfo<'info>,

    #[account(mut)]
    pub user_token_x: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_token_y: Account<'info, TokenAccount>,

    #[account(mut)]
    pub reserve_token_x: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reserve_token_y: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: AccountInfo<'info>,
}

// token x: sol, token y: move
#[account]
pub struct Exchange {
    pub name_pool: String,
    pub provider: Pubkey,
    pub token_x_amount: Pubkey,
    pub token_y_amount: Pubkey,
    pub reserve_token_x: Pubkey,
    pub reserve_token_y: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("NotEnoughBalance")]
    NotEnoughBalance,
    #[msg("AmountTooLow")]
    AmountTooLow,
    #[msg("NotEnoughLiquidity")]
    NotEnoughLiquidity,
}

pub fn name_seed(name: &str) -> &[u8] {
    let b = name.as_bytes();
    if b.len() > NAME_MAX_LEN {
        &b[0..NAME_MAX_LEN]
    } else {
        b
    }
}
