use super::GENESIS;
use super::hash::{H256};
use super::fork::{ForkBuffer};
use super::block::{Header};
use std::collections::{HashMap};

pub struct BlockChain {
    pub height: usize,
    pub by_height_chain: HashMap<usize, H256>, 
    pub by_hash_chain: HashMap<H256, usize>, 
    pub latest_hash: H256,
    pub fork_buffer: ForkBuffer,

}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain {
            height: 0,
            by_height_chain: HashMap::new(),
            by_hash_chain: HashMap::new(),
            latest_hash: GENESIS,
            fork_buffer: ForkBuffer::new(),

        } 
    }

    // return if inserted to the blockchain or buffered
    pub fn insert(&mut self, header: &Header) -> bool {
        let hash = header.hash;

        if header.prev_hash == self.latest_hash {
            let height = &mut self.height;
            self.by_height_chain.insert(*height, hash);
            self.by_hash_chain.insert(hash, *height);
            *height += 1;
            self.latest_hash = hash;
            println!("main chain");
            return true;
        } else {
            println!("side chain");
            match self.by_hash_chain.get(&hash) {
                Some(contact_height) => {
                    if (header.height != contact_height + 1) {
                        println!("height inconsistant") ;
                        return false;
                    }
                    self.fork_buffer.record_contact_height(hash, header.height);
                },
                _ => (),
            }
            self.fork_buffer.insert(&header, header.height); 
            match self.fork_buffer.get_longest_chain_by_hash(self.height) {
                None => false,
                Some((contact_height, switch_chain)) => {
                    panic!("NNNNNeed to reorder");     
                }
            }
        }
    }

    pub fn get_height(&self) -> usize {
        self.height 
    }

    pub fn get_latest_block_hash(&self) -> H256 {
        self.latest_hash 
    }
}
