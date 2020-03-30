use solana_sdk::{hash::hashv, instruction::Instruction, message::Message, pubkey::Pubkey};
use solana_stake_program::{
    stake_instruction,
    stake_state::{Authorized, Lockup, StakeAuthorize},
};

pub const MAX_SEED_LEN: usize = 32;

#[derive(Debug)]
pub enum PubkeyError {
    MaxSeedLengthExceeded,
}

// TODO: Once solana-1.1 is released, use `Pubkey::create_with_seed`.
fn create_with_seed(base: &Pubkey, seed: &str, program_id: &Pubkey) -> Result<Pubkey, PubkeyError> {
    if seed.len() > MAX_SEED_LEN {
        return Err(PubkeyError::MaxSeedLengthExceeded);
    }

    Ok(Pubkey::new(
        hashv(&[base.as_ref(), seed.as_ref(), program_id.as_ref()]).as_ref(),
    ))
}

pub(crate) fn derive_stake_account_address(base_pubkey: &Pubkey, i: usize) -> Pubkey {
    create_with_seed(base_pubkey, &i.to_string(), &solana_stake_program::id()).unwrap()
}

// Return derived addresses
pub(crate) fn derive_stake_account_addresses(
    base_pubkey: &Pubkey,
    num_accounts: usize,
) -> Vec<Pubkey> {
    (0..num_accounts)
        .map(|i| derive_stake_account_address(base_pubkey, i))
        .collect()
}

pub(crate) fn new_stake_account(
    fee_payer_pubkey: &Pubkey,
    sender_pubkey: &Pubkey,
    base_pubkey: &Pubkey,
    lamports: u64,
    stake_authority_pubkey: &Pubkey,
    withdraw_authority_pubkey: &Pubkey,
) -> Message {
    let seed = 0;
    let stake_account_address = derive_stake_account_address(base_pubkey, seed);
    let authorized = Authorized {
        staker: *stake_authority_pubkey,
        withdrawer: *withdraw_authority_pubkey,
    };
    let instructions = stake_instruction::create_account_with_seed(
        sender_pubkey,
        &stake_account_address,
        &base_pubkey,
        &seed.to_string(),
        &authorized,
        &Lockup::default(),
        lamports,
    );
    Message::new_with_payer(&instructions, Some(fee_payer_pubkey))
}

fn authorize_stake_accounts_instructions(
    stake_account_address: &Pubkey,
    stake_authority_pubkey: &Pubkey,
    withdraw_authority_pubkey: &Pubkey,
    new_stake_authority_pubkey: &Pubkey,
    new_withdraw_authority_pubkey: &Pubkey,
) -> Vec<Instruction> {
    let instruction0 = stake_instruction::authorize(
        &stake_account_address,
        stake_authority_pubkey,
        new_stake_authority_pubkey,
        StakeAuthorize::Staker,
    );
    let instruction1 = stake_instruction::authorize(
        &stake_account_address,
        withdraw_authority_pubkey,
        new_withdraw_authority_pubkey,
        StakeAuthorize::Withdrawer,
    );
    vec![instruction0, instruction1]
}

fn move_stake_account(
    stake_account_address: &Pubkey,
    new_base_pubkey: &Pubkey,
    i: usize,
    fee_payer_pubkey: &Pubkey,
    stake_authority_pubkey: &Pubkey,
    withdraw_authority_pubkey: &Pubkey,
    new_stake_authority_pubkey: &Pubkey,
    new_withdraw_authority_pubkey: &Pubkey,
    lamports: u64,
) -> Message {
    let new_stake_account_address = derive_stake_account_address(new_base_pubkey, i);
    let mut instructions = stake_instruction::split_with_seed(
        stake_account_address,
        stake_authority_pubkey,
        lamports,
        &new_stake_account_address,
        new_stake_authority_pubkey,
        &i.to_string(),
    );

    let authorize_instructions = authorize_stake_accounts_instructions(
        &new_stake_account_address,
        stake_authority_pubkey,
        withdraw_authority_pubkey,
        new_stake_authority_pubkey,
        new_withdraw_authority_pubkey,
    );

    instructions.extend(authorize_instructions.into_iter());
    Message::new_with_payer(&instructions, Some(&fee_payer_pubkey))
}

pub(crate) fn authorize_stake_accounts(
    fee_payer_pubkey: &Pubkey,
    base_pubkey: &Pubkey,
    stake_authority_pubkey: &Pubkey,
    withdraw_authority_pubkey: &Pubkey,
    new_stake_authority_pubkey: &Pubkey,
    new_withdraw_authority_pubkey: &Pubkey,
    num_accounts: usize,
) -> Vec<Message> {
    let stake_account_addresses = derive_stake_account_addresses(base_pubkey, num_accounts);
    stake_account_addresses
        .iter()
        .map(|stake_account_address| {
            let instructions = authorize_stake_accounts_instructions(
                stake_account_address,
                stake_authority_pubkey,
                withdraw_authority_pubkey,
                new_stake_authority_pubkey,
                new_withdraw_authority_pubkey,
            );
            Message::new_with_payer(&instructions, Some(&fee_payer_pubkey))
        })
        .collect::<Vec<_>>()
}

pub(crate) fn move_stake_accounts(
    fee_payer_pubkey: &Pubkey,
    new_base_pubkey: &Pubkey,
    stake_authority_pubkey: &Pubkey,
    withdraw_authority_pubkey: &Pubkey,
    new_stake_authority_pubkey: &Pubkey,
    new_withdraw_authority_pubkey: &Pubkey,
    balances: &[(Pubkey, u64)],
) -> Vec<Message> {
    balances
        .iter()
        .enumerate()
        .map(|(i, (stake_account_address, lamports))| {
            move_stake_account(
                stake_account_address,
                new_base_pubkey,
                i,
                fee_payer_pubkey,
                stake_authority_pubkey,
                withdraw_authority_pubkey,
                new_stake_authority_pubkey,
                new_withdraw_authority_pubkey,
                *lamports,
            )
        })
        .collect()
}
