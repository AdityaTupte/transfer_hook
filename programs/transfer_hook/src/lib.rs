use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use anchor_spl::{
    associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};


declare_id!("6Gt4YXeoa6MKgn5PjMBf1sE1bh6TaaUfxWuiEj9LnzSj");


#[error_code]
pub enum ErrorCode {
    #[msg("Failed to calculate ExtraAccountMetaList size")]
    ExtraAccountMetaListSizeError,

     #[msg("Failed to initialize ExtraAccountMetaList")]
    ExtraAccountMetaListInitError,

    #[msg("Failed to unpack transfer hook instruction")]
    InstructionUnpackError,

    #[msg("math overflow")]
    Overflow,

    #[msg("math underflow")]
    Underflow,
}   


pub const PRECISION: u128 = 1_000_000_000_000; // 1e12

pub const MAIN_PROGRAM_ID: Pubkey =
    pubkey!("6Gt4YXeoa6MKgn5PjMBf1sE1bh6TaaUfxWuiEj9LnzSj");

#[program]
pub mod transfer_hook {                                                  
    use super::*;

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {

        // The `addExtraAccountsToInstruction` JS helper function resolving incorrectly
        let account_metas = vec![
            // dividend_per_token
            ExtraAccountMeta::new_with_seeds(
                &[
            Seed::Literal {
                bytes: b"dividend_per_token".to_vec(),
            },
            Seed::AccountKey {
                index: 1, // mint (based on account order in TransferHook)
                }],
                false, // is_signer
                true,  // is_writable
            ).map_err(|_| anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds))?,
            

            // reward pda for source_token
            ExtraAccountMeta::new_with_seeds(
                &[
            Seed::Literal {
                bytes: b"rewardpda".to_vec(),
            },
            Seed::AccountKey {
                index: 1, // mint (based on account order in TransferHook)
                },
            Seed::AccountKey{
                index:0,
            }],
                false, // is_signer
                true,  // is_writable
            ).map_err(|_| anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds))?,

            //reward pda for destination token

