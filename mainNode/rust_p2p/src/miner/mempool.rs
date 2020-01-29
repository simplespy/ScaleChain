use std::collections::{HashMap};
use super::hash::{H256};
use super::block::{Block, Transaction, Header};
use super::miner::{ManagerMessage};
use super::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self};
use std::collections::{HashSet};

pub struct Mempool {
    mining_tx: HashSet<H256>,
    block: Block,
    mining_size: usize,
    miner_sender: mpsc::Sender<ManagerMessage>, 
    blockchain: Arc<Mutex<BlockChain>>,
}

impl Mempool {
    pub fn new(
        miner_sender: mpsc::Sender<ManagerMessage>, 
        blockchain: Arc<Mutex<BlockChain>>,
    ) -> Mempool {
        Mempool {
            block: Block::default(), 
            mining_tx: HashSet::new(),
            mining_size: 3,
            miner_sender: miner_sender,
            blockchain: blockchain,
        } 
    }

    pub fn insert(&mut self, transaction: Transaction) {
        self.block.insert(transaction);
        if self.block.len() >= self.mining_size {
            for tx in self.block.transactions.iter() {
                self.mining_tx.insert(tx.hash); 
            }
            let blockchain = self.blockchain.lock().unwrap();
            // update merkle root TODO
            let header = Header {
                hash: H256::default(),
                nonce: H256::new(),
                height: blockchain.height,
                root: H256::default(),
                prev_hash: blockchain.latest_hash,
            };
            drop(blockchain);
            self.block.header = header;

            self.block.update_root();

            self.block.update_hash();

            
            let miner_message = ManagerMessage::Data(self.block.clone());
            self.miner_sender.send(miner_message);
            
            self.block.clear();
        }
    }

    fn get_block_header(&self) {
        unimplemented!(); 
    } 
}

