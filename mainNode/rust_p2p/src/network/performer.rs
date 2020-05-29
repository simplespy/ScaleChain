use std::sync::mpsc::{self};
use super::message::{Message, TaskRequest, PeerHandle};
use std::io::{self};
use std::thread;
use super::blockDb::{BlockDb};
use super::blockchain::blockchain::{BlockChain};
use super::mempool::mempool::{Mempool};
use super::scheduler::{Scheduler, Token};
use super::contract::contract::{Contract};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use super::contract::interface::{Handle, Answer};
use super::contract::interface::Error as ContractError;
use super::primitive::block::ContractState;
use std::sync::{Arc, Mutex};
use crossbeam::channel::{self, Sender, Receiver};
use super::primitive::hash::{H256};
use super::crypto::hash;
use super::primitive::block::{Block, EthBlkTransaction};
extern crate crypto;
use crypto::sha2::Sha256;
use crypto::digest::Digest;

use super::contract::utils;

pub struct Performer {
    task_source: Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>, 
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
    scheduler_handler: Sender<Option<Token>>,
    contract_handler: Sender<Handle>,
}

impl Performer {
    pub fn new(
        task_source: Receiver<TaskRequest>, 
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
        mempool: Arc<Mutex<Mempool>>,
        scheduler_handler: Sender<Option<Token>>,
        contract_handler: Sender<Handle>,
    ) -> Performer {
        Performer {
            task_source,
            chain: blockchain,
            block_db: block_db,
            mempool: mempool,
            contract_handler: contract_handler,
            scheduler_handler: scheduler_handler,
        } 
    }

    pub fn start(mut self) -> io::Result<()> {
        let handler = thread::spawn(move || {
            self.perform(); 
        }); 
        info!("Performer started");
        Ok(())
    }

    // TODO  compute H256
    pub fn compute_local_curr_hash(
        &self, 
        block: &Block,
        local_hash: H256
    ) -> H256 {
        let block_ser = block.ser();
        let block_ser_hex = hex::encode(&block_ser);
        let mut hasher = Sha256::new();
        hasher.input_str(&block_ser_hex);
        let mut block_hash = [0u8;32];
        hasher.result(&mut block_hash);
        let curr_hash: [u8; 32] = local_hash.into();

        let concat_str = [curr_hash, block_hash].concat();
        let local_hash: H256 = hash(&concat_str);
        return local_hash;
    }

    fn get_eth_transactions(&self, start: usize, end: usize) -> Vec<EthBlkTransaction> {
        let (answer_tx, answer_rx) = channel::bounded(1);
        let handle = Handle {
            message: ContractMessage::GetAll(([0 as u8;32], start, end)),
            answer_channel: Some(answer_tx),
        };
        self.contract_handler.send(handle);

        match answer_rx.recv() {
            Ok(answer) => {
                match answer {
                    Answer::Success(resp) => {
                        match resp {
                            ContractResponse::GetAll(requested_list) => requested_list,
                            _ => panic!("performer contract get wrong answer"), 
                        }
                    },
                    _ => panic!("fail"),
                }
            },
            Err(e) => panic!("performer contract channel broke"), 
        }
    }


    fn get_eth_curr_state(&self) -> ContractState {
        let (answer_tx, answer_rx) = channel::bounded(1);
        let handle = Handle {
            message: ContractMessage::GetCurrState,
            answer_channel: Some(answer_tx),
        };
        self.contract_handler.send(handle);

        match answer_rx.recv() {
            Ok(answer) => {
                match answer {
                    Answer::Success(resp) => {
                        match resp {
                            ContractResponse::GetCurrState(state) => state,
                            _ => panic!("get_all_eth_contract_state wrong answer"), 
                        }
                    },
                    _ => panic!("get_all_eth_contract_state fail"),
                }
            },
            Err(e) => {
                panic!("performer to contract handler channel broke");
            }, 
        }

    }

    fn update_block(&self, main_node_block: EthBlkTransaction) {
        let peer_state = main_node_block.contract_state;
        let peer_block = main_node_block.block;
        let mut chain = self.chain.lock().unwrap();
        let local_state = match chain.get_latest_state() {
            Some(s) => s,
            None => {
                info!("sync blockchain in performer");
                let eth_transactions = self.get_eth_transactions(0, 0);         
                let eth_states: Vec<ContractState> = eth_transactions.
                    into_iter().
                    map(|tx| {
                        tx.contract_state
                    }).collect();
                chain.replace(eth_states);                              
                chain.get_latest_state().expect("eth blockchain is empty")
            }
        };

        
        // 1. compute curr_hash locally using all prev blocks stored in block_db
        if local_state.block_id+1 == peer_state.block_id {
            let local_comp_hash = self.compute_local_curr_hash(
                &peer_block, 
                local_state.curr_hash
            );
            let local_comp_state = ContractState {
                curr_hash: local_comp_hash,
                block_id: chain.get_height() + 1,
            };

            // peer is dishonest and lazy
            if local_comp_hash != peer_state.curr_hash {
                warn!("peer is dishonest and lazy");
                drop(chain);
                return;
            }

            // get latest state from ethernet, check if peer is honest node
            let eth_curr_state = self.get_eth_curr_state();
            if local_comp_state == eth_curr_state {
                info!("honest node -> update chain");
                // honest -> need to sync up
                // add to block database
                let mut block_db = self.block_db.lock().unwrap();
                block_db.insert(&peer_block);
                drop(block_db);
                // add to blockchain if not there
                chain.insert(&peer_state);;
            } else {
                warn!("peer is malicious and complicated. TODO use some mechanism to remember it");
                return;
            }
        } else if local_state.block_id == peer_state.block_id {
            info!("local chain already synced");
        } else if local_state.block_id+1 < peer_state.block_id {
            info!("possibly lagging many nodes");
            // possibly lagging many blocks, 
            // 1. query get all from current chain height to current eth height
            // 2. query peer to collect all blocks(the upper bound is unknown)
            let miss_eth_transactions = self.get_eth_transactions(local_state.block_id, 0);
            let mut block_db = self.block_db.lock().unwrap();
            for eth_tx in miss_eth_transactions {
                block_db.insert(&eth_tx.block);
                chain.append(&eth_tx.contract_state);
            }
            drop(block_db);
        } else {
            panic!("local chain screw up, it is greater than eth chain");
        }
        drop(chain);
    }

    fn perform(&mut self) {
        loop {
            let task = self.task_source.recv().unwrap();
            match task.msg {
                Message::Ping(info_msg) => {
                    info!("receive Ping {}", info_msg);
                    let response_msg = Message::Pong("pong".to_string());
                    task.peer.unwrap().response_sender.send(response_msg);
                }, 
                Message::Pong(info_msg) => {
                    info!("receive Pong {}", info_msg);                  
                },
                Message::SyncBlock(main_node_block) => {
                    info!("receive sync block");
                    self.update_block(main_node_block);
                },
                Message::SendTransaction(transaction) => {
                    let mut mempool = self.mempool.lock().expect("perform locl mempool");
                    mempool.insert(transaction);
                    drop(mempool);
                },
                Message::PassToken(token) => {
                    // call scheduler
                    info!("receive token");
                    self.scheduler_handler.send(Some(token));
                },
                // temporary heck, need to move to scale-network
                Message::ScaleProposeBlock(Block) => {
                    println!("receive ScaleProposeBlock");
                },
                Message::ScaleReqChunks => {

                },
                Message::ScaleGetChunks => {

                },
                Message::ScaleGetAllChunks => {

                },
            }
        } 
    }
}

