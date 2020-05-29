use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::net::{SocketAddr};
use super::mempool::{Mempool};
use super::message::{Message, ServerSignal};
use mio_extras::channel::Sender as MioSender;
use crossbeam::channel::{Receiver};
use std::{thread, time};
use chain::block_header::{BlockHeader};

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
}

impl Scheduler {
    pub fn new(
        socket: SocketAddr,
        token: Option<Token>,
        mempool: Arc<Mutex<Mempool>>,
        server_control_sender: MioSender<ServerSignal>,
        handle: Receiver<Option<Token>>,
    ) -> Scheduler {
        Scheduler {
            socket,
            token,
            mempool,
            server_control_sender,
            handle,
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
                                //info!("reiceive a token, propose a block");
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
            info!("scheduler {:?} propose block", self.socket);
            let mut mempool = self.mempool.lock().unwrap();
            let block = match mempool.prepare_block() {
                None => return false,
                Some(block) => block,
            };

            // get CMT


            // broadcast block to scalenode
            let message =  Message::ScaleProposeBlock(block);

            // broadcast block without using CMT
            let signal = ServerSignal::ServerBroadcast(message);
            self.server_control_sender.send(signal);

            let sleep_time = time::Duration::from_millis(500);
            thread::sleep(sleep_time);
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
