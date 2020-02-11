use std::sync::mpsc::{self};
use super::message::{Message, TaskRequest, PeerHandle};
use std::io::{self};
use std::thread;
use super::blockDb::{BlockDb};
use super::blockchain::blockchain::{BlockChain};
use super::mempool::mempool::{Mempool};
use super::contract::contract::{Contract};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use super::contract::interface::{Handle, Answer};
use super::contract::interface::Error as ContractError;
use super::primitive::block::ContractState;
use std::sync::{Arc, Mutex};
use crossbeam::channel::{self, Sender};
use super::primitive::hash::{H256};
use super::crypto::hash;
use super::primitive::block::{Block, MainNodeBlock};

pub struct Performer {
    task_source: mpsc::Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>, 
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
    contract_handler: Sender<Handle>,
}

impl Performer {
    pub fn new(
        task_source: mpsc::Receiver<TaskRequest>, 
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
        mempool: Arc<Mutex<Mempool>>,
        contract_handler: Sender<Handle>,
    ) -> Performer {
        Performer {
            task_source,
            chain: blockchain,
            block_db: block_db,
            mempool: mempool,
            contract_handler: contract_handler,
        } 
    }

    pub fn start(mut self) -> io::Result<()> {
        let handler = thread::spawn(move || {
            self.perform(); 
        }); 
        println!("Performer started");
        Ok(())
    }

    // TODO  compute H256
    pub fn compute_local_curr_hash(&self, block: Block, local_hash: H256) -> H256 {
        let block_ser = block.ser();
        let block_hash: [u8; 32] = hash(&block_ser).into();
        let chain = self.chain.lock().unwrap();
        let state = chain.get_latest_state().unwrap();
        let curr_hash: [u8; 32] = state.curr_hash.into();
        let concat_str = [curr_hash, block_hash].concat();
        let local_hash: H256 = hash(&concat_str);
        return local_hash;
    }

    fn get_eth_contract_state(&self, init_hash: [u8;32], start: usize, end: usize) -> Vec<ContractState> {
        let (answer_tx, answer_rx) = channel::bounded(1);
        let handle = Handle {
            message: ContractMessage::GetAll((init_hash, start, end)),
            answer_channel: Some(answer_tx),
        };
        self.contract_handler.send(handle);

        match answer_rx.recv() {
            Ok(answer) => {
                match answer {
                    Answer::Success(resp) => {
                        match resp {
                            ContractResponse::GetAll(contract_list) => contract_list,
                            _ => panic!("get_all_eth_contract_state wrong answer"), 
                        }
                    },
                    _ => panic!("get_all_eth_contract_state fail"),
                }
            },
            Err(e) => Vec::new(), 
        }
    }

    // TODO implement optimization 
    fn update_local_chain(
        &self, 
        peer_state: ContractState, 
        chain_state: ContractState,
        blockchain: &mut BlockChain,
    ) {
        let eth_states = self.get_eth_contract_state([0;32], 0, 0);         
        blockchain.replace(eth_states);
        //if chain_state.block_id > peer_state.block_id {
        //    let eth_state = eth_states[peer_state.block_id];
        //    if eth_state == peer_state {
        //        chain.revise(peer_state.block_id, eth_states[peer_state.block_id..].to_vec()); 
        //    } 
        //}
}

    fn perform(&self) {
        loop {
            let task = self.task_source.recv().unwrap();
            match task.msg {
                Message::Ping(info_msg) => {
                    println!("receive Ping {}", info_msg);
                    let response_msg = Message::Pong("pong".to_string());
                    task.peer.unwrap().response_sender.send(response_msg);
                }, 
                Message::Pong(info_msg) => {
                    println!("receive Pong {}", info_msg);                  
                },
                Message::SyncBlock(main_node_block) => {
                    println!("receive sync block");
                    let peer_state = main_node_block.contract_state;
                    let peer_block = main_node_block.block;

                    let mut chain = self.chain.lock().unwrap();
                    let chain_state = match chain.get_latest_state() {
                        Some(s) => s,
                        None => {
                        // TODO chain initialization, but need to handle case when eth chain is 0 too
                            let eth_states = self.get_eth_contract_state([0;32], 0, 0);         
                            chain.replace(eth_states);               
                            chain.get_latest_state().unwrap()
                        }
                    };
                    
                    if chain_state.block_id == peer_state.block_id - 1 {
                        // 1. compute curr_hash locally using all prev blocks stored in block_db
                        let local_update_hash = self.compute_local_curr_hash(peer_block, chain_state.curr_hash);
                        if local_update_hash == peer_state.curr_hash {
                            // add to blockchain
                            chain.insert(&peer_state);;
                        } else {
                            // sync one block
                            self.update_local_chain(peer_state, chain_state, &mut chain);
                        }
                    } else if chain_state == peer_state {
                        println!("receive block already present in blockchain");    
                    } else {
                        // need to sync block
                         self.update_local_chain(peer_state, chain_state, &mut chain); 
                    }
                    drop(chain);

                    // TODO store block in blockdb
                    
                },
                Message::SendTransaction(transaction) => {
                    let mut mempool = self.mempool.lock().expect("perform locl mempool");
                    mempool.insert(transaction);
                },
            }
        } 
    }
}