             ExtraAccountMeta::new_with_seeds(
                &[
            Seed::Literal {
                bytes: b"rewardpda".to_vec(),
            },
            Seed::AccountKey {
                index: 1, // mint (based on account order in TransferHook)
                },
            Seed::AccountKey { 
                index: 2 
            }],
                false, // is_signer
                true,  // is_writable
            ).map_err(|_| anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::ConstraintSeeds))?,

        ];

        // calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len()).map_err(|_| ErrorCode::ExtraAccountMetaListSizeError)?;
       
        // calculate minimum required lamports
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        // create ExtraAccountMetaList account
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size as u64,
            ctx.program_id,
        )?;

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        ).map_err(|_| ErrorCode::ExtraAccountMetaListInitError)?;

        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        
        

        let dividend_per_token = &mut ctx.accounts.dividend_account;

        let source_ata = &ctx.accounts.source_token;

        let destination_ata = &ctx.accounts.destination_token;

        let source_reward = &mut ctx.accounts.source_reward_pda;

        let destination_reward = &mut ctx.accounts.destination_reward_pda;

        if source_reward.key() == destination_reward.key() {
            return Ok(());
        }


        let dpt = dividend_per_token.dividend_per_token;

        let transfer_amount = amount; // passed via instruction

        let source_old_balance = source_ata.amount
            .checked_add(transfer_amount)
            .ok_or(ErrorCode::Overflow)?;

            let destination_old_balance = destination_ata.amount
            .checked_sub(transfer_amount)
            .ok_or(ErrorCode::Underflow)?;

            // 1. Calculate accumulated (OLD balance)
            let source_accumulate = (source_old_balance as u128)
                .checked_mul(dpt)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(PRECISION)
                .ok_or(ErrorCode::Overflow)?;

          


            // 2. Calculate pending
            let current_source_pending = source_accumulate
                .checked_sub(source_reward.reward_debt)
                .ok_or(ErrorCode::Underflow)?;

            // 3. Safe cast
            let pending_u64 = u64::try_from(current_source_pending)
                .map_err(|_| ErrorCode::Overflow)?;

            // 4. Accumulate rewards
            source_reward.pending_reward = source_reward.pending_reward
                .checked_add(pending_u64)
                .ok_or(ErrorCode::Overflow)?;

        
            // 5. Update reward debt (NEW balance)
            source_reward.reward_debt = (source_ata.amount as u128)
                .checked_mul(dpt)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(PRECISION)
                .ok_or(ErrorCode::Overflow)?;
            

             // 1. Calculate accumulated (OLD balance)
            let destination_accumulate = (destination_old_balance as u128)
                .checked_mul(dpt)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(PRECISION)
                .ok_or(ErrorCode::Overflow)?;

            // 2. Calculate pending
            let current_destiantion_pending = destination_accumulate
                .checked_sub(destination_reward.reward_debt)
                .ok_or(ErrorCode::Underflow)?;

            // 3. Safe cast
            let destiantion_pending_u64 = u64::try_from(current_destiantion_pending)
                .map_err(|_| ErrorCode::Overflow)?;

            // 4. Accumulate rewards
            destination_reward.pending_reward = destination_reward.pending_reward
                .checked_add(destiantion_pending_u64)
                .ok_or(ErrorCode::Overflow)?;
            

            // 5. Update reward debt (NEW balance)
            destination_reward.reward_debt = (destination_ata.amount as u128)
                .checked_mul(dpt)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(PRECISION)
                .ok_or(ErrorCode::Overflow)?;

            Ok(())
    }

    pub fn initailize(_ctx:Context<InitalizeAccounts>)->Result<()>{

        msg!("Reward account is created for the mint ");

        Ok(())


    }

    pub fn initailizediv(ctx:Context<AccountsDiv>)->Result<()>{

       let dic = &mut ctx.accounts.dividend_pda;

                // initialize only once
                if !dic.bool {
                    dic.dividend_per_token = 0;
                    dic.bool = true;
                }

                // update dividend_per_token safely
            let increment = (50 as u128)
                .checked_mul(PRECISION)
                .ok_or(ErrorCode::Overflow)?
                .checked_div(100 as u128)
                .ok_or(ErrorCode::Underflow)?;

            dic.dividend_per_token = dic.dividend_per_token
                .checked_add(increment)
                .ok_or(ErrorCode::Overflow)?;

                Ok(())


    }



    // fallback instruction handler as workaround to anchor instruction discriminator check
    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data).map_err(|_| ErrorCode::InstructionUnpackError)?;

        // match instruction discriminator to transfer hook interface execute instruction  
        // token2022 program CPIs this instruction on token transfer
        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();

                // invoke custom transfer hook instruction on our program
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Order of accounts matters for this struct.
// The first 4 accounts are the accounts required for token transfer (source, mint, destination, owner)
// Remaining accounts are the extra accounts required from the ExtraAccountMetaList account
// These accounts are provided via CPI to this program from the token2022 program
#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint, 
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        token::mint = mint,
    )]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner, can be SystemAccount or PDA owned by another program
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()], 
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"dividend_per_token",mint.key().as_ref()],
        bump,
        // seeds::program = MAIN_PROGRAM_ID,
        // owner = MAIN_PROGRAM_ID 
    )]
    pub dividend_account: Account<'info, DividendPerToken>,

    #[account(
        mut,
        seeds = [b"rewardpda",mint.key().as_ref(),source_token.key().as_ref()],
        bump, 
    )]
    pub source_reward_pda : Account<'info,RewardPda>,

    #[account(
        mut,
        seeds = [b"rewardpda",mint.key().as_ref(),destination_token.key().as_ref()],
        bump, 
    )]
    pub destination_reward_pda : Account<'info,RewardPda>,

}


#[derive(Accounts)]
pub struct InitalizeAccounts<'info>{

    #[account(
        mut,
    )]
    pub payer: Signer<'info>,

    pub mint : InterfaceAccount<'info,Mint>,
    

    #[account(
        associated_token::mint = mint,
        associated_token::authority = payer,
         associated_token::token_program = token_program,
    )]
    pub token_account : InterfaceAccount<'info,TokenAccount>,


    #[account(
        init,
        payer = payer,
        seeds=[
            b"rewardpda",
            mint.key().as_ref(),
            token_account.key().as_ref(),
        ],
        bump,
        space = 8 + 25
    )]
    pub reward_pda : Account<'info,RewardPda>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program : Program<'info,System>,

}



#[derive(Accounts)]
pub struct AccountsDiv<'info>{

    #[account(
        mut,
    )]
    pub payer: Signer<'info>,

    pub mint : InterfaceAccount<'info,Mint>,

    #[account(
        associated_token::mint = mint,
        associated_token::authority = payer,
         associated_token::token_program = token_program,
    )]
    pub token_account : InterfaceAccount<'info,TokenAccount>,


    #[account(
        init,
        payer = payer,
        seeds=[
            b"dividend_per_token",mint.key().as_ref()
        ],
        bump,
        space = 8 + 25
    )]
    pub dividend_pda : Account<'info,DividendPerToken>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program : Program<'info,System>,

}




#[account]
pub struct RewardPda {

    pub reward_debt: u128,

    pub pending_reward: u64,
}
#[account]

pub struct DividendPerToken{

    pub dividend_per_token : u128,

    pub bool : bool,

}
