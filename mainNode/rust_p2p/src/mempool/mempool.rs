use std::collections::{HashMap};
use super::hash::{H256};
use super::block::{Block, Header};
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
use super::cmtda::Transaction;
use super::cmtda::H256 as CMTH256;

use super::cmtda::{BlockHeader, BLOCK_SIZE, HEADER_SIZE, read_codes};
use chain::decoder::{Code, Decoder, TreeDecoder, CodingErr, IncorrectCodingProof};
use chain::decoder::{Symbol};
use primitives::bytes::{Bytes};
use ser::{deserialize, serialize};
use std::net::{SocketAddr};
use rand::Rng;

pub struct Mempool {
    transactions: VecDeque<Transaction>,
    block_size: usize,
    contract_handler: Sender<Handle>,
    schedule_handler: Sender<Option<Token>>,
    returned_blocks: VecDeque<Block>,
    cmt_block: Option<CMTBlock>,
    addr: SocketAddr,
    codes_for_encoding: Vec<Code>,
    codes_for_decoding: Vec<Code>,
}

impl Mempool {
    pub fn new(
        contract_handler: Sender<Handle>,
        schedule_handler: Sender<Option<Token>>,
        addr: SocketAddr,
        codes_for_encoding: Vec<Code>,
        codes_for_decoding: Vec<Code>,
    ) -> Mempool {
        
        Mempool {
            transactions: VecDeque::new(), 
            block_size: BLOCK_SIZE as usize, // in bytes
            contract_handler: contract_handler,
            schedule_handler: schedule_handler,
            returned_blocks: VecDeque::new(),
            cmt_block: None,
            addr: addr,
            codes_for_encoding: codes_for_encoding,
            codes_for_decoding: codes_for_decoding,
        } 
    }

    pub fn transaction_size_in_bytes(&self) -> usize {
        let mut trans_byte = self.transactions.iter().map(Transaction::bytes).collect::<Vec<Bytes>>();
        let mut total_size = 0;
        for tx in &trans_byte {
            total_size +=  tx.len();
        }
        total_size
    }

    pub fn change_mempool_size(&mut self, size: usize) {
        self.block_size = size;
        

        if self.transaction_size_in_bytes() >= self.block_size {
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
    pub fn sample_cmt(&mut self, sample_idx: Vec<u32>) -> (BlockHeader, Vec<Vec<Symbol>>, Vec<Vec<u64>>) {
        match &self.cmt_block {
            None => panic!("I don't have cmt block"),
            Some(cmt_block) => {
                //info!("{:?} sample cmt of size {}", self.addr, sample_idx.len());
                //info!("{:?} header {:?}", self.addr, cmt_block.block_header);
                //info!("{:?} transactions {:?}", self.addr, cmt_block.transactions);

                let (mut symbols, mut idx) = cmt_block.sampling_to_decode(1000 as u32); //sample_idx.len()
                //info!("idx {:?}", idx);
                //info!("symbols {:?}", symbols);
                // tests 
                //info!("{:?} after reading code, len {} {}", self.addr, self.codes_for_encoding.len(), self.codes_for_decoding.len());
                //info!("{:?} before constructing tree decoder", self.addr);
                //let mut decoder: TreeDecoder = TreeDecoder::new(self.codes_for_decoding.to_vec(), &cmt_block.block_header.coded_merkle_roots_hashes);
                //info!("{:?}, treedecoder n {} height {}", self.addr, decoder.n, decoder.height);
                //match decoder.run_tree_decoder(symbols.clone(), idx.clone()) {
                    //Ok(()) => (),
                    //_ => info!("tree decoder error"),
                //};
                //info!("{:?} after calling tree decoder", self.addr);
                (cmt_block.block_header.clone(), symbols, idx)
            }
        }
    }

    // currently a hack, need to combine with sample_cmt
    pub fn get_cmt_header(&self) -> BlockHeader {
        match &self.cmt_block {
            None => panic!("I don't have cmt block"),
            Some(cmt_block) => cmt_block.block_header.clone(),
        }
    }

    pub fn prepare_cmt_block(&mut self) -> Option<BlockHeader> {
        if self.transactions.len() == 0 {
            return None;
        }

        // get CMT
        let mut rng = rand::thread_rng();
        let header = BlockHeader {
            version: 1,
            previous_header_hash: CMTH256::default(),
            merkle_root_hash: CMTH256::default(),
            time: 4u32,
            bits: 5.into(),
            nonce: rng.gen(),
            coded_merkle_roots_hashes: vec![CMTH256::default(); 8],
        };
        // CMT - propose block
        //let transaction_size = String::from(t).len();
        //info!("{:?} transaction_size {:?}", self.addr, transaction_size);

        let mut transactions: Vec<Transaction> = Vec::new();
        let tx_bytes_size = self.transaction_size_in_bytes();

        // need to truncate 
        //info!("num transaction {}", self.transactions.len());
        //info!("tx_bytes_size {} self.block_size {}", tx_bytes_size , self.block_size);
        if tx_bytes_size > self.block_size {
            let mut s = 0;
            for i in 0..self.transactions.len() {
                s += self.transactions[i].bytes().len();
                if s > self.block_size {
                    if self.transactions.len() == 0 {
                        panic!("single transaction too large, block size is insufficient");
                    }
                    break;
                } else {
                    transactions.push(self.transactions[i].clone());
                }
            }

            for _ in 0..transactions.len() {
                self.transactions.pop_front();
            }
        } else {
            for tx in &self.transactions {
                transactions.push(tx.clone());
            }
            self.transactions.clear();

        }

        let mut trans_byte = transactions.iter().map(Transaction::bytes).collect::<Vec<Bytes>>();
        let mut total_size = 0;
        for tx in &trans_byte {
            total_size +=  tx.len();
        }
        info!("{:?} total_size {}  hello {:?}", self.addr, transactions.len(), total_size); 

        //let num_transactions = BLOCK_SIZE / (transaction_size as u64);
        //info!("{:?} num_transactions {:?}", self.addr, num_transactions);

        //let transactions: Vec<Transaction> = vec![t.into();num_transactions as usize];
        
        // number of systematic symbols for the codes on the four layers of CMT
        // info!("num code {:?} Code len {}", self.codes_for_encoding.len(), self.codes_for_encoding[0].symbols.len()); 
        //for i in 0..(codes_for_encoding.len()-1) {
            //info!("codes level {} parity len {} inner len {} {}", 
                  //i, codes_for_encoding[i].parities.len(), 
                  //codes_for_encoding[i].parities[0].len(),
                  //codes_for_encoding[i].parities[100].len());
            //info!("codes level {} symbol len {} inner len {} {}", 
                  //i, codes_for_encoding[i].symbols.len(),
                  //codes_for_encoding[i].symbols[0].len(),
                  //codes_for_encoding[i].symbols[100].len());
        //}

        let block: CMTBlock = CMTBlock::new(
            header.clone(), 
            &transactions, 
            BLOCK_SIZE as usize, 
            HEADER_SIZE, 
            &self.codes_for_encoding, 
            vec![true; self.codes_for_encoding.len()]);



        //info!("codes_for_encoding.len() {}", self.codes_for_encoding.len());

        //let (mut symbols, mut idx) = block.sampling_to_decode(1000 as u32); //sample_idx.len()
        let cmt_header = block.block_header.clone();
        
        self.cmt_block = Some(block);

        return Some(cmt_header);
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
            transactions: vec![],
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
        let tx_bytes_size = self.transaction_size_in_bytes();
        info!("insert transaction");

        // need to truncate 
        if tx_bytes_size > self.block_size {
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

