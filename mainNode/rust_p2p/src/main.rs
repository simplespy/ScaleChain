#[macro_use]
extern crate clap;

use std::{thread, time};
use std::sync::mpsc::{self};
use mio_extras::channel::{self};
use clap::{Arg, App, SubCommand};
use std::fs::File;
use std::io::{BufRead, BufReader};
use system_rust::network::message::{ServerSignal, ConnectResult, ConnectHandle, Message};
use system_rust::network::performer;

use system_rust::network::server;
use system_rust::mempool::scheduler::{Scheduler, Token};
use system_rust::db::blockDb::{BlockDb};
use system_rust::blockchain::blockchain::{BlockChain};
use system_rust::mempool::mempool::{Mempool};
use system_rust::contract::contract::{Contract, Account};
use std::sync::{Arc, Mutex};
use system_rust::api::apiServer::ApiServer;
use system_rust::experiment::transactionGenerator::{TransactionGenerator};
use rand::Rng;
use std::net::{SocketAddr};
use crossbeam::channel as cbchannel;
use log::{info, warn, error, debug};
use system_rust::mainChainManager::{Manager};
use system_rust::cmtda::{read_codes};
use chain::decoder::{Code};
use system_rust::contract::interface::{Handle, Answer};
use system_rust::contract::interface::Message as ContractMessage;
use system_rust::contract::interface::Response as ContractResponse;
use system_rust::contract::utils::{BLSKey, BLSKeyStr};

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
        (@arg has_token: -t --has_token "Sets init token")
        (@arg scale_id: -s --scale_id  +takes_value "Sets scalechain node")
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
    let has_token = matches.is_present("has_token");
    let mut scale_id: usize = matches.value_of("scale_id").expect("missing scaleid").parse::<usize>().unwrap();

    // config + get peers
    let api_socket: SocketAddr = ("127.0.0.1:".to_string() + &api_port).parse().unwrap();
    let neighbors = parse_addr_file(neighbor_path);
    let sidenodes = parse_addr_file(sidenodes_path);

    let is_scale_node: bool = (scale_id > 0);
    
    // get accounts
    info!("api port {:?}", api_port);
    let file = File::open(account_path).unwrap();
    let account: Account = serde_json::from_reader(file).expect("deser account");
    let key_file = File::open(key_path).unwrap();
    let key_str: BLSKeyStr = serde_json::from_reader(key_file).expect("deser key file");
    let key: BLSKey = BLSKey::new(key_str);
    

    // roles
    let block_db = Arc::new(Mutex::new(BlockDb::new()));
    let blockchain = Arc::new(Mutex::new(BlockChain::new()));

    let (task_sender, task_receiver) =cbchannel::unbounded();

    let (server_ctx, mut server_handle) = server::Context::new(
        task_sender.clone(), 
        listen_socket,
        is_scale_node,
    );
    server_ctx.start();

    let (schedule_handle_sender, schedule_handle_receiver) = cbchannel::unbounded();
    let (contract_handle_sender, contract_handle_receiver) = cbchannel::unbounded();
    let (manager_handle_sender, manager_handle_receiver) = cbchannel::unbounded();
    
    let k_set: Vec<u64> = vec![128,64,32,16,8,4];//vec![32,16,8];//  //128,64,32,16,8
    let (codes_for_encoding, codes_for_decoding) = read_codes(k_set.clone());
    let mempool = Arc::new(Mutex::new(Mempool::new(
        contract_handle_sender.clone(),
        schedule_handle_sender.clone(),
        listen_socket.clone(),
        codes_for_encoding.clone(),
        codes_for_decoding.clone(),
    )));

    let token = init_token(has_token, listen_socket.clone(), &sidenodes);

    let contract = Contract::new(
        account,
        key,
        task_sender.clone(),
        server_handle.control_tx.clone(),
        contract_handle_receiver,
        mempool.clone(),
        blockchain.clone(),
        block_db.clone(),
        listen_socket.to_string()
    );

    let manager = Manager::new(
        contract_handle_sender.clone(),
        blockchain.clone(),
        server_handle.control_tx.clone(),
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

    let scheduler = Scheduler::new(
        listen_socket.clone(), 
        token, 
        mempool.clone(), 
        server_handle.control_tx.clone(), 
        schedule_handle_receiver.clone(), 
        blockchain.clone(),
        contract_handle_sender.clone()
    );
    scheduler.start();
    contract.start();

    // create main actors
    let mut performer = performer::new(
        task_receiver, 
        blockchain.clone(), 
        block_db.clone(),
        mempool.clone(),
        schedule_handle_sender.clone(),
        contract_handle_sender.clone(),
        listen_socket.clone(),
        key_path.to_string(),
        scale_id,
        0,
        server_handle.control_tx.clone(),
        manager_handle_sender.clone(),
        (neighbors.len()-sidenodes.len()) as u64,
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
        let addr: SocketAddr = neighbor.to_string().parse().unwrap();
        loop {
            if addr == listen_socket {
                break;
            }
            match server_handle.connect(addr) {
                Ok(_) => {
                    break;
                },
                Err(e) => {
                    error!(
                        "Error connecting to peer {}, retrying in one second: {}",
                        addr, e
                    );
                    thread::sleep(time::Duration::from_millis(1000));
                    continue;
                }
            }
        }
    }
    thread::park();
}

pub fn init_token(
    has_token: bool, 
    listen_socket: SocketAddr, 
    sidenodes: &Vec<SocketAddr>
) -> Option<Token> {
    let mut token: Option<Token> = None;
    if has_token {
        let number_token = sidenodes.len();
        let mut tok = Token {
            version: 0,
            ring_size: 0,
            node_list: vec![],
        };
        for node in sidenodes.iter() {
            tok.ring_size += 1;
            tok.node_list.push(node.clone());
        }
        token = Some(tok);
    }
    token
}

pub fn parse_addr_file(filename: &str) -> Vec<SocketAddr> {
    let f = File::open(filename).expect("Unable to open file");
    let f = BufReader::new(f);

    let mut neighbors: Vec<SocketAddr> = vec![];
    for line in f.lines() {
        let addr = line.expect("file read error").parse().expect("unable to parse addr");
        neighbors.push(addr);
    }
    neighbors
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
