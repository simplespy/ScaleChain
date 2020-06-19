use super::hash::{H256};
use super::block::{Block};
use std::sync::{Mutex, Arc};
use std::collections::{HashMap};
use super::cmtda::H256 as CMTH256;
use chain::block::Block as SBlock;
use super::network::message::{ChunkReply};



pub struct BlockDb {
    pub block_db: HashMap<H256, Block>, //curr_hash -> Block  not used
    pub cmt_db: HashMap<usize, Vec<ChunkReply> >,
    pub sblock_db: HashMap<usize, SBlock>,
}

impl BlockDb {
    pub fn new() -> BlockDb{
        BlockDb {
            block_db: HashMap::new(), 
            cmt_db: HashMap::new(),
            sblock_db: HashMap::new(),
        }  
    }
    
    pub fn insert(&mut self, block: &Block) {
        let hash = block.header.hash;
        let old_value = self.block_db.insert(hash.clone(), block.clone()); 
        match old_value {
            Some(v) => println!("key {:?}, v {:?}, new {:?}", hash, v, block),
            None => (),
        }
    }

    pub fn insert_sblock(&mut self, block_id: usize, sblock: SBlock){
        self.sblock_db.insert(block_id, sblock);
    }

    pub fn insert_cmt_sample(&mut self, block_id: usize , chunk: &ChunkReply) {
        match self.cmt_db.get_mut(&block_id) {
            Some(chunks) => chunks.push(chunk.clone()),
            None => {
                self.cmt_db.insert(block_id, vec![chunk.clone()]);
            }
        }
    }

    pub fn get_chunk(&self, block_id: usize) -> Vec<ChunkReply> {
         match self.cmt_db.get(&block_id) {
            Some(chunks) => return chunks.clone(),
            None => vec![],
        }
    }

    pub fn replace(&mut self, blocks: Vec<Block>) {
        self.block_db.clear();
        for block in blocks {
            self.insert(&block);
        }
    }

    pub fn get_num_blocks(&self) -> usize {
        self.block_db.len()
    }

    pub fn get_block(&self, block_hash: H256) -> Option<Block> {
        match self.block_db.get(&block_hash) {
            Some(block) => return Some(block.clone()),
            None => return None,
        }
    }
}
