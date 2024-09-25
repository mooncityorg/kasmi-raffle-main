use anchor_lang::{accounts::cpi_account::CpiAccount, prelude::*, AccountSerialize};
use anchor_spl::{
    token::{self, Token, TokenAccount, Transfer},
};
use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
use metaplex_token_metadata::state::Metadata;


pub mod account;
pub mod constants;
pub mod error;
pub mod utils;

use account::*;
use constants::*;
use error::*;
use utils::*;

declare_id!("3qJm618bPvosqjFZqMjrUYgVXNKGDPBmTVRMqHar92DK");

#[program]
pub mod raffle {
    use super::*;
    /**
     * @dev Initialize the project
     */
    pub fn initialize(ctx: Context<Initialize>, _bump: u8) -> Result<()> {
        let global_authority = &mut ctx.accounts.global_authority;
        let _collection = ctx.accounts.collection.load_init()?;
        global_authority.super_admin = ctx.accounts.admin.key();
        Ok(())
    }

    /**
     * @dev Add collections for using this platform
     * 
     */
    pub fn add_collection(ctx: Context<AddCollection>) -> Result<()> {
        let mut collection = ctx.accounts.collection.load_mut()?;
        collection.append(ctx.accounts.collection_id.key());
        Ok(())
    }

    /**
     * @dev Create new raffle with new arguements
     * @Context has admin, global_authority accounts.
     * and zero-account Raffle, owner's nft ATA and global_authority's nft ATA
     * and nft mint address
     * @param global_bump: global authority's bump
     * @param ticket_price_sol: ticket price by sol
     * @param end_timestamp: the end time of raffle
     * @param max_entrants: entrants amount to take part in this raffle
     */
    pub fn create_raffle(
        ctx: Context<CreateRaffle>,
        _global_bump: u8,
        ticket_price_sol: u64,
        end_timestamp: i64,
        max_entrants: u64,
    ) -> Result<()> {
        let mint_metadata = &mut &ctx.accounts.mint_metadata;

        msg!("Metadata Account: {:?}", ctx.accounts.mint_metadata.key());
        let (metadata, _) = Pubkey::find_program_address(
            &[
                metaplex_token_metadata::state::PREFIX.as_bytes(),
                metaplex_token_metadata::id().as_ref(),
                ctx.accounts.nft_mint_address.key().as_ref(),
            ],
            &metaplex_token_metadata::id(),
        );
        require!(
            metadata == mint_metadata.key(),
            RaffleError::InvaliedMetadata
        );

        let collection = ctx.accounts.collection.load_mut()?;

        // verify metadata is legit
        let nft_metadata = Metadata::from_account_info(mint_metadata)?;
        if let Some(creators) = nft_metadata.data.creators {
            let mut valid: u8 = 0;
            for creator in creators {
                for j in 0..collection.count  {
                    if creator.address == collection.collections[j as usize] && creator.verified == true
                    {
                        valid = 1;
                        break;
                    }
                }
                if valid == 1 {
                    break;
                }
            }
            if valid != 1 {
                return Err(error!(RaffleError::InvalidCollection));
            }
        } else {
            return Err(error!(RaffleError::MetadataCreatorParseError));
        };   

        let mut raffle = ctx.accounts.raffle.load_init()?;
        let timestamp = Clock::get()?.unix_timestamp;

        if max_entrants > 2000 {
            return Err(error!(RaffleError::MaxEntrantsTooLarge));
        }
        if timestamp + DAY > end_timestamp {
            return Err(error!(RaffleError::EndTimeError));
        }

        // Transfer NFT to the PDA
        let src_token_account_info = &mut &ctx.accounts.owner_temp_nft_account;
        let dest_token_account_info = &mut &ctx.accounts.dest_nft_token_account;
        let token_program = &mut &ctx.accounts.token_program;

        let cpi_accounts = Transfer {
            from: src_token_account_info.to_account_info().clone(),
            to: dest_token_account_info.to_account_info().clone(),
            authority: ctx.accounts.admin.to_account_info().clone(),
        };
        token::transfer(
            CpiContext::new(token_program.clone().to_account_info(), cpi_accounts),
            1,
        )?;

        raffle.creator = ctx.accounts.admin.key();
        raffle.nft_mint = ctx.accounts.nft_mint_address.key();
        raffle.ticket_price_sol = ticket_price_sol;
        raffle.start_timestamp = timestamp;
        raffle.end_timestamp = end_timestamp;
        raffle.max_entrants = max_entrants;

        Ok(())
    }

