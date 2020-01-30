use super::hash::{H256};
use super::block::{Block};
use std::sync::{Mutex, Arc};
use std::collections::{HashMap};

pub struct BlockDb {
    block_db: HashMap<H256, Block>, //curr_hash -> Block  
}

impl BlockDb {
    pub fn new() -> BlockDb{
        BlockDb {
            block_db: HashMap::new(), 
        }  
    }
    
    pub fn insert(&mut self, block: &Block) {
        let hash = block.header.hash;
        self.block_db.insert(hash, block.clone()); 
    }

    pub fn get_block(&self, block_hash: H256) -> Option<Block> {
        match self.block_db.get(&block_hash) {
            Some(block) => return Some(block.clone()),
            None => return None,
        }
    }
}
