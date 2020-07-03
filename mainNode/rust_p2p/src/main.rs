#[macro_use]
extern crate clap;
 #[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;
#[macro_use(lazy_static)]
extern crate lazy_static;
extern crate serialization as ser;

#[macro_use]
extern crate serialization_derive;

mod network;
mod api;
mod crypto;
mod primitive;
mod db;
mod blockchain;
mod contract;
mod experiment;
mod mempool;
mod cmtda;
mod mainChainManager;



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
use mempool::scheduler::{Scheduler, Token};
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
use mainChainManager::{Manager};
use cmtda::{read_codes};
use chain::decoder::{Code};
use contract::interface::{Handle, Answer};
use contract::interface::Message as ContractMessage;
use contract::interface::Response as ContractResponse;
use contract::utils::{BLSKey, BLSKeyStr};



fn main() {
    env_logger::init();
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Bowen Xue.<bx3@uw.edu>")
        (about: "simple blockchain network")
        (@arg neighbor: -n --neighbor +takes_value "Sets ip to connect to")
        (@arg side_node: -o --side_node +takes_value "Sets side ip to connect to")
        (@arg ip: -i --ip  +takes_value "Sets ip to listen")
        (@arg port: -p --port  +takes_value "Sets port to listen")
        (@arg account: -d --account  +takes_value "Sets account address")
        (@arg key: -k --key  +takes_value "Sets key address")
        (@arg api_port: -a --api_port  +takes_value "Sets port for api")
        (@arg has_token: -t --has_token  +takes_value "Sets init token")
        (@arg scale_id: -s --scale_id  +takes_value "Sets scalechain node")
        (@arg threshold: -h --threshold  +takes_value "Sets threshold ")
    )
    .get_matches();

    let listen_port: String = matches.value_of("port").expect("missing port").to_string();
    let listen_ip: String = matches.value_of("ip").expect("missing ip").to_string();
    let listen_socket: SocketAddr = (listen_ip.clone() + ":" + &listen_port).parse().unwrap();
    let api_port: String = matches.value_of("api_port").expect("missing api port").to_string();
    let neighbor_path = matches.value_of("neighbor").expect("missing neighbor file");
    let sidenodes_path = matches.value_of("side_node").expect("missing side node file");
    let account_path = matches.value_of("account").expect("missing account file");
    let key_path = matches.value_of("key").expect("missing key file");
    let threshold: usize = matches.value_of("threshold").expect("missing threshold").parse::<usize>().unwrap();

    let has_token = matches.value_of("has_token").expect("missing token indicator");
    let mut scale_id: usize = matches.value_of("scale_id").expect("missing scaleid").parse::<usize>().unwrap();

    // connect to peer
    let f = File::open(neighbor_path).expect("Unable to open file");
    let f = BufReader::new(f);

    let side_file = File::open(sidenodes_path).expect("Unable to open file");
    let side_file = BufReader::new(side_file);
    

    let mut neighbors: Vec<String> = vec![];
    for line in f.lines() {
        let line = line.expect("Unable to read line").to_string();
        if line != listen_socket.clone().to_string() {
            println!("{:?} read neighbor {:?}", listen_socket, line);
            neighbors.push(line.to_string());
        }
    }

    let mut sidenodes: Vec<String> = vec![];
    for line in side_file.lines() {
        let line = line.expect("Unable to read line").to_string();
        if line != listen_socket.clone().to_string() {
            println!("{:?} read neighbor {:?}", listen_socket, line);
            sidenodes.push(line.to_string());
        }
    }


    // get accounts
    let file = File::open(account_path).unwrap();
    let account: Account = serde_json::from_reader(file).expect("deser account");
    let key_file = File::open(key_path).unwrap();
    let key_str: BLSKeyStr = serde_json::from_reader(key_file).expect("deser key file");
    let key: BLSKey = BLSKey::new(key_str);
    let api_socket: SocketAddr = ("127.0.0.1:".to_string() + &api_port).parse().unwrap();

    info!("listen port {:?} {:?}", listen_port, account );

    // get main data structures 
    let block_db = Arc::new(Mutex::new(BlockDb::new()));
    let blockchain = Arc::new(Mutex::new(BlockChain::new()));

    let (task_sender, task_receiver) =cbchannel::unbounded();
    let is_scale_node: bool = (scale_id >= 0);
    let (server, server_control_sender) = network::server::Context::new(
        task_sender.clone(), 
        listen_socket,
        is_scale_node,
    );
    server.start();

    let (contract_handle_sender, contract_handle_receiver) = cbchannel::unbounded();
    let (manager_handle_sender, manager_handle_receiver) = cbchannel::unbounded();
    let (schedule_handle_sender, schedule_handle_receiver) = cbchannel::unbounded();
    let k_set: Vec<u64> = vec![32,16,8]; //512,256,128, 64
    let (codes_for_encoding, codes_for_decoding) = read_codes(k_set.clone());
    let mempool = Arc::new(Mutex::new(Mempool::new(
                contract_handle_sender.clone(),
                schedule_handle_sender.clone(),
                listen_socket.clone(),
                codes_for_encoding.clone(),
                codes_for_decoding.clone(),
                )));
    let contract = Contract::new(
        account,
        key,
        task_sender.clone(),
        server_control_sender.clone(),
        contract_handle_receiver,
        mempool.clone(),
        blockchain.clone(),
        block_db.clone(),
        listen_socket.to_string()
    );

    let manager = Manager::new(
            contract_handle_sender.clone(),
            blockchain.clone(),
            server_control_sender.clone(),
            listen_socket.clone(),
            manager_handle_receiver,
            block_db.clone(),
            codes_for_encoding.clone(),
            codes_for_decoding.clone(),
            k_set.clone()
        );
    if scale_id == 0 {
        manager.start();
    }
    

    let mut token: Option<Token>;
    if has_token == "0" {
        token = None;
    } else {
        let nt: String  = has_token.to_string();
        let number_token = nt.parse::<usize>().unwrap();
        let mut tok = Token {
            version: 0,
            ring_size: 1,
            node_list: vec![listen_socket.clone()],
        };
        for neighbor in sidenodes.clone().iter() {
            info!("token len increment 1");
            tok.ring_size += 1;
            let dest: SocketAddr = neighbor.parse().unwrap();
            tok.node_list.push(dest);
            if tok.ring_size == number_token {
                break;
            }
        }
        token = Some(tok);
    }
    let scheduler = Scheduler::new(
        listen_socket.clone(), 
        token, mempool.clone(), 
        server_control_sender.clone(), 
        schedule_handle_receiver.clone(), 
        blockchain.clone(),
        contract_handle_sender.clone()
        );
    scheduler.start();

    contract.start();
    //sync_chain(contract_handle_sender.clone());

    // create main actors
    let mut performer = Performer::new(
        task_receiver, 
        blockchain.clone(), 
        block_db.clone(),
        mempool.clone(),
        schedule_handle_sender.clone(),
        contract_handle_sender.clone(),
        listen_socket.clone(),
        key_path.to_string(),
            scale_id,
        threshold,
        server_control_sender.clone(),
        manager_handle_sender.clone(),
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


    //server_control_sender.send(ServerSignal::ServerConnect(connect_handle.clone()));
   

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
