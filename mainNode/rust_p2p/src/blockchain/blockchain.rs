use super::GENESIS;
use super::hash::{H256};
use super::fork::{ForkBuffer};
use super::block::{Header};
use std::collections::{HashMap};
use super::primitive::block::{ContractState};
use super::experiment::snapshot::PERFORMANCE_COUNTER;

pub struct BlockChain {
    blockchain: Vec<ContractState>,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain {
            blockchain: Vec::new(),
        } 
    }

    // input must be consistent with previous block
    pub fn insert(&mut self, contract_state: &ContractState) {
        self.blockchain.push(contract_state.clone());
    }

    // block_id itself is changed
    pub fn revise(&mut self, block_id: usize, states: Vec<ContractState>) {

    }

    pub fn replace(&mut self, chain: Vec<ContractState>) {
        self.blockchain = chain;
        PERFORMANCE_COUNTER.store_chain_depth(self.blockchain.len());
    }

    // block id should start at 0, so is consistent with height
    pub fn get_height(&self) -> usize {
        self.blockchain.len() 
    }

    pub fn get_latest_state(&self) -> Option<ContractState> {
        match self.blockchain.last() {
            Some(c) => Some(c.clone()),
            None => None,
        }
    }
}
