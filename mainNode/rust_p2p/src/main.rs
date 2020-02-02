#[macro_use]
extern crate clap;
extern crate rand;
mod network;
mod api;
mod crypto;
mod primitive;
mod db;
mod blockchain;
mod contract;

use std::{thread, time};
use std::sync::mpsc::{self};
use mio_extras::channel::{self};
use clap::{Arg, App, SubCommand};
use std::fs::File;
use std::io::{BufRead, BufReader};
use network::message::{ServerApi, ConnectResult, ConnectHandle};
use network::message::{Message};
use network::performer::{Performer};
use db::blockDb::{BlockDb};
use blockchain::blockchain::{BlockChain};
use contract::{Contract, Account};
use std::sync::{Arc, Mutex};
use api::apiServer::ApiServer;
use api::transactionGenerator::{TransactionGenerator};
use primitive::block::{Transaction};
use rand::Rng;
use std::net::{SocketAddr};

fn main() {
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
    let contract = Arc::new(Mutex::new(Contract::new(account)));

    let (task_sender, task_receiver) = mpsc::channel();
    let (server_api_sender, server_api_receiver) = channel::channel();

    // create main actors
    let mut performer = Performer::new(
        task_receiver, 
        contract.clone(),
        blockchain.clone(), 
        block_db.clone()
    );
    performer.start();

    let mut server = network::server::Context::new(
        task_sender.clone(), 
        server_api_receiver, 
        listen_socket
        );
    server.start();

    ApiServer::start(api_socket);

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
            println!("{} send ServerApi::ServerConnect to {:?}. attempt {}", listen_socket, neighbor, attempt);
            server_api_sender.send(ServerApi::ServerConnect(connect_handle.clone()));           
            match receiver.recv() {
                Ok(result) => {
                    match result {
                        ConnectResult::Success => {
                            println!("connect success");
                            break;
                        },
                        ConnectResult::Fail => {
                            println!("ConnectResult::Fail {:?}", neighbor);
                        },
                    } 
                },
                Err(e) => println!("receive error {:?}", e),
            }
            let sleep_time = time::Duration::from_millis(500);
            thread::sleep(sleep_time);
        }
    }
   

    //let mut tx_gen = TransactionGenerator::new();
    let mut text_i = 0;
    loop {
        if true {
            text_i += 1;
            let text = String::from("hello") + &(text_i).to_string(); 
            let performer_message = Message::Ping(text);
            let control_message = ServerApi::ServerBroadcast(performer_message);
            server_api_sender.send(control_message).expect("broadcast to peer");

            // web 3
            // 1. interact with smartContract
             
            // 2. Broadcast to all peers

            
            // sleep
            let num = rand::thread_rng().gen_range(0, 500);
            let sleep_time = time::Duration::from_millis(num); //num
            thread::sleep(sleep_time);
        }
    }
    thread::park();
}
