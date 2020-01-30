use std::sync::mpsc::{self};
use super::message::{Message, TaskRequest, PeerHandle};
use std::io::{self};
use mio_extras::channel::{self};
use std::thread;
use super::blockDb::{BlockDb};
use super::blockchain::blockchain::{BlockChain};
use super::contract::{Contract, ContractState};
use std::sync::{Arc, Mutex};

pub struct Performer {
    task_source: mpsc::Receiver<TaskRequest>,
    contract: Arc<Mutex<Contract>>,
    chain: Arc<Mutex<BlockChain>>, // may not use chain at all
    block_db: Arc<Mutex<BlockDb>>,
}

impl Performer {
    pub fn new(
        task_source: mpsc::Receiver<TaskRequest>, 
        contract: Arc<Mutex<Contract>>,
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
         
    ) -> Performer {
        Performer {
            task_source,
            contract: contract,
            chain: blockchain,
            block_db: block_db,
        } 
    }

    pub fn start(mut self) -> io::Result<()> {
        let handler = thread::spawn(move || {
            self.perform(); 
        }); 
        println!("Performer started");
        Ok(())
    }

    fn perform(&self) {
        loop {
            let task = self.task_source.recv().unwrap();
            match task.msg {
                Message::Ping(info_msg) => {
                    println!("receive Ping {}", info_msg);
                    // send Pong message
                    let response_msg = Message::Pong("pong".to_string());
                    assert!(task.peer.is_some());
                    task.peer.unwrap().response_sender.send(response_msg);
                }, 
                Message::Pong(info_msg) => {
                    println!("receive Pong {}", info_msg);                  
                },
                Message::SyncBlock(main_node_block) => {
                    let contract_state = ContractState {
                        curr_hash: main_node_block.curr_hash,
                        block_id: main_node_block.block_id,
                    };
                    // TODO check, curr_hash and block are coherent
                    // 1. compute curr_hash locally using all prev blocks stored in block_db
                    // 2. if some blocks are not present, use get_block function to retrieve from
                    //    eth archive node, need some search


                    let mut block_db = self.block_db.lock().unwrap();
                    block_db.insert(&main_node_block.block);
                    let mut chain = self.chain.lock().unwrap();
                    chain.insert(&contract_state);
                    println!("contract state {:?}", contract_state);
                },
            }
        } 
    }
}