    /**
     * @dev Buy tickets functions
     * @Context has buyer and raffle's account.
     * global_authority and creator address and their reap token ATAs
     * @param global_bump: global_authority's bump
     * @param amount: the amount of the tickets
     */
    pub fn buy_tickets(ctx: Context<BuyTickets>, _global_bump: u8, amount: u64) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        let mut raffle = ctx.accounts.raffle.load_mut()?;

        if timestamp > raffle.end_timestamp {
            return Err(error!(RaffleError::RaffleEnded));
        }
        if raffle.count + amount > raffle.max_entrants {
            return Err(error!(RaffleError::NotEnoughTicketsLeft));
        }

        let total_amount_sol = amount * raffle.ticket_price_sol;

        if ctx.accounts.buyer.to_account_info().lamports() < total_amount_sol {
            return Err(error!(RaffleError::NotEnoughSOL));
        }

        // Check how many no repeat accounts bought the tickets
        if raffle.count == 0 {
            raffle.no_repeat = 1;
        } else {
            let mut index: u64 = 0;
            for i in 0..raffle.count {
                if raffle.entrants[i as usize] == ctx.accounts.buyer.key() {
                    index = i + 1 as u64;
                    break;
                }
            }
            if index == 0 {
                raffle.no_repeat += 1;
            }
        }

        for _ in 0..amount {
            raffle.append(ctx.accounts.buyer.key());
        }
        
        // Transfer SOL from the buyer to the Raffle Creator's wallet
        let creator_amount = total_amount_sol * (100 - COMMISSION_FEE) / 100;
        sol_transfer_user(
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            creator_amount,
        )?;

        // Transfer COMMISSION_FEE SOL from the buyer to the treasury wallet 
        let fee_amount = total_amount_sol * COMMISSION_FEE / 100;
        sol_transfer_user(
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.treasury_wallet.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            fee_amount,
        )?;

        Ok(())
    }

    /**
     * @dev Reaveal winner function
     * @Context has buyer and raffle account address
     */
    pub fn reveal_winner(ctx: Context<RevealWinner>) -> Result<()> {
        let mut raffle = ctx.accounts.raffle.load_mut()?;
        let timestamp = Clock::get()?.unix_timestamp;
        if timestamp < raffle.end_timestamp {
            return Err(error!(RaffleError::RaffleNotEnded));
        }
        if raffle.count == 0 {
            return Err(error!(RaffleError::InvalidRevealedData));
        }

        // Get the random number of the entrant amount
        let (player_address, _bump) = Pubkey::find_program_address(
            &[
                RANDOM_SEED.as_bytes(),
                timestamp.to_string().as_bytes(),
            ],
            &raffle::ID,
        );
        let char_vec: Vec<char> = player_address.to_string().chars().collect();
        let mut mul = 1;
        for i in 0..7 {
            mul *= u64::from(char_vec[i as usize]);
        }
        mul += u64::from(char_vec[7]);
        let winner_index = mul % raffle.count;
        raffle.winner_index = winner_index;
        raffle.winner = raffle.entrants[winner_index as usize];
        raffle.claimed = 2;
        Ok(())
    }

    /**
     * @dev Claim reward function
     * @Context has claimer and global_authority account
     * raffle account and the nft ATA of claimer and global_authority.
     * @param global_bump: the global_authority's bump
     */
    pub fn claim_reward(ctx: Context<ClaimReward>, global_bump: u8) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        let mut raffle = ctx.accounts.raffle.load_mut()?;

        if timestamp < raffle.end_timestamp {
            return Err(error!(RaffleError::RaffleNotEnded));
        }
        if raffle.winner != ctx.accounts.claimer.key() {
            return Err(error!(RaffleError::NotWinner));
        }

        // Transfer NFT to the winner's wallet
        let src_token_account = &mut &ctx.accounts.src_nft_token_account;
        let dest_token_account = &mut &ctx.accounts.claimer_nft_token_account;
        let token_program = &mut &ctx.accounts.token_program;
        let seeds = &[GLOBAL_AUTHORITY_SEED.as_bytes(), &[global_bump]];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: src_token_account.to_account_info().clone(),
            to: dest_token_account.to_account_info().clone(),
            authority: ctx.accounts.global_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                token_program.clone().to_account_info(),
                cpi_accounts,
                signer,
            ),
            1,
        )?;
        raffle.claimed = 1;
    
        Ok(())
    }
    /**
     * @dev Withdraw NFT function
     * @Context has claimer and global_authority account
     * raffle account and creator's nft ATA and global_authority's nft ATA
     * @param global_bump: global_authority's bump
     */
    pub fn withdraw_nft(ctx: Context<WithdrawNft>, global_bump: u8) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        let mut raffle = ctx.accounts.raffle.load_mut()?;

        if timestamp < raffle.start_timestamp + DAY {
            return Err(error!(RaffleError::RaffleNotEnded));
        }
        if raffle.creator != ctx.accounts.claimer.key() {
            return Err(error!(RaffleError::NotCreator));
        }
        if raffle.count != 0 {
            return Err(error!(RaffleError::OtherEntrants));
        }

        // Transfer NFT to the creator's wallet after the raffle ends or 
        // creator wants to cancel raffle because no tickets are sold
        let src_token_account = &mut &ctx.accounts.src_nft_token_account;
        let dest_token_account = &mut &ctx.accounts.claimer_nft_token_account;
        let token_program = &mut &ctx.accounts.token_program;
        let seeds = &[GLOBAL_AUTHORITY_SEED.as_bytes(), &[global_bump]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: src_token_account.to_account_info().clone(),
            to: dest_token_account.to_account_info().clone(),
            authority: ctx.accounts.global_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                token_program.clone().to_account_info(),
                cpi_accounts,
                signer,
            ),
            1,
        )?;
        raffle.claimed = 3;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
        payer = admin,
        space = GlobalPool::LEN
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(zero)]
    pub collection: AccountLoader<'info, CollectionPool>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AddCollection<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub collection: AccountLoader<'info, CollectionPool>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub collection_id: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(_global_bump: u8)]
