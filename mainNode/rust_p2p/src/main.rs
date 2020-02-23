#[macro_use]
extern crate clap;
 #[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;
#[macro_use(lazy_static)]
extern crate lazy_static;
mod network;
mod api;
mod crypto;
mod primitive;
mod db;
mod blockchain;
mod contract;
mod experiment;
mod mempool;


use std::{thread, time};
use std::sync::mpsc::{self};
use mio_extras::channel::{self};
use clap::{Arg, App, SubCommand};
use std::fs::File;
use std::io::{BufRead, BufReader};
use network::message::{ServerSignal, ConnectResult, ConnectHandle};
use network::message::{Message};
use network::performer::{Performer};
use network::peer::{PeerContext};
use network::scheduler::{Scheduler};
use db::blockDb::{BlockDb};
use blockchain::blockchain::{BlockChain};
use mempool::mempool::{Mempool};
use contract::contract::{Contract, Account};
use std::sync::{Arc, Mutex};
use api::apiServer::ApiServer;
use experiment::transactionGenerator::{TransactionGenerator};
use primitive::block::{Transaction};
use rand::Rng;
use std::net::{SocketAddr};
use crossbeam::channel as cbchannel;
use log::{info, warn, error, debug};

use contract::interface::{Handle, Answer};
use contract::interface::Message as ContractMessage;
use contract::interface::Response as ContractResponse;




fn main() {
    env_logger::init();
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Bowen Xue.<bx3@uw.edu>")
        (about: "simple blockchain network")
        (@arg neighbor: -n --neighbor +takes_value "Sets ip to connect to")
        (@arg ip: -i --ip  +takes_value "Sets ip to listen")
        (@arg port: -p --port  +takes_value "Sets port to listen")
        (@arg account: -d --account  +takes_value "Sets account address")
        (@arg api_port: -a --api_port  +takes_value "Sets port for api")
    )
    .get_matches();

    let listen_port: String = matches.value_of("port").expect("missing port").to_string();
    let listen_ip: String = matches.value_of("ip").expect("missing ip").to_string();
    let listen_socket: SocketAddr = (listen_ip + ":" + &listen_port).parse().unwrap();
    let api_port: String = matches.value_of("api_port").expect("missing api port").to_string();
    let neighbor_path = matches.value_of("neighbor").expect("missing neighbor file");
    let account_path = matches.value_of("account").expect("missing account file");

    // get accounts
    let file = File::open(account_path).unwrap();
    let account: Account = serde_json::from_reader(file).expect("deser account");
    let api_socket: SocketAddr = ("127.0.0.1:".to_string() + &api_port).parse().unwrap();

    // get main data structures 
    let block_db = Arc::new(Mutex::new(BlockDb::new()));
    let blockchain = Arc::new(Mutex::new(BlockChain::new()));

    let (task_sender, task_receiver) =cbchannel::unbounded();
    let (server, server_control_sender) = network::server::Context::new(
        task_sender.clone(), 
        listen_socket
    );
    server.start();

    let (contract_handle_sender, contract_handle_receiver) = cbchannel::unbounded();
    let mempool = Arc::new(Mutex::new(Mempool::new(contract_handle_sender.clone())));
    let contract = Contract::new(
        account, 
        task_sender.clone(),
        server_control_sender.clone(),
        contract_handle_receiver,
        mempool.clone(),
        blockchain.clone(),
        block_db.clone(),
    ); 


    contract.start();
    sync_chain(contract_handle_sender.clone());

    // create main actors
    let mut performer = Performer::new(
        task_receiver, 
        blockchain.clone(), 
        block_db.clone(),
        mempool.clone(),
        contract_handle_sender.clone(),
    );
    performer.start();



    



    let (tx_gen, tx_control_sender) = TransactionGenerator::new(mempool.clone());
    tx_gen.start();

    ApiServer::start(
        api_socket, 
        tx_control_sender, 
        mempool.clone(), 
        contract_handle_sender.clone(), 
        blockchain.clone(),
        block_db.clone(),
    );

    // connect to peer
    let f = File::open(neighbor_path).expect("Unable to open file");
    let f = BufReader::new(f);

    let mut neighbors: Vec<String> = vec![];
    for line in f.lines() {
        let line = line.expect("Unable to read line");
        neighbors.push(line.to_string());
    }

    let mut num_connected = 0;
    for neighbor in neighbors.iter() {
        let (sender, receiver) = mpsc::channel();
        let connect_handle = ConnectHandle {
            result_sender: sender,
            dest_addr: neighbor.clone(),
        };
        let mut attempt = 0;
        loop {
            attempt += 1;
            info!("{} send ServeSignal::ServerConnect to {:?}. attempt {}", listen_socket, neighbor, attempt);
            server_control_sender.send(ServerSignal::ServerConnect(connect_handle.clone()));           
            match receiver.recv() {
                Ok(result) => {
                    match result {
                        ConnectResult::Success => {
                            info!("connect success");
                            break;
                        },
                        ConnectResult::Fail => {
                            info!("ConnectResult::Fail {:?}", neighbor);
                        },
                    } 
                },
                Err(e) => info!("receive error {:?}", e),
            }
            let sleep_time = time::Duration::from_millis(500);
            thread::sleep(sleep_time);
        }
    }
   

    /*
    let mut text_i = 0;
    loop {
        if true {
            text_i += 1;
            let text = String::from("hello") + &(text_i).to_string(); 
            let performer_message = Message::Ping(text);
            let control_message = ServerSignal::ServerBroadcast(performer_message);
            server_control_sender.send(control_message).expect("broadcast to peer");

            // web 3
            // 1. interact with smartContract
             
            // 2. Broadcast to all peers

            
            // sleep
            let num = rand::thread_rng().gen_range(0, 5000);
            let sleep_time = time::Duration::from_millis(1000); //num
            thread::sleep(sleep_time);
        }
    }
    */
    thread::park();
}

pub fn sync_chain(contract_channel: cbchannel::Sender<Handle>) -> usize {
    let (answer_tx, answer_rx) = cbchannel::bounded(1);
    let handle = Handle {
        message: ContractMessage::SyncChain,
        answer_channel: Some(answer_tx),
    };
    contract_channel.send(handle);
    let chain_len = match answer_rx.recv() {
        Ok(answer) => {
            match answer {
                Answer::Success(response) => {
                    match response {
                        ContractResponse::SyncChain(chain_len) => chain_len,
                        _ => {
                            panic!("answer to GetMainNodes: invalid response type");
                        },
                    }
                },
                Answer::Fail(reason) => {
                    panic!("sync chain failure");
                },
            }
        },
        Err(e) => {
            panic!("main contract channel broke");
        },
    };   
    chain_len
}
