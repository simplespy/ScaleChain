use std::collections::{HashMap};
use super::hash::{H256};
use super::block::{Block, Transaction, Header};
use super::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self};
use std::collections::{HashSet, VecDeque};
use super::contract::interface::{Message, Handle, Answer};
use web3::types::{TransactionReceipt};
use super::contract::interface::Response as ContractResponse;
use crossbeam::channel::{self, Sender};

pub struct Mempool {
    transactions: VecDeque<Transaction>,
    block_size: usize,
    contract_handler: Sender<Handle>,
    returned_blocks: VecDeque<Block>,
}

impl Mempool {
    pub fn new(
        contract_handler: Sender<Handle>
    ) -> Mempool {
        Mempool {
            transactions: VecDeque::new(), 
            block_size: 1,
            contract_handler: contract_handler,
            returned_blocks: VecDeque::new(),
        } 
    }

    pub fn change_mempool_size(&mut self, size: usize) {
        self.block_size = size;
        if self.transactions.len() >= self.block_size {
            self.send_block();
        }
    }

    pub fn get_num_transaction(&self) -> usize {
        return self.transactions.len();
    }

    // TODO change the height in the header, but leave to future when tx has meaning
    pub fn return_block(&mut self, block: Block) {
        self.returned_blocks.push_back(block);
    }

    fn prepare_block(&mut self) -> Block {
        let mut transactions: Vec<Transaction> = Vec::new();
        assert!(self.transactions.len() >= self.block_size);
        for _ in 0..self.block_size {
            let tx = self.transactions.pop_front().expect("mempool prepare block");
            transactions.push(tx);
        }

        Block {
            header: Header::default(),
            transactions: transactions,
        }

    }

    pub fn send_block(&mut self) {
        // resend those blocks
        while self.returned_blocks.len() != 0 {
            match self.returned_blocks.pop_front() {
                Some(block) => {
                    self._send_block(block);
                },
                None => (),
            }
        }
        
        let mut block = self.prepare_block();
        block.update_root();
        block.update_hash();

        // send block to ethereum network
        self._send_block(block);
    }

    fn _send_block(&self, block: Block) {
        let message = Message::SendBlock(block);
        let handle = Handle {
            message: message,
            answer_channel: None,
        };
        self.contract_handler.send(handle); 
    }

    pub fn insert(&mut self, transaction: Transaction) {
        self.transactions.push_back(transaction);
        if self.transactions.len() >= self.block_size {
            self.send_block();
        }
    }

    pub fn estimate_gas(&mut self, transaction: Transaction) {
        self.transactions.push_back(transaction);
        if self.transactions.len() >= self.block_size {
            let mut block = self.prepare_block();
            block.update_root();
            block.update_hash();
            let message = Message::EstimateGas(block);
            let handle = Handle {
                message: message,
                answer_channel: None,
            };
            self.contract_handler.send(handle);
        }
    }
}

