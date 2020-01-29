extern crate rand;
use super::block::{Block, Transaction};
use super::hash::{H256};
use super::message::ApiMessage as ServerApiMessage;
use std::{thread, time};
use mio_extras::channel::{self};
use std::sync::mpsc::{self, TryRecvError};
use rand::Rng;


pub struct Manager {
    control_receiver: mpsc::Receiver<ManagerMessage>,
    control_sender: mpsc::Sender<ManagerMessage>,
    server_api_sender: channel::Sender<ServerApiMessage>, 
    num_miner: u8,
}

#[derive(Clone, Debug)]
pub enum ManagerMessage {
    Start,
    Stop,
    Data(Block),
    Success(Block),
    Fail,
}

#[derive(Clone, Debug)]
pub enum MinerMessage {
    Data(Block),
}

pub enum ConnectResult {
    Success,
    Fail,
}

impl Manager {
    pub fn new(
        server_api_sender: channel::Sender<ServerApiMessage>
    ) -> (Manager, mpsc::Sender<ManagerMessage>) {
        let (control_sender, control_receiver) = mpsc::channel(); 
        let miner_manager = Manager {
            control_receiver: control_receiver, 
            control_sender: control_sender.clone(),
            server_api_sender: server_api_sender,
            num_miner: 1,

        }; 
        (miner_manager, control_sender)
    }

    pub fn start(mut self) {
        let _hadnler = thread::spawn(move || {
            self.manage_mining();  
        }); 
    }

    fn manage_mining(&mut self) {
        let (miner_context, message_sender) = MinerContext::new(self.control_sender.clone());
        miner_context.start();
        println!("started mining manager");
        loop {
            let received = self.control_receiver.recv();
            match received {
                Ok(message) => {
                    match message {
                        ManagerMessage::Start => {
                            //message_sender.send(MinerMessage::Start);
                        },
                        ManagerMessage::Stop => {
                            //message_sender.send(MinerMessage::Stop);
                        },
                        ManagerMessage::Data(block) => {
                            //mine block 
                            message_sender.send(MinerMessage::Data(block));
                        },
                        ManagerMessage::Success(block) => {
                            // construct block and send result to somewhere
                            self.server_api_sender.send( ServerApiMessage::MinedBlock(block)).unwrap();
                        },
                        ManagerMessage::Fail => {
                            println!("mining fails"); 
                        },
                    } 
                },
                Err(ref e)  => {
                    println!("mining manager try receive {:?}", e); 
                },
            }
        } 
    }
}

pub struct MinerContext {
    has_block: bool,
    block: Option<Block>,
    control_sender: mpsc::Sender<ManagerMessage>,
    message_receiver: mpsc::Receiver<MinerMessage>,
    threshold: u8, //TODO
}

impl MinerContext {
    fn new(
        control_sender: mpsc::Sender<ManagerMessage>
    ) -> (MinerContext, mpsc::Sender<MinerMessage>) {
        let (message_sender, message_receiver) = mpsc::channel();
        let miner_context = MinerContext {
            has_block: false,
            block: None, 
            control_sender: control_sender,
            message_receiver: message_receiver,
            threshold: 250,
        };
        (miner_context, message_sender)
    }

    fn start(mut self) {
        thread::spawn(move || {
            self.mining();          
        });
    }

    fn mining(&mut self) {
        loop {
            // nonce better be random
            let nonce: H256 = H256::default();
            let received = self.message_receiver.try_recv();
            match received {
                Ok(message) => {
                    match message {
                        MinerMessage::Data(block) => {
                            //mine block
                            self.block = Some(block); 
                            self.has_block = true;
                            
                        },
                    } 
                },
                Err(ref e) => {
                    match e {
                        TryRecvError::Empty => (),
                        TryRecvError::Disconnected => println!("channel disconnected {}", e), 
                    } 
                } 
            }

            // mine stuff
            if self.has_block {
                match &mut self.block {
                    Some(block) => {
                        let mut block = block;
                        let new_hash = block.update_nonce(H256::new());
                        let num = rand::thread_rng().gen_range(0, 50);
                        let sleep_time = time::Duration::from_millis(num);
                        thread::sleep(sleep_time);
                        
                        if new_hash.0[0] > self.threshold {
                            println!("Mined block");
                            self.control_sender.send(ManagerMessage::Success(block.clone()));
                            self.has_block = false;
                        }
                    },
                    None => unreachable!(),
                } 
            }
        } 
    }

    fn check_threshold(&mut self, hash: &H256) -> bool {
        true
    }
}
