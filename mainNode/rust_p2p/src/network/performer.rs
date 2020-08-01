use std::sync::mpsc::{self};
use std::io::{self};
use std::thread;
use std::sync::{Arc, Mutex};

use crate::db::blockDb::{BlockDb};
use crate::blockchain::blockchain::{BlockChain};
use crate::mempool::mempool::{Mempool};
use crate::mempool::scheduler;

use super::message::{Message, TaskRequest, PeerHandle, Samples, ServerSignal};
use super::contract::contract::{Contract};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use super::contract::interface::{Handle, Answer};
use super::contract::interface::Error as ContractError;
use super::primitive::block::ContractState;

use crossbeam::channel::{self, Sender, Receiver};
use std::net::{SocketAddr};
use super::primitive::hash::{H256};
use super::crypto::hash;
use super::primitive::block::{Block, EthBlkTransaction};
use chain::transaction::Transaction;
extern crate crypto;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use super::contract::utils;
use std::collections::HashMap;
use web3::types::{U256};
use core::borrow::BorrowMut;
use primitives::bytes::{Bytes};
use ser::{deserialize, serialize};
use mio_extras::channel::Sender as MioSender;
use super::cmtda::{BlockHeader};
use hex;
use std::time::{SystemTime, UNIX_EPOCH};
use chain::constants::{BLOCK_SIZE, BASE_SYMBOL_SIZE, RATE, UNDECODABLE_RATIO};

pub struct Performer {
    task_source: Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>, 
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
    scheduler_handler: Sender<scheduler::Signal>,
    contract_handler: Sender<Handle>,
    addr: SocketAddr,
    proposer_by_addr: HashMap<SocketAddr, Sender<Samples> >,
    key_file: String,
    scale_id: usize,
    agg_sig: Arc<Mutex<HashMap<String, (String, String, usize)>>>,
    threshold: usize,
    server_control_sender: MioSender<ServerSignal>,
    manager_source: Sender<(usize, Option<Samples>)>,
    num_nodes: u64,
}

pub fn new(
    task_source: Receiver<TaskRequest>, 
    blockchain: Arc<Mutex<BlockChain>>,
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
    scheduler_handler: Sender<scheduler::Signal>,
    contract_handler: Sender<Handle>,
    addr: SocketAddr,
    key_file: String,
    scale_id: usize,
    threshold: usize,
    server_control_sender: MioSender<ServerSignal>,
    manager_source: Sender<(usize, Option<Samples>)>,
    num_nodes: u64,
) -> Performer {
    Performer {
        task_source,
        chain: blockchain,
        block_db: block_db,
        mempool: mempool,
        contract_handler: contract_handler,
        scheduler_handler: scheduler_handler,
        addr: addr,
        proposer_by_addr: HashMap::new(),
        key_file,
        scale_id,
        agg_sig: Arc::new(Mutex::new(HashMap::new())),
        threshold,
        server_control_sender: server_control_sender,
        manager_source: manager_source,
        num_nodes: num_nodes,
    } 
}

