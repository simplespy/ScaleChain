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
use super::scheduler::{Token};

use super::cmtda::Block as CMTBlock;
use super::cmtda::Transaction as CMTTransaction;
use super::cmtda::H256 as CMTH256;

use super::cmtda::{BlockHeader, BLOCK_SIZE, HEADER_SIZE, read_codes};
use chain::decoder::{Code, Decoder, TreeDecoder, CodingErr, IncorrectCodingProof};
use chain::decoder::{Symbol};
use ser::{deserialize, serialize};
use std::net::{SocketAddr};

pub struct Mempool {
    transactions: VecDeque<Transaction>,
    block_size: usize,
    contract_handler: Sender<Handle>,
    schedule_handler: Sender<Option<Token>>,
    returned_blocks: VecDeque<Block>,
    cmt_block: Option<CMTBlock>,
    addr: SocketAddr,
}

impl Mempool {
    pub fn new(
        contract_handler: Sender<Handle>,
        schedule_handler: Sender<Option<Token>>,
        addr: SocketAddr,
    ) -> Mempool {
        Mempool {
            transactions: VecDeque::new(), 
            block_size: 1,
            contract_handler: contract_handler,
            schedule_handler: schedule_handler,
            returned_blocks: VecDeque::new(),
            cmt_block: None,
            addr: addr,
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

    // TODO currently we ask the block to give us random chunks
    pub fn sample_cmt(&mut self, sample_idx: Vec<u32>) -> (CMTH256, Vec<Vec<Symbol>>, Vec<Vec<u64>>) {
        let mut hash = CMTH256::default();
        let (symbols, idx) = match &self.cmt_block {
            None => panic!("I don't have cmt block"),
            Some(cmt_block) => {
                info!("{:?} sample cmt of size {}", self.addr, sample_idx.len());
                hash = cmt_block.block_header.merkle_root_hash;
                let (symbols_1, idx_1) = cmt_block.sampling_to_decode(3 as u32);
                info!("idx1 {:?}", idx_1);
                info!("symbols_1 {:?}", symbols_1);
                

                let (mut symbols, mut idx) = cmt_block.sampling_to_decode(1000 as u32); //sample_idx.len()
                info!("idx {:?}", idx);
                info!("symbols {:?}", symbols);
                info!("{:?} symbols len {}", self.addr, symbols[0].len());
                info!("{:?} idx len {}", self.addr, idx[0].len());

                // since there are only one layer
                let mut j = 0;
                for i in &idx_1[0] {
                    if idx[0].contains(&i) {
                        //
                    } else {
                        idx[0].push(*i);
                        symbols[0].push(symbols_1[0][j]);
                    }
                    j += 1;
                }
                info!("new idx {:?}", idx);
                info!("new symbols {:?}", symbols);


                // tests 
                let k_set: Vec<u64> = vec![16];
                let (codes_for_encoding, codes_for_decoding) = read_codes(k_set);
                info!("{:?} after reading code, len {} {}", self.addr, codes_for_encoding.len(), codes_for_decoding.len());
                info!("{:?} before constructing tree decoder", self.addr);
                let mut decoder: TreeDecoder = TreeDecoder::new(codes_for_decoding.to_vec(), &cmt_block.block_header.coded_merkle_roots_hashes);
                info!("{:?}, treedecoder n {} height {}", self.addr, decoder.n, decoder.height);
                match decoder.run_tree_decoder(symbols.clone(), idx.clone()) {
                    Ok(()) => (),
                    _ => info!("tree decoder error"),
                };
                info!("{:?} after calling tree decoder", self.addr);
                (symbols, idx)
            }
        };

        (hash, symbols, idx)
    }

    // currently a hack, need to combine with sample_cmt
    pub fn get_cmt_header(&self) -> BlockHeader {
        match &self.cmt_block {
            None => panic!("I don't have cmt block"),
            Some(cmt_block) => cmt_block.block_header.clone(),
        }
    }

    pub fn prepare_cmt_block(&mut self) -> BlockHeader {
        // get CMT
        let header = BlockHeader {
            version: 1,
            previous_header_hash: CMTH256::default(),
            merkle_root_hash: CMTH256::default(),
            time: 4u32,
            bits: 5.into(),
            nonce: 6u32,
            coded_merkle_roots_hashes: vec![CMTH256::default(); 4],
        };
        // CMT - propose block
        let t = "0100000001a6b97044d03da79c005b20ea9c0e1a6d9dc12d9f7b91a5911c9030a439eed8f5000000004948304502206e21798a42fae0e854281abd38bacd1aeed3ee3738d9e1446618c4571d1090db022100e2ac980643b0b82c0e88ffdfec6b64e3e6ba35e7ba5fdd7d5d6cc8d25c6b241501ffffffff0100f2052a010000001976a914404371705fa9bd789a2fcd52d2c580b65d35549d88ac00000000";
        let transaction_size = String::from(t).len();
        info!("{:?} transaction_size {:?}", self.addr, transaction_size);

        let num_transactions = BLOCK_SIZE / (transaction_size as u64);
        info!("{:?} num_transactions {:?}", self.addr, num_transactions);

        let transactions: Vec<CMTTransaction> = vec![t.into();num_transactions as usize];
        
        // number of systematic symbols for the codes on the four layers of CMT
        let k_set: Vec<u64> = vec![16];
        let (codes_for_encoding, codes_for_decoding) = read_codes(k_set);
        info!("num code {:?} Code len {}", codes_for_encoding.len(), codes_for_encoding[0].symbols.len()); 
        let block: CMTBlock = CMTBlock::new(
            header.clone(), 
            &transactions, 
            BLOCK_SIZE as usize, 
            HEADER_SIZE, 
            &codes_for_encoding, 
            vec![true; codes_for_encoding.len()]);
        
        info!("{:?} after create CMT block", self.addr);
        self.cmt_block = Some(block);
        return header;
    }

    pub fn prepare_block(&mut self) -> Option<Block> {
        if self.transactions.len() == 0 {
            return None;
        }

        let mut transactions: Vec<Transaction> = Vec::new();
        //assert!(self.transactions.len() >= self.block_size);
        for _ in 0..self.block_size {
            let tx = self.transactions.pop_front().expect("mempool prepare block");
            transactions.push(tx);
        }



        return Some(Block {
            header: Header::default(),
            transactions: transactions,
        });

    }

    // desolete mempool should be called by scheduler
    pub fn send_block(&mut self) {
        // resend those blocks
        while self.returned_blocks.len() != 0 {
            match self.returned_blocks.pop_front() {
                Some(block) => {
                    //self._send_block(block);
                },
                None => (),
            }
        }
        
        let mut block = self.prepare_block().expect("send block");
        //block.update_root();
        //block.update_hash();

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
            //self.send_block();
            self.schedule_handler.send(None);
        }
    }

    pub fn estimate_gas(&mut self, transaction: Transaction) {
        self.transactions.push_back(transaction);
        if self.transactions.len() >= self.block_size {
            let mut block = self.prepare_block().expect("send block");
            //block.update_root();
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

