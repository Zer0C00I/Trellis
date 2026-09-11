use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

declare_id!("6FM6U29Kz1KBjnztNJ6W1bjtQqfKTCMZruw8Rsij4C8S");

#[program]
pub mod family_vault {
    use super::*;
}
