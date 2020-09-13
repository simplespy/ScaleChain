use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::net::{SocketAddr};
use super::mempool::{Mempool};
use super::message::{Message, ServerSignal};
use super::blockchain::{BlockChain};
use mio_extras::channel::Sender as MioSender;
use crossbeam::channel::{Receiver, Sender, self};
use std::{thread, time};
use super::cmtda::{BlockHeader, Block, H256, BLOCK_SIZE, HEADER_SIZE, Transaction, read_codes};
use super::contract::utils;
use ser::{deserialize, serialize};
use super::contract::interface::{Handle, Answer};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Response as ContractResponse;
use crypto::sha3::Sha3;
use crypto::digest::Digest;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use web3::types::Address;
use crate::experiment::snapshot::PERFORMANCE_COUNTER;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub version: usize,
    pub ring_size: usize,
    pub node_list: Vec<SocketAddr>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Signal {
    Data(Token),
    Control,
}

pub struct Scheduler {
    pub addr: SocketAddr, //p2p
    pub token: Option<Token>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub server_control_sender: MioSender<ServerSignal>,
    pub contract_handler: Sender<Handle>,
    pub handle: Receiver<Signal>,
    pub chain: Arc<Mutex<BlockChain>>, 
    //pub side_id: u64,
    pub sidenodes: Vec<SocketAddr>,
    pub address: Address,
    pub slot_time: u64, 
    pub start_sec: u64, 
    pub start_millis: u64,
}

impl Scheduler {
    pub fn new(
        addr: SocketAddr,
        token: Option<Token>,
        mempool: Arc<Mutex<Mempool>>,
        server_control_sender: MioSender<ServerSignal>,
        handle: Receiver<Signal>,
        chain: Arc<Mutex<BlockChain>>,
        contract_handler: Sender<Handle>,
        //side_id: u64,
        sidenodes: Vec<SocketAddr>,
        address: Address,
        slot_time: u64,
        start_sec: u64,
        start_millis: u64,
    ) -> Scheduler {
        Scheduler {
            addr,
            token,
            mempool,
            server_control_sender,
            contract_handler,
            handle,
            chain: chain,
            //side_id,
            sidenodes,
            address,
            slot_time: slot_time,
            start_sec: start_sec,
            start_millis: start_millis,
        }
    }

    // to participate a token ring group
    pub fn register_token(&mut self) -> bool {
        if let Some(ref mut token) = self.token {
            token.ring_size += 1;
            token.node_list.push(self.addr.clone());
            return true;
        } else {
            return false;
        }
    }

    

    //pub fn start(mut self) {
        //let _ = std::thread::spawn(move || {
            //loop {
                //match self.handle.recv() {
                    //Ok(v) => {
                        //match v {
                            //Signal::Control => {
                                //// try transmit
                                //match self.token.as_mut() {
                                    //None => (),
                                    //Some(token) => {
                                        ////info!("with token, propose a block");
                                        //self.propose_block();
                                    //},
                                //}
                            //},
                            //Signal::Data(token) => {
                                ////info!("reiceive a token, propose a block");
                                //self.token = Some(token);
                                //self.propose_block();
                            //}
                        //}
                    //},
                    //Err(e) => info!("scheduler error"),
                //}
            //}
        //});
    //}

    //pub fn get_time_diff() {
        //let curr_time: u64 = match time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            //Ok(n) => n.as_secs(),
            //Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        //};
         
    //}
    //

    pub fn get_side_id(&self) -> u64 {
        match self.sidenodes.
            iter().
            position(|&x| x== self.addr) 
        {
            Some(i) => i as u64,
            None => panic!("my socketaddr is not included in the side nodes ring"),
        }
    }

    pub fn start(mut self) {
        let _ = std::thread::spawn(move || {
            loop {
                let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
                let round = self.sidenodes.len() as u64;
                let round_millis = round * self.slot_time * 1000;

                let curr_id = curr_slot % round;
                let side_id = self.get_side_id();     

                // my slot
                if curr_id == side_id {
                    PERFORMANCE_COUNTER.record_token_update(true);
                    if self.propose_block() {
                        let (curr_slot, elapsed) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
                        // go over the deadline
                        if curr_slot%round != side_id {
                            continue;
                        } else {
                            // to next slot
                            let target_time = ((side_id+1) * self.slot_time)*1000 - elapsed%round_millis;
                            thread::sleep(time::Duration::from_millis(target_time));
                        }
                    } else {
                        // retry again
                        thread::sleep(time::Duration::from_millis(500));
                    }
                } else if curr_id < side_id {
                    PERFORMANCE_COUNTER.record_token_update(false);
                    let target = side_id*self.slot_time* 1000 - elapsed%round_millis;
                    thread::sleep(time::Duration::from_millis(target));
                } else {
                    PERFORMANCE_COUNTER.record_token_update(false);
                    let time_left = round*self.slot_time*1000 - elapsed%round_millis + side_id*self.slot_time*1000;
                    thread::sleep(time::Duration::from_millis(time_left));
                }
            }
        });
    }

