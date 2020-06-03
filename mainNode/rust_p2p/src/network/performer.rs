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
use std::net::{SocketAddr};
use super::primitive::hash::{H256};
use super::crypto::hash;
use super::primitive::block::{Block, EthBlkTransaction};
extern crate crypto;
use crypto::sha2::Sha256;
use crypto::digest::Digest;

use super::contract::utils;
use std::collections::HashMap;

pub struct Performer {
    task_source: Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>, 
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
    scheduler_handler: Sender<Option<Token>>,
    contract_handler: Sender<Handle>,
    is_scale_node: bool,
    addr: SocketAddr,
    proposer_by_addr: HashMap<SocketAddr, Sender<String> >,
    //curr_proposer: Option<Sender<String>>,
}

impl Performer {
    pub fn new(
        task_source: Receiver<TaskRequest>, 
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
        mempool: Arc<Mutex<Mempool>>,
        scheduler_handler: Sender<Option<Token>>,
        contract_handler: Sender<Handle>,
        is_scale_node: bool,
        addr: SocketAddr,
    ) -> Performer {
        Performer {
            task_source,
            chain: blockchain,
            block_db: block_db,
            mempool: mempool,
            contract_handler: contract_handler,
            scheduler_handler: scheduler_handler,
            is_scale_node: is_scale_node,
            addr: addr,
            proposer_by_addr: HashMap::new(),
            //curr_proposer: None,
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
                    info!("{:?} receive token", self.addr);
                    self.scheduler_handler.send(Some(token));
                },
                // temporary heck, need to move to scale-network
                Message::ScaleProposeBlock(Block) => {
                    if self.is_scale_node {
                        info!("{:?} receive ScaleProposeBlock", self.addr);
                        let local_addr = self.addr.clone();
                        let peer_handle = task.peer.unwrap();
                        let proposer_addr = peer_handle.addr;

                        let (tx, rx) = channel::unbounded();
                        self.proposer_by_addr.insert(proposer_addr, tx);
                        // this scalenode receive propose block
                        // ask the node to send chunks
                        //let start_time = time::
                        // send neighbor to send chunks
                        let response_msg = Message::ScaleReqChunks;
                        peer_handle.response_sender.send(response_msg);
                        // timed loop
                        thread::spawn(move || {
                            let mut num_chunk = 0;
                            let mut chunk_complete = false;
                            loop {
                                match rx.recv() {
                                    Ok(reply) => {
                                        info!("receive ScaleReqChunksreply"); //say 1 chunks is sufficient
                                        num_chunk += 1;
                                    },
                                    Err(e) => info!("proposer error"),
                                }
                                if num_chunk > 0 {
                                    // send to ethereum 
                                    info!("{:?} is ready to aggregate sign", local_addr);
                                    let response_msg = Message::MySign(local_addr.to_string());
                                    peer_handle.response_sender.send(response_msg);
                                    

                                }

                                //wait for other sign + modify rx message type

                                //commit to eth after receiving all message
                            }
                            
                        });  

                    }
                },
                Message::MySign(m) => {
                    info!("{:?} receive MySign message {:?}", self.addr, m);
                    // send to spawned thread like ScaleReqChunksReply

                },
                Message::ScaleReqChunks => {
                    info!("{:?} receive ScaleReqChunks", self.addr);
                    // this client needs to prepare chunks in response to 
                    // scalenode

                    // this sends chunk to the scale node
                    let response_msg = Message::ScaleReqChunksReply;
                    task.peer.unwrap().response_sender.send(response_msg);
                },
                Message::ScaleReqChunksReply => {
                    if self.is_scale_node {
                        info!("{:?} receive ScaleReqChunksReply", self.addr);
                        let proposer_socket = task.peer.unwrap().addr ;
                        
                        match &self.proposer_by_addr.get(&proposer_socket) {
                            Some(sender) => {
                                sender.send("Chunk ready".to_string());
                            },
                            None => info!("case when no proposer but receive chunk reply"),
                        }
                        
                    } 
                },
                Message::ScaleGetAllChunks => {
                    if self.is_scale_node {
                        info!("{:?} receive ScaleGetAllChunks", self.addr);

                    }
                },
            }
        } 
    }
}