impl Performer {
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
            message: ContractMessage::GetCurrState(0),
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
            let peer_handle = task.peer.unwrap();
            match task.msg {
                Message::Ping(info_msg) => {
                    info!("receive Ping {}", info_msg);
                    let response_msg = Message::Pong("pong".to_string());
                    peer_handle.write(response_msg);
                }, 
                Message::Pong(info_msg) => {
                    info!("receive Pong {}", info_msg);                  
                },
                Message::SyncBlock(main_node_block) => {
                    info!("receive sync block");
                    self.update_block(main_node_block);
                },
                Message::SendTransaction(transaction_ser) => {
                    let transaction: Transaction = deserialize(&transaction_ser as &[u8]).unwrap();
                    let mut mempool = self.mempool.lock().expect("perform locl mempool");
                    mempool.insert(transaction);
                    drop(mempool);
                },
                Message::PassToken(token) => {
                    info!("{:?} receive token", self.addr);
                    self.scheduler_handler.send(scheduler::Signal::Data(token));
                },
                Message::ProposeBlock((header, block_id)) => {
                    if self.scale_id > 0 {
                        let hash_str = utils::hash_header_hex(&header);
                        //info!("{:?} receive ProposeBlock: header hash: {:?}", self.addr, hash_str);
                        let local_addr = self.addr.clone();
                        
                        let proposer_addr = peer_handle.addr;

                        let (tx, rx) = channel::unbounded();
                        self.proposer_by_addr.insert(proposer_addr, tx);
                        // this scalenode receive propose block
                        let response_msg = Message::ScaleReqChunks;

                        let header_cmt: BlockHeader = deserialize(&header.clone() as &[u8]).unwrap();
                        // hard coded
                        let num_symbol = BLOCK_SIZE/(BASE_SYMBOL_SIZE as u64) *((1.0/RATE) as u64);
                        let samples_idx = get_sample_index(self.scale_id, num_symbol as usize, self.num_nodes); //self.scale_id 

                        let response_msg = Message::ScaleReqChunks(samples_idx);
                        peer_handle.write(response_msg);

                        let keyfile = self.key_file.clone();
                        let scaleid = self.scale_id.clone();
                        let local_aggsig = self.agg_sig.clone();
                        let broadcaster = self.server_control_sender.clone();
                        let db = self.block_db.clone();

                        let contract_handler = self.contract_handler.clone();
                        let num_nodes = self.num_nodes;
                        
                        // timed loop
                        thread::spawn(move || {
                            let mut num_chunk = 0;
                            let chunk_thresh = (((BLOCK_SIZE/BASE_SYMBOL_SIZE as u64) as f32 )/RATE*UNDECODABLE_RATIO) as u64 / num_nodes; 

                            let mut chunk_complete = false;

                            loop {
                                match rx.recv() {
                                    Ok(chunk_reply) => {
                                        let mut local_db = db.lock().unwrap();
                                        // compute id
                                        local_db.insert_cmt_sample(block_id, &chunk_reply);
                                        let num =  get_num_base_symbols(&chunk_reply.idx);
                                        num_chunk += num;
                                    },
                                    Err(e) => info!("proposer error"),
                                }
                                info!("number stored chunks {} thresh {}", num_chunk, chunk_thresh);
                                if num_chunk > chunk_thresh {
                                    // vote
                                    let header_str: String = hex::encode(&header);
                                    
                                    let (sigx, sigy) = utils::_sign_bls(header_str.clone(), keyfile);
                                    let sid = 0;
                                    let response_msg = Message::MySign(header_str.clone(), sid, block_id, sigx.clone(), sigy.clone(), scaleid);
                                    let signal = ServerSignal::ServerBroadcast(response_msg);
                                    
                                    
                                    let mut aggsig = local_aggsig.lock().unwrap();
                                    if aggsig.get(&header_str).is_none() {
                                        aggsig.insert(header_str.clone(),  (sigx.clone(), sigy.clone(), (1 << scaleid)));
                                    } else {
                                        let ( x, y, bitset) = aggsig.get(&header_str).unwrap();
                                        let (sigx, sigy) = utils::_aggregate_sig(x.to_string(), y.to_string(), sigx, sigy);
                                        let bitset = bitset + (1 << scaleid);
                                        aggsig.insert(
                                            header_str.clone(),  
                                            (sigx.clone(), sigy.clone(), bitset.clone()));
                                    }
                                    broadcaster.send(signal);
                                    break;
                                }
                            }
                            // after time out
                            // vote and communicate signature depending on number of recv chunks
                        });
                    }
                },
                Message::MySign(header , sid, bid, sigx, sigy, scale_id) => {
                    if self.scale_id <= 0 {
                        continue;
                    }

                    // new
                    let decode_header = hex::decode(&header).unwrap();
                    let header_hash_str = utils::hash_header_hex(&decode_header);
                    //info!("{:?} receive Mysign header hash: {:?}, header size {}", self.addr, header_hash_str, header.len());
                    let mut sigx = sigx;
                    let mut sigy = sigy;
                    //info!("{:?} receive MySign message from node {:?}.", self.addr, scale_id);
                    // send to spawned thread like ScaleReqChunksReply

                    let local_aggsig = self.agg_sig.clone();
                    let mut aggsig = local_aggsig.lock().unwrap();
                    let threshold = (UNDECODABLE_RATIO*(self.num_nodes as f32)).ceil() as usize ;
                    info!("num node {}", self.num_nodes);

                    if aggsig.get(&header).is_none() {
                        //info!("{:?} Mysign first seen header", self.addr);
                        aggsig.insert(header.clone(),  (sigx, sigy, (1 << scale_id)));
                    } else {
                        let ( x, y, mut bitset) = aggsig.get(&header).unwrap().clone();
                        if (1 << scale_id) & bitset.clone() == 0 {
                            info!("Mysign {:?} old bitste {}", self.addr, bitset);
                            let (sigx_t, sigy_t) = utils::_aggregate_sig(x.to_string(), y.to_string(), sigx.clone(), sigy.clone());
                            sigx = sigx_t;
                            sigy = sigy_t;
                            bitset = bitset + (1 << scale_id);
                            aggsig.insert(header.clone(), (sigx.clone(), sigy.clone(), bitset.clone()));
                        }
                        info!("signature num {} thresh {}", utils::_count_sig(bitset.clone()) , threshold);
                        if utils::_count_sig(bitset.clone()) >= threshold {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: ContractMessage::SubmitVote(header, U256::from(sid), U256::from(bid), U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(bitset.clone())),
                                answer_channel: Some(answer_tx),
                            };
                            //info!("{:?} fake submit vote", self.addr);
                            self.contract_handler.send(handle);

                            //info!("smart contarct update");
                        }
                    }
                },
                Message::ScaleReqChunks(samples_idx) => {
                    let sample_len = samples_idx.len();
                    // this client needs to prepare chunks in response to 
                    let mut mempool = self.mempool.lock().expect("lock mempool");
                    let (header, symbols, idx) = mempool.sample_cmt(samples_idx);
                    let header_bytes = serialize(&header);

                    let hash_str = utils::hash_header_hex(&header_bytes);
                    //info!("{:?} recv {:?} ScaleReqChunks, header hash: {:?} ", self.addr, sample_len, hash_str);
                    // this sends chunk to the scale node
                    let chunks = Samples {
                        header: header_bytes.into(),
                        symbols: symbols,
                        idx: idx,
                    };
                    // this client needs to prepare chunks in response to scalenode

                    let response_msg = Message::ScaleReqChunksReply(chunks);
                    peer_handle.write(response_msg);
                },
                Message::ScaleReqChunksReply(chunks) => {
                    if self.scale_id > 0 {
                        let proposer_socket = peer_handle.addr ;
                        //info!("{:?} receive ScaleReqChunksReply from {:?}", self.addr, proposer_socket);
                        match &self.proposer_by_addr.get(&proposer_socket) {
                            Some(sender) => {
                                sender.send(chunks);
                            },
                            None => info!("case when no proposer but receive chunk reply"),
                        }
                        
                    } 
                },
                Message::ScaleGetAllChunks(state) => {
                    if self.scale_id > 0 {
                        //info!("{:?} receive ScaleGetAllChunks {:?}", self.addr, state);
                        let local_db = self.block_db.lock().unwrap();
                        let chunk = local_db.get_chunk(state.block_id);
                        drop(local_db);
                        let response_msg = match chunk {
                            Some(chunk) => Message::ScaleGetAllChunksReply((Some(chunk), state.block_id)),
                            None => Message::ScaleGetAllChunksReply((None, state.block_id)),
                        };
                        peer_handle.write(response_msg);
                    }
                },
                Message::ScaleGetAllChunksReply((chunk, block_id)) => {
                    //info!("{:?} recv ScaleGetAllChunksReply", self.addr);
                    self.manager_source.send((block_id, chunk));
                },
            }
        } 
    }
}

// scale id starts at 1
pub fn get_sample_index(scale_id: usize, num_trans: usize, num_node: u64) -> Vec<u32> {
    let num_sample = ((num_trans as f32) / (num_node as f32)).ceil() as usize;
    let mut sample_idx = vec![];
    let start = (scale_id-1)*num_sample;
    let stop = if scale_id*num_sample > num_trans {
        num_trans
    } else {
        scale_id*num_sample
    };
    for i in start..stop {
        sample_idx.push(i as u32);
    }
    sample_idx
}

pub fn get_num_base_symbols(idx: &Vec<Vec<u64>>) -> u64 {
    assert!(idx.len() > 0);
    idx[0].len() as u64
}
