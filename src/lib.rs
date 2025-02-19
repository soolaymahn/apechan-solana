use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program_error::ProgramError,
    sysvar::{rent::Rent, Sysvar},
    program_pack::{Pack, IsInitialized},
    system_instruction,
};
use spl_token::state::Account as TokenAccount;
use borsh::{BorshDeserialize, BorshSerialize};

// Define the state struct for our message board info
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MessageBoardInfo {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub token: Pubkey,
    pub url: String,
}

// Program entrypoint
entrypoint!(process_instruction);

// Program logic
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Deserialize instruction data
    let instruction = MessageBoardInstruction::unpack(instruction_data)?;

    match instruction {
        MessageBoardInstruction::CreateBoard { token, url } => {
            create_board(program_id, accounts, token, url)
        }
    }
}

// Instruction enum
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum MessageBoardInstruction {
    CreateBoard {
        token: Pubkey,
        url: String,
    },
}

impl MessageBoardInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match variant {
            0 => Self::CreateBoard {
                token: Pubkey::new(&rest[..32]),
                url: String::from_utf8(rest[32..].to_vec()).map_err(|_| ProgramError::InvalidInstructionData)?,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

fn create_board(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token: Pubkey,
    url: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let sender = next_account_info(account_info_iter)?;
    let board_account = next_account_info(account_info_iter)?;
    let token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let rent_sysvar_account = next_account_info(account_info_iter)?;

    // Check if the sender is the signer
    if !sender.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate that the sender owns the token account
    if token_account.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let token_account_data = TokenAccount::unpack(&token_account.data.borrow())?;
    if token_account_data.owner != *sender.key || token_account_data.mint != token || token_account_data.amount == 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the board account
    let rent = Rent::from_account_info(rent_sysvar_account)?;
    let space = std::mem::size_of::<MessageBoardInfo>();
    let lamports = rent.minimum_balance(space);

    // Create account instruction
    let create_account_ix = system_instruction::create_account(
        sender.key,
        board_account.key,
        lamports,
        space as u64,
        program_id,
    );

    // Execute create account instruction
    solana_program::program::invoke_signed(
        &create_account_ix,
        &[sender.clone(), board_account.clone(), system_program.clone()],
        &[],
    )?;

    // Initialize the board account data
    let mut board_info = MessageBoardInfo {
        is_initialized: true,
        owner: *sender.key,
        token,
        url,
    };

    board_info.serialize(&mut &mut board_account.data.borrow_mut()[..])?;

    msg!("Message board created for token: {:?}", token);
    Ok(())
}