pub struct CreateRaffle<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(zero)]
    pub raffle: AccountLoader<'info, RafflePool>,

    #[account(mut)]
    pub collection: AccountLoader<'info, CollectionPool>,

    #[account(
        mut,
        constraint = owner_temp_nft_account.mint == *nft_mint_address.to_account_info().key,
        constraint = owner_temp_nft_account.owner == *admin.key,
    )]
    pub owner_temp_nft_account: CpiAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = dest_nft_token_account.mint == *nft_mint_address.to_account_info().key,
        constraint = dest_nft_token_account.owner == *global_authority.to_account_info().key,
    )]
    pub dest_nft_token_account: CpiAccount<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint_address: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = mint_metadata.owner == &metaplex_token_metadata::ID
    )]
    pub mint_metadata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    // the token metadata program
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(constraint = token_metadata_program.key == &metaplex_token_metadata::ID)]
    pub token_metadata_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(_global_bump: u8)]
pub struct BuyTickets<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, RafflePool>,

    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
    )]
    pub global_authority: Account<'info, GlobalPool>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub creator: AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = treasury_wallet.key() == TREASURY_WALLET.parse::<Pubkey>().unwrap() 
    )]
    pub treasury_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevealWinner<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, RafflePool>,
}

#[derive(Accounts)]
#[instruction(global_bump: u8)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, RafflePool>,

    #[account(
        mut,
        constraint = claimer_nft_token_account.mint == *nft_mint_address.to_account_info().key,
        constraint = claimer_nft_token_account.owner == *claimer.key,
    )]
    pub claimer_nft_token_account: CpiAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = src_nft_token_account.mint == *nft_mint_address.to_account_info().key,
        constraint = src_nft_token_account.owner == *global_authority.to_account_info().key,
    )]
    pub src_nft_token_account: CpiAccount<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint_address: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(global_bump: u8)]
pub struct WithdrawNft<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, RafflePool>,

    #[account(
        mut,
        constraint = claimer_nft_token_account.mint == *nft_mint_address.to_account_info().key,
        constraint = claimer_nft_token_account.owner == *claimer.key,
    )]
    pub claimer_nft_token_account: CpiAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = src_nft_token_account.mint == *nft_mint_address.to_account_info().key,
        constraint = src_nft_token_account.owner == *global_authority.to_account_info().key,
    )]
    pub src_nft_token_account: CpiAccount<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint_address: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
