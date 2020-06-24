use crossbeam::channel::{self, Sender, Receiver, TryRecvError};
use super::contract::interface::{Handle, Answer};
use super::primitive::block::ContractState;
use std::{thread, time};
use mio_extras::channel::Sender as MioSender;
use super::network::message::{Message, ServerSignal, ChunkReply};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
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
use crypto::digest::Digest;
use super::primitive::hash::H256;
use ser::{deserialize, serialize};

pub struct Manager {
    pub contract_handler: Sender<Handle>,
    pub chain: Arc<Mutex<BlockChain>>, 
    pub block_db: Arc<Mutex<BlockDb>>,
    pub server_control_sender: MioSender<ServerSignal>,
    pub addr: SocketAddr,
    pub job_sink: Receiver<(usize, ChunkReply)>,
    pub chunk_senders: HashMap<usize, Sender<ChunkReply> >,
    pub codes_for_encoding: Vec<Code>,
    pub codes_for_decoding: Vec<Code>,
    pub k_set: Vec<u64>,
}

pub struct JobManager {
    state: ContractState,
    addr: SocketAddr,
    server_control_sender: MioSender<ServerSignal>,
    chunk_receiver: Receiver<ChunkReply>,
    block_source: Sender<Result<SBlock, CodingErr>>,
    codes_for_encoding: Vec<Code>,
    codes_for_decoding: Vec<Code>,
    k_set: Vec<u64>,
}

// currently only handle one layer encoding
fn collect_cmt_chunks(job_manager: JobManager)
{
    info!("{:?} managing chunks", job_manager.addr);
    let chunk_receiver = &job_manager.chunk_receiver;
    let mut coll_symbols: Vec<Vec<Symbol>> = Vec::new();
    let mut coll_idx: Vec<Vec<u64>> = Vec::new();
    let mut is_first_recv = true;
    loop {
        // accumulate chunks
        match chunk_receiver.recv() {
            // accumulate chunks 
            Ok(chunk) => {
                info!("{:?} get all receive chunks", job_manager.addr);
                let header: BlockHeader = deserialize(&chunk.header as &[u8]).unwrap();;
                let mut hash  = [0u8; 32];
                let header_hex: String = hex::encode(&chunk.header);
                // get hash in the same way as bls
                let mut hasher = Sha3::keccak256();
                hasher.input_str(&header_hex);
                hasher.result(&mut hash);
                let hash: H256 = H256(hash);
                info!("header hash {:?}", hash);
                
                // compare if smart contract hash equals to local
                if hash != job_manager.state.curr_hash {
                    info!("{:?}, inconsistent hash {:?} {:?}", 
                          job_manager.addr,
                          hash, 
                          job_manager.state.curr_hash); 
                    continue;
                }
                
                let num_layer = job_manager.k_set.len();
                if is_first_recv {
                    is_first_recv = false;
                    coll_symbols = chunk.symbols.clone();
                    coll_idx = chunk.idx.clone();
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

                //let mut recon:Vec<u8> = vec![];
                //let systematic_symbol_len = k_set[0];
                //for i in 0..systematic_symbol_len {
                    //for j in 0..(coll_symbols[0].len()) {
                        //if idx[0][j] == i as u64 {
                            //match symbols[0][j] {
                                //Symbol::Base(s) => recon.extend_from_slice(&s),
                                //_ => unreachable!(),
                            //}
                        //}
                    //}
                //}

                //let mut trans_byte = cmt_block.transactions.iter().map(CMTTransaction::bytes).collect::<Vec<Bytes>>();
                //info!("trans_byte {:?}", trans_byte);

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
                let cmt_pass = true;
                if cmt_pass {
                    info!("{:?} cmt pass", job_manager.addr);
                    // collect all base + reconstruct block


                } else {
                    info!("{:?} cmt invalid", job_manager.addr);
                    // error handle
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
        job_sink: Receiver<(usize, ChunkReply)>,
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
            job_sink: job_sink,
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
            loop {
                let mut rm: Vec<usize> = vec![];
                for (block_id, block_sink) in &blocks_sink {
                    match block_sink.try_recv() {
                        Err(TryRecvError::Empty) => (),
                        Err(TryRecvError::Disconnected) => panic!("block sink broken"),
                        Ok(result) => {
                            // a thread has finished processing cmt
                            match result {
                                Ok(sblock) => {
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

                                    // test if update block chain
                                    for i in (tip_state.block_id+1) .. (*block_id+1) {
                                        match ready_blocks.get(&i) {
                                            None => info!("{:?} block {} is missing", self.addr, i),
                                            Some(s) => {
                                                local_chain.append(s);
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

                match self.job_sink.try_recv() {
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => panic!("manager sink broken"),
                    Ok((block_id, chunks)) => {
                        match self.chunk_senders.get(&block_id) {
                            None => info!("{:?} Error  no cmt get all request", self.addr),
                            Some(chunk_sender) => chunk_sender.send(chunks).unwrap(),
                        }
                    }
                }
                info!("{:?} pulling for mainchain update", self.addr);
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
                                                info!("manager found new state {:?} tip_state {:?}", state, tip_state);

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
                // every 1 sec pull mainchain state
                let sleep_time = time::Duration::from_millis(1000);
                thread::sleep(sleep_time);
            }
        });
    }

    
}