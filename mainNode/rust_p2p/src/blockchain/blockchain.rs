use super::GENESIS;
use super::hash::{H256};
use super::fork::{ForkBuffer};
use super::block::{Header};
use std::collections::{HashMap};
use super::contract::{ContractState};

pub struct BlockChain {
    // may potentially store history
    pub curr_state: ContractState,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain {
            curr_state: ContractState::default(),
        } 
    }

    pub fn insert(&mut self, contract_state: &ContractState) {
        self.curr_state = contract_state.clone();
    }

    pub fn get_height(&self) -> usize {
        self.curr_state.block_id 
    }

    pub fn get_latest_block_hash(&self) -> H256 {
        self.curr_state.curr_hash
    }
}
