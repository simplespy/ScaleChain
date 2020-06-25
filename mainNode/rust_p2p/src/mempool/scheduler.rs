use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::net::{SocketAddr};
use super::mempool::{Mempool};
use super::message::{Message, ServerSignal};
use super::blockchain::{BlockChain};
use mio_extras::channel::Sender as MioSender;
use crossbeam::channel::{Receiver};
use std::{thread, time};
use super::cmtda::{BlockHeader, Block, H256, BLOCK_SIZE, HEADER_SIZE, Transaction, read_codes};
use super::contract::utils;
use ser::{deserialize, serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub version: usize,
    pub ring_size: usize,
    pub node_list: Vec<SocketAddr>,
}

pub struct Scheduler {
    pub socket: SocketAddr,
    pub token: Option<Token>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub server_control_sender: MioSender<ServerSignal>,
    pub handle: Receiver<Option<Token>>,
    pub chain: Arc<Mutex<BlockChain>>, 
}

impl Scheduler {
    pub fn new(
        socket: SocketAddr,
        token: Option<Token>,
        mempool: Arc<Mutex<Mempool>>,
        server_control_sender: MioSender<ServerSignal>,
        handle: Receiver<Option<Token>>,
        chain: Arc<Mutex<BlockChain>>
    ) -> Scheduler {
        Scheduler {
            socket,
            token,
            mempool,
            server_control_sender,
            handle,
            chain: chain,
        }
    }

    // to participate a token ring group
    pub fn register_token(&mut self) -> bool {
        if let Some(ref mut token) = self.token {
            token.ring_size += 1;
            token.node_list.push(self.socket.clone());
            return true;
        } else {
            return false;
        }
    }

    pub fn start(mut self) {
        let _ = std::thread::spawn(move || {
            loop {
                match self.handle.recv() {
                    // mempool query
                    Ok(v) => {
                        match v {
                            None => {
                                match self.token.as_mut() {
                                    None => (),
                                    Some(token) => {
                                        self.propose_block();
                                    },
                                }
                            },
                            Some(token) => {
                                info!("reiceive a token, propose a block");
                                self.token = Some(token);
                                self.propose_block();
                            }
                        }
                    },
                    Err(e) => info!("scheduler error"),
                }
            }
        });
    } 
     

    pub fn propose_block(&mut self) -> bool {
        // pass token, drop token
        if let Some(ref mut token) = self.token {
            info!("{:?} scheduler propose block", self.socket);
            let mut mempool = self.mempool.lock().unwrap();
            let header = match mempool.prepare_cmt_block() {
                Some(h) => h,
                None => return false,
            };
            drop(mempool);
            let header_bytes = serialize(&header);

            let header_message: Vec<u8> = header_bytes.into(); //
            //let back: BlockHeader = deserialize(&header_message as &[u8]).unwrap();

            // get block id
            let chain = self.chain.lock().unwrap();
            let tip_state = chain.get_latest_state().unwrap();
            drop(chain);

            
           
            // broadcast block to scalenode
            //let random_header = utils::_generate_random_header();
            let message =  Message::ProposeBlock((header_message, tip_state.block_id)); //random_header
            let signal = ServerSignal::ServerBroadcast(message);
            self.server_control_sender.send(signal);

            // new side chain message

            // Pass token
            info!("{:?} scheduler sleep", self.socket);
            let sleep_time = time::Duration::from_millis(10000);
            thread::sleep(sleep_time);
            info!("{:?} scheduler waked up", self.socket);
            if token.ring_size >= 2 {
                let mut index = 0;
                for sock in &token.node_list {
                    if *sock == self.socket {
                        let next_index = (index + 1) % token.ring_size;
                        //info!("next_index {}", next_index);
                        let next_sock = token.node_list[next_index];
                        //info!{"next sock {:?}", next_sock};
                        let message = Message::PassToken(token.clone());
                        let signal = ServerSignal::ServerUnicast((next_sock, message));
                        self.server_control_sender.send(signal);
                        break;
                    }
                    index = (index + 1) % token.ring_size;
                }
            }
            self.token = None;
            return true;
        } else {
            return false;
        }
    }
}
