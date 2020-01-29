use std::sync::mpsc::{self};
use super::message::{Message, TaskRequest, PeerHandle};
use std::io::{self};
use mio_extras::channel::{self};
use std::thread;
use super::blockDb::{BlockDb};
use super::blockchain::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};

pub struct Performer {
    task_source: mpsc::Receiver<TaskRequest>,
    chain: Arc<Mutex<BlockChain>>,
    block_db: Arc<Mutex<BlockDb>>,
}



impl Performer {
    pub fn new(
        task_source: mpsc::Receiver<TaskRequest>, 
        blockchain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
         
    ) -> Performer {
        Performer {
            task_source,
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
                Message::NewBlock(block) => {

                    let mut block_db = self.block_db.lock().unwrap();
                    block_db.insert(&block);
                    let mut chain = self.chain.lock().unwrap();
                    chain.insert(&block.header);
                    println!("blockchain height {}, latest hash {:?}", chain.get_height(), chain.latest_hash);
                },
                Message::NewTransaction(transaction) => {
                    // check if transactin is valid TODO
                    //
                    //
                    
                    //let mut mempool = self.mempool.lock().unwrap();
                    //mempool.insert(transaction);
                },
            }
        } 
    }
}

