use std::sync::mpsc::{self};
use super::message::{Message, TaskRequest, PeerHandle};
use std::io::{self};
use mio_extras::channel::{self};
use std::thread;
use super::blockDb::{BlockDb};
use super::blockchain::blockchain::{BlockChain};
use super::mempool::mempool::{Mempool};
use super::contract::contract::{Contract};
use super::contract::interface::Message as ContractMessage;
use super::contract::interface::Handle;
use super::primitive::block::ContractState;
use std::sync::{Arc, Mutex};
use crossbeam::channel::{Sender};

pub struct Performer {
    task_source: mpsc::Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>, 
    block_db: Arc<Mutex<BlockDb>>,
    mempool: Arc<Mutex<Mempool>>,
}

impl Performer {
    pub fn new(
        task_source: mpsc::Receiver<TaskRequest>, 
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
        mempool: Arc<Mutex<Mempool>>,
    ) -> Performer {
        Performer {
            task_source,
            chain: blockchain,
            block_db: block_db,
            mempool: mempool,
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
                    let response_msg = Message::Pong("pong".to_string());
                    task.peer.unwrap().response_sender.send(response_msg);
                }, 
                Message::Pong(info_msg) => {
                    println!("receive Pong {}", info_msg);                  
                },
                Message::SyncBlock(main_node_block) => {
                    let contract_state = main_node_block.contract_state;

                    // TODO check, curr_hash and block are coherent
                    // 1. compute curr_hash locally using all prev blocks stored in block_db
                    let mut chain = self.chain.lock().unwrap();
                    let latest_state = chain.get_latest_state();


                    // 2. if some blocks are not present, use get_block function to retrieve from
                    //    eth archive node, need some search

                    let mut block_db = self.block_db.lock().unwrap();
                    block_db.insert(&main_node_block.block);

                    chain.insert(&contract_state);
                    println!("contract state {:?}", contract_state);
                    drop(chain);
                },
                Message::SendTransaction(transaction) => {
                    let mut mempool = self.mempool.lock().expect("perform locl mempool");
                    mempool.insert(transaction);
                },
            }
        } 
    }
}

