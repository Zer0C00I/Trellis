use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod family_vault {
    use super::*;
}