    //let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    //let round_time = self.slot_time* (self.sidenodes.len() as u64);
    //let curr_id = (curr_time.as_secs() % round_time) / self.slot_time;
    //let side_id = self.get_side_id();     
    //// my slot
    //if curr_id == side_id {
        //PERFORMANCE_COUNTER.record_token_update(true);
        //if self.propose_block() {
            //let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(); 
            //let r_time = curr_time.as_secs() % round_time ;
            //let ceil_time = (( r_time/self.slot_time + 1) as u64) *self.slot_time;
            //if ceil_time <= r_time {
                //continue;
            //} else {
                //let n_time = (ceil_time - r_time as u64) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
                //thread::sleep(time::Duration::from_nanos(n_time));
            //}
        //} else {
            //thread::sleep(time::Duration::from_millis(500));
        //}
    //} else if curr_id < side_id {
        //PERFORMANCE_COUNTER.record_token_update(false);
        //let time_left = ((side_id)*self.slot_time - (curr_time.as_secs() % round_time)) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
        //let sleep_sec = time::Duration::from_nanos(time_left);
        //thread::sleep(sleep_sec);
    //} else {
        //PERFORMANCE_COUNTER.record_token_update(false);
        //let time_left = (round_time - (curr_time.as_secs() % round_time)+ side_id*self.slot_time) * 1_000_000_000 - curr_time.subsec_nanos() as u64;
        //let sleep_sec = time::Duration::from_nanos(time_left);
        //thread::sleep(sleep_sec);
    //}

    fn pass_token(&mut self, token: Token) {
        info!("{:?} passing token.", self.addr);
        if token.ring_size >= 2 {
            let mut index = 0;
            for sock in &token.node_list {
                if *sock == self.addr {
                    let next_index = (index + 1) % token.ring_size;
                    let next_sock = token.node_list[next_index];
                    let message = Message::PassToken(token);
                    let signal = ServerSignal::ServerUnicast((next_sock, message));
                    self.server_control_sender.send(signal);
                    break;
                }
                index = (index + 1) % token.ring_size;
            }
        } else {
            let sleep_time = time::Duration::from_millis(1000);
            thread::sleep(sleep_time);
            self.propose_block();
        }
    }

    
    pub fn propose_block(&mut self) -> bool {
        // get curr state
        let (answer_tx, answer_rx) = channel::bounded(1);
        let handle = Handle {
            message: ContractMessage::GetCurrState(0),
            answer_channel: Some(answer_tx),
        };
        self.contract_handler.send(handle);
        let tip_state = match answer_rx.recv() {
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
        };
        
        // construct message and broadcast 
        let (curr_slot, elapsed)= get_curr_slot(self.start_sec, self.start_millis, self.slot_time);
        let new_block_id =  curr_slot + 1;
        info!("new block id {}", new_block_id);

        let tx_thresh = BLOCK_SIZE as usize / 316 - 1000;
        // generate a coded block
        let mut mempool = self.mempool.lock().unwrap();
        let num_tx = mempool.get_num_transaction();
        if num_tx <= tx_thresh  { // transation size
            info!("{:?} Skip: {} less than {:?}", self.addr, num_tx, tx_thresh); 
            return false;
        }

        PERFORMANCE_COUNTER.record_propose_block_update(new_block_id);

        let header = match mempool.prepare_cmt_block(new_block_id) {
            Some(h) => h,
            None => return false,
        };
        drop(mempool);

        let header_bytes = serialize(&header);
        let header_message: Vec<u8> = header_bytes.clone().into();

        let side_id = self.get_side_id();
        let (curr_slot, _) = get_curr_slot(self.start_sec, self.start_millis, self.slot_time); 
        let curr_id = curr_slot % self.sidenodes.len() as u64;
        if curr_id != side_id {
            info!("{:?} preempt take too long to construct block", self.addr);
            return false;
        }

        let hash_str = utils::hash_header_hex(&header_message);
        let message =  Message::ProposeBlock(
            self.addr, 
            new_block_id as u64, 
            header_message); 
        let signal = ServerSignal::ServerBroadcast(message);

        self.server_control_sender.send(signal);
        PERFORMANCE_COUNTER.record_propose_block_stop();

        let sleep_time = time::Duration::from_millis(1000);
        thread::sleep(sleep_time);

        let chain = self.chain.lock().unwrap();
        let tip_state = chain.get_latest_state().unwrap();
        drop(chain);

        true 
    }
}

// return slot and time elapsed as nano
// precision to millis, return curr_slot
pub fn get_curr_slot(start_sec: u64, start_millis: u64, slot_time: u64) -> (u64, u64) {
    let curr_time = time::SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let time_elapsed_millis = (curr_time.as_secs() - start_sec) *1000 + curr_time.subsec_millis() as u64 - start_millis;
    (time_elapsed_millis/(slot_time*1000), time_elapsed_millis)
}

