use std::collections::{HashMap};
use super::hash::{H256};
use super::block::{Block, Transaction, Header};
use super::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self};
use std::collections::{HashSet};
use super::contract::interface::{Message, Handle, Answer};
use web3::types::{TransactionReceipt};
use super::contract::interface::Response as ContractResponse;
use crossbeam::channel::{self, Sender};

pub struct Mempool {
    block: Block,
    block_size: usize,
    contract_handler: Sender<Handle>
}

impl Mempool {
    pub fn new(
        contract_handler: Sender<Handle>
    ) -> Mempool {
        Mempool {
            block: Block::default(), 
            block_size: 1,
            contract_handler: contract_handler,
        } 
    }

    pub fn change_mempool_size(&mut self, size: usize) {
        self.block_size = size;
        if self.block.len() >= self.block_size {
            self.send_block();
        }
    }

    pub fn get_num_transaction(&self) -> usize {
        return self.block.transactions.len();
    }

    pub fn send_block(&mut self) {
        // update block header

        //let blockchain = self.blockchain.lock().unwrap();
        // update merkle root TODO
        //let header = Header {
        //    hash: H256::default(),
        //    nonce: H256::new(),
        //    height: blockchain.height,
        //    root: H256::default(),
        //    prev_hash: blockchain.latest_hash,
        //};
        //drop(blockchain);
        //self.block.header = header;

        self.block.update_root();

        self.block.update_hash();

        // send block to ethereum network
        let message = Message::SendBlock(self.block.ser());
        let (answer_tx, answer_rx) = channel::bounded(1);
        let handle = Handle {
            message: message,
            answer_channel: Some(answer_tx),
        };
        self.contract_handler.send(handle);
        let receipt = match answer_rx.recv() {
            Ok(answer) => {
                match answer {
                    Answer::Success(response) => {
                        match response {
                            ContractResponse::TxReceipt(receipt) => receipt,
                            _ => {
                                panic!("answer to SendBlock: invalid response type");
                            },
                        }
                    },
                    Answer::Fail(reason) => {
                        println!("contract query fails {:?}", reason);
                        return;
                    },
                }
            },
            Err(e) => {
                println!("contract channel broken");
                return;
            },
        };
        // send Block success
        if (!receipt.status.unwrap().is_zero()) {
            println!("Receipt: {:#?}", receipt);
            self.block.clear();
            //TODO: broadcast block
        }
        else {
            println!("sendBlock Failed");
        }
    }

    pub fn insert(&mut self, transaction: Transaction) {
        self.block.insert(transaction);
        if self.block.len() >= self.block_size {
            self.send_block();
        }
    }

    fn get_block_header(&self) {
        unimplemented!(); 
    } 
}

