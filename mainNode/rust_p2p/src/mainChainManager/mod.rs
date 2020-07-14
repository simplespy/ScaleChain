use crossbeam::channel::{self, Sender, Receiver, TryRecvError};
use super::contract::interface::{Handle, Answer};
use super::primitive::block::ContractState;
use std::{thread, time};
use mio_extras::channel::Sender as MioSender;
use super::network::message::{Message, ServerSignal, ChunkReply};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use super::contract::utils;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use super::blockchain::blockchain::{BlockChain};
use std::collections::HashMap;
use super::db::blockDb::{BlockDb};
use chain::block::Block as SBlock;
use chain::decoder::CodingErr;
use chain::decoder::{Symbol};
use chain::decoder::{Code, Decoder, TreeDecoder, IncorrectCodingProof};
use super::cmtda::{read_codes, BlockHeader};
use super::cmtda::Transaction as CMTTransaction;
use primitives::bytes::{Bytes};
use crypto::sha3::Sha3;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use super::primitive::hash::H256;
use ser::{deserialize, serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use super::experiment::snapshot::PERFORMANCE_COUNTER;

pub struct Manager {
    pub contract_handler: Sender<Handle>,
    pub chain: Arc<Mutex<BlockChain>>, 
    pub block_db: Arc<Mutex<BlockDb>>,
    pub server_control_sender: MioSender<ServerSignal>,
    pub addr: SocketAddr,
    pub manager_sink: Receiver<(usize, Option<ChunkReply>)>,
    pub chunk_senders: HashMap<usize, Sender<Option<ChunkReply>> >,
    pub codes_for_encoding: Vec<Code>,
    pub codes_for_decoding: Vec<Code>,
    pub k_set: Vec<u64>,
}

pub struct JobManager {
    state: ContractState,
    addr: SocketAddr,
    server_control_sender: MioSender<ServerSignal>,
    chunk_receiver: Receiver<Option<ChunkReply>>,
    block_source: Sender<Result<SBlock, CodingErr>>,
    codes_for_encoding: Vec<Code>,
    codes_for_decoding: Vec<Code>,
    k_set: Vec<u64>,
}

// currently only handle one layer encoding
fn collect_cmt_chunks(job_manager: JobManager) {
    //info!("{:?} managing chunks", job_manager.addr);
    let chunk_receiver = &job_manager.chunk_receiver;
    let mut coll_symbols: Vec<Vec<Symbol>> = Vec::new();
    let mut coll_idx: Vec<Vec<u64>> = Vec::new();
    let mut is_first_recv = true;
    let start = SystemTime::now();
    info!("{:?} start collect cmt for {:?}", job_manager.addr, job_manager.state);
    loop {
        // accumulate chunks
        match chunk_receiver.recv() {
            // accumulate chunks 
            Ok(chunk) => {
                match chunk {
                    None => info!("does not recv chunk"),
                    Some(chunk) => {
                        let elapsed = start.elapsed();
                        let hash_str = utils::hash_header_hex(&chunk.header as &[u8]);
                        info!("{:?} get hash_str  {:?} {:?}", job_manager.addr, hash_str, elapsed);
                        let header: BlockHeader = deserialize(&chunk.header as &[u8]).unwrap();

                        
                                                
                        let num_layer = job_manager.k_set.len();
                        if is_first_recv {
                            is_first_recv = false;
                            coll_symbols = chunk.symbols.clone();
                            coll_idx = chunk.idx.clone();
                            info!("coll_symbols {:?}", coll_symbols.len());
                            info!("coll_idx {:?}", coll_idx.len());
                        } else {
                            for l in 0..num_layer {
                                let symbols = &chunk.symbols[l];
                                let idx = &chunk.idx[l];
                                let mut c_symbols = &mut coll_symbols[l];
                                let mut c_idx = &mut coll_idx[l];
                                let mut j =0;
                                for i in idx {
                                    if c_idx.contains(i) {
                                        //
                                    } else {
                                        c_idx.push(*i);
                                        c_symbols.push(symbols[j]);
                                    }
                                    j += 1;
                                }
                            }
                        }

                        
                        //info!("recon len {}", recon.len());
                        //info!("{:?}", hex::encode(recon));
                        
                        // accumulate chunks + currently only handle single layer
                        let mut decoder: TreeDecoder = TreeDecoder::new(
                            job_manager.codes_for_decoding.to_vec(), 
                            &header.coded_merkle_roots_hashes);
                        let mut cmt_pass = match decoder.run_tree_decoder(coll_symbols.clone(), coll_idx.clone()) {
                            Ok(()) => true,
                            _ => false,
                        };
                        if cmt_pass {
                            info!("{:?} cmt pass", job_manager.addr);
                            // collect all base + reconstruct block
                            let mut recon:Vec<u8> = vec![];
                            let systematic_symbol_len = job_manager.k_set[0];
                            for i in 0..systematic_symbol_len {
                                for j in 0..(coll_symbols[0].len()) {
                                    if coll_idx[0][j] == i as u64 {
                                        match coll_symbols[0][j] {
                                            Symbol::Base(s) => recon.extend_from_slice(&s),
                                            _ => unreachable!(),
                                        }
                                    }
                                }
                            }
                            // convert bytes into vec<transaction>
                            //info!("{:?} vec<tx> {:?}", self.addr, recon);
                            // TODO
                            let mut r = SBlock {
                                block_header: header.clone(),
                                transactions: vec![],
                                coded_tree: vec![],
                                block_size_in_bytes:0 
                            };

                            job_manager.block_source.send(Ok(r));
                        } else {
                            info!("{:?} cmt invalid", job_manager.addr);
                            // error handle
                        }
                    }
                }
            }
           _ => panic!("{:?} job manager error", job_manager.addr),
        }
    }
}

impl Manager {
    pub fn new(
        contract_handler: Sender<Handle>, 
        chain: Arc<Mutex<BlockChain>>,
        server_control_sender: MioSender<ServerSignal>,
        addr: SocketAddr,     
        manager_sink: Receiver<(usize, Option<ChunkReply>)>,
        block_db: Arc<Mutex<BlockDb>>,
        codes_for_encoding: Vec<Code>,
        codes_for_decoding: Vec<Code>,
        k_set: Vec<u64>,
    ) -> Manager {
        Manager {
            contract_handler: contract_handler,
            chain: chain,
            server_control_sender: server_control_sender,
            addr: addr,
            chunk_senders: HashMap::new(),
            manager_sink: manager_sink,
            block_db: block_db,
            codes_for_encoding: codes_for_encoding,
            codes_for_decoding: codes_for_decoding,
            k_set: k_set,
        }
    }

    // spawn a new thread pulling for update from mainchain 
    pub fn start(mut self){
        thread::spawn(move || {
            let mut blocks_sink: HashMap<usize, Receiver<Result<SBlock, CodingErr>>> = HashMap::new();
            let mut register_blocks: HashMap<usize, ContractState> = HashMap::new();
            let mut ready_blocks: HashMap<usize, ContractState> = HashMap::new();
            let mut longest_id = 0;
            let mut start = SystemTime::now();

            let check_hash = false;

            loop {
                let mut rm: Vec<usize> = vec![];
                // check if any threads finish
                for (block_id, block_sink) in &blocks_sink {
                    match block_sink.try_recv() {
                        Err(TryRecvError::Empty) => (),
                        Err(TryRecvError::Disconnected) => panic!("block sink broken"),
                        Ok(result) => {
                            // a thread has finished processing cmt
                            match result {
                                Ok(sblock) => {
                                    info!("{:?} cmt finishes", self.addr);
                                    rm.push(*block_id);
                                    let mut sblock_db = self.block_db.lock().unwrap();
                                    sblock_db.insert_sblock(*block_id, sblock);
                                    drop(sblock_db);

                                    // update ready chain
                                    let state = register_blocks.remove(block_id).expect("get block state");
                                    ready_blocks.insert(*block_id, state);

                                    // update blockchain
                                    let mut local_chain = self.chain.lock().unwrap();
                                    let tip_state = local_chain.get_latest_state().unwrap();
                                    info!("{:?} tip_state {:?} longest_id {}", self.addr, tip_state, longest_id);

                                    let mut curr_hash = tip_state.curr_hash.clone();
                                    // test if update block chain
                                    for i in (tip_state.block_id+1) .. (longest_id+1) {
                                        match ready_blocks.get(&i) {
                                            None => info!("{:?} block {} is missing", self.addr, i),
                                            Some(s) => {
                                                info!("{:?} db get block {:?}", self.addr, i);
                                                let mut sblock_db = self.block_db.lock().unwrap();
                                                let header = match sblock_db.get_sblock(*block_id) {
                                                    Some(b) => b.block_header.clone(),
                                                    None => unreachable!(),
                                                };
                                                drop(sblock_db);

                                                let mut hash  = [0u8; 32];
                                                let header_bytes = serialize(&header);
                                                // get hash in the same way as bls
                                                let mut hasher = Sha256::new();;
                                                hasher.input(&header_bytes);
                                                hasher.result(&mut hash);
                                                let hash_str = hex::encode(&hash);
                                                info!("header hash {:?}", hash_str);

                                                //let mut curr_hash_str: String = hex::encode(&curr_hash.0);
                                                let v = [ curr_hash.0, hash].concat();
                                                let mut sec_hasher = Sha256::new();
                                                let mut hash  = [0u8; 32];
                                                sec_hasher.input(&v);
                                                sec_hasher.result(&mut hash);

                                                // compare if smart contract hash equals to local
                                                let new_hash = H256(hash);
                                                info!("new hash {:?}", new_hash);
                                                if  new_hash != s.curr_hash {
                                                    info!("{:?}, inconsistent hash {:?} smart contract {} hash {:?}", 
                                                          self.addr,
                                                          new_hash, 
                                                          i,
                                                          s.curr_hash); 
                                                    if check_hash{
                                                        break; 
                                                    }
                                                }

                                                info!("{:?} local chain update to {:?}", self.addr, s);

                                                local_chain.append(s);
                                                curr_hash = s.curr_hash;
                                                PERFORMANCE_COUNTER.record_chain_update();
                                            },
                                        }
                                    }
                                    drop(local_chain);
                                },
                                Err(e) => info!("cmt handler error "),
                            }
                        }
                    }
                }
                // romove finished threads handler
                for block_id in &rm {
                    blocks_sink.remove(block_id);
                }

                // job distributor to threads sender receiver
                match self.manager_sink.try_recv() {
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => panic!("manager sink broken"),
                    Ok((block_id, chunk)) => {
                        match self.chunk_senders.get(&block_id) {
                            None => info!("{:?} Error  no cmt get all request", self.addr),
                            Some(chunk_sender) => chunk_sender.send(chunk).unwrap(),
                        }
                    }
                }

                let interval = time::Duration::from_millis(100);
                thread::sleep(interval);

                // check state every  sec
                if start.elapsed().unwrap() > time::Duration::from_millis(2000) {
                    //check current state
                    //info!("{:?} check smart contract", self.addr);
                    start = SystemTime::now();
                    let (answer_tx, answer_rx) = channel::bounded(1);
                    let handle = Handle {
                        message: ContractMessage::GetCurrState(0),
                        answer_channel: Some(answer_tx),
                    };
                    self.contract_handler.send(handle);
                    let mut curr_state: Option<ContractState> = None;
                    match answer_rx.recv() {
                        Ok(answer) => {
                            match answer {
                                Answer::Success(resp) => {
                                    match resp {
                                        ContractResponse::GetCurrState(state) => {
                                            let mut local_chain = self.chain.lock().unwrap();
                                            let tip_state = local_chain.get_latest_state().expect("blockchain does not have state");
                                            drop(local_chain);
                                            // Ask performer to do the task
                                            if tip_state != state {
                                                if (false) {
                                                    // if get correct block from side chain network
                                                } else {
                                                    // if task is already handled
                                                    if self.chunk_senders.contains_key(&state.block_id) {
                                                        continue;
                                                    } 
                                                    info!("{:?}, update start: mainchain new state {:?} tip_state {:?}", self.addr, state, tip_state);
                                                    if longest_id < state.block_id {
                                                        longest_id = state.block_id;
                                                    }

                                                    // get block from scale node network
                                                    let (chunk_sender, chunk_receiver) = crossbeam::channel::unbounded();
                                                    let (block_sender, block_receiver) = crossbeam::channel::unbounded();
                                                    register_blocks.insert(state.block_id, state.clone());
                                                    blocks_sink.insert(state.block_id, block_receiver);
                                                    self.chunk_senders.insert(state.block_id, chunk_sender);
                                                    let mut job_manager = JobManager {
                                                        state: state.clone(), 
                                                        addr: self.addr.clone(),
                                                        server_control_sender: self.server_control_sender.clone(),
                                                        chunk_receiver: chunk_receiver,
                                                        block_source: block_sender,
                                                        k_set: self.k_set.clone(),
                                                        codes_for_encoding: self.codes_for_encoding.clone(),
                                                        codes_for_decoding: self.codes_for_decoding.clone(),
                                                    };

                                                    // create a new handler for each block
                                                    thread::spawn(move || {
                                                        collect_cmt_chunks(job_manager);
                                                   });

                                                    // broadcast get all chunks
                                                    let response_msg = Message::ScaleGetAllChunks(state.clone());
                                                    info!("{:?} broadcase ScaleGetAllChunks {:?}", self.addr, state);
                                                    let signal = ServerSignal::ServerBroadcast(response_msg);
                                                    self.server_control_sender.send(signal);

                                                }
                                            }
                                        },
                                        _ => panic!("performer contract get wrong answer"), 
                                    }
                                },
                                _ => panic!("fail"),
                            }
                        },
                        Err(e) => panic!("performer contract channel broke"), 
                    }
                }
            }
        });
    }

    
}
