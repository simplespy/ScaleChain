extern crate tiny_http;

use super::{TxGenSignal};
use super::mempool::mempool::{Mempool};
use super::blockchain::blockchain::BlockChain;
use super::contract::interface::{Message, Handle, Answer};
use super::contract::interface::Response as ContractResponse;
use crossbeam::channel::{self, Sender};
use std::thread;
use tiny_http::{Server, Response, Header};
use url::Url;
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use serde::{Serialize};
use super::network::message::{PeerHandle};
use super::network::message::Message as PerformerMessage;

pub struct ApiServer {
    addr: SocketAddr,
    tx_gen_control: Sender<TxGenSignal>,
    contract_channel: Sender<Handle>,
}

pub struct RequestContext {
    tx_control: Sender<TxGenSignal>,
    mempool: Arc<Mutex<Mempool>>,
    chain: Arc<Mutex<BlockChain>>,
    contract_channel: Sender<Handle>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let api_result = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let response = Response::from_string(serde_json::to_string_pretty(&api_result).unwrap())
            .with_header(content_type);
        $req.respond(response).unwrap();
    }};
}



impl ApiServer {
    pub fn start(socket: SocketAddr, 
                 tx_control: Sender<TxGenSignal>, 
                 mempool: Arc<Mutex<Mempool>>,
                 contract_channel: Sender<Handle>,
                 chain: Arc<Mutex<BlockChain>>,
    ) {
        let server = Server::http(&socket).unwrap();
        let _handler = thread::spawn(move || {
            for request in server.incoming_requests() {
                let rc = RequestContext {
                    tx_control: tx_control.clone(),
                    mempool: mempool.clone(),
                    chain: chain.clone(),
                    contract_channel: contract_channel.clone(),
                };
                // new thread per request
                let _ = thread::spawn(move || {
                    let url_path = request.url();
                    let mut url_base = Url::parse(&format!("http://{}/", &socket)).expect("get url base");
                    let url = url_base.join(url_path).expect("join url base and path");
                    
                    match url.path() {
                        "/transaction-generator/start" => {
                            rc.tx_control.send(TxGenSignal::Start);
                        },
                        "/transaction-generator/stop" => {
                            rc.tx_control.send(TxGenSignal::Stop);
                        },
                        "/transaction-generator/step" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let step = match pairs.get("step") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing step");
                                    return;
                                },
                            };
                            let step = match step.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "step needs to be numeric");
                                    return;
                                },
                            };
                            rc.tx_control.send(TxGenSignal::Step(step));
                            respond_result!(request, true, "ok");
                        },
                        "/transaction-generator/simulate" => {
                            rc.tx_control.send(TxGenSignal::Simulate);
                            respond_result!(request, true, "ok");
                        },
                        "/blockchain/get-curr-state" => {
                            println!("before /blockchain/get-curr-state lock" );
                            let mut chain = rc.chain.lock().expect("api get-curr-state");
                            println!("after /blockchain/get-curr-state lock" );
                            let state = chain.get_latest_state();
                            drop(chain);
                            respond_result!(request, true, format!("{:?}", state));
                        },
                        "/mempool/change-size" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let size = match pairs.get("size") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing size");
                                    return;
                                },
                            };
                            let size = match size.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    respond_result!(request, false, "size need to be numeric");
                                    return;
                                },
                            };
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            mempool.change_mempool_size(size);
                            drop(mempool);
                            respond_result!(request, true, format!("mempool size changed to {}", size));
                        },
                        "/mempool/num-transaction" => {
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            let num = mempool.get_num_transaction();
                            drop(mempool);
                            respond_result!(request, true, &num.to_string());
                        },
                        "/contract/send-block" => {
                            let mut mempool = rc.mempool.lock().expect("api send block");
                            mempool.send_block();
                            drop(mempool);
                            respond_result!(request, true, "ok");
                        },
                        "/contract/get-tx-receipt" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let hash = match pairs.get("hash") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing hash");
                                    return;
                                },
                            };
                            let a = hex::decode(hash).unwrap();
                            let tx_hash: &[u8] = a.as_ref();
                            let tx_hash = web3::types::H256::from_slice(tx_hash);

                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetTxReceipt(tx_hash),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let receipt = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::TxReceipt(receipt) => receipt,
                                                _ => {
                                                    panic!("answer to GetMainNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", receipt));
                        },
                        "/contract/count-main-nodes" => {
                            // USE CALLBACK
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::CountMainNodes,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let num_node = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::CountMainNode(num_main_node) => num_main_node,
                                                _ => {
                                                    panic!("answer to NumMainNode: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{}", num_node));
                        },
                        "/contract/get-curr-state" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetCurrState,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let curr_state = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::GetCurrState(curr_state) => curr_state,
                                                _ => {
                                                    panic!("answer to GetCurrState: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", curr_state));
                        },
                        "/contract/get-main-nodes" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetMainNodes,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let main_nodes = match answer_rx.recv() {
                                Ok(answer) => {
                                    match answer {
                                        Answer::Success(response) => {
                                            match response {
                                                ContractResponse::MainNodesList(main_nodes) => main_nodes,
                                                _ => {
                                                    panic!("answer to GetMainNodes: invalid response type");
                                                },
                                            }
                                        },
                                        Answer::Fail(reason) => {
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", main_nodes));
                        },
                        "/contract/add-main-node" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let address = match pairs.get("address") {
                                Some(s) => s,
                                None => {
                                    respond_result!(request, false, "missing address");
                                    return;
                                },
                            };
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let address = address.parse().unwrap();
                            let handle = Handle {
                                message: Message::AddMainNode(address),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                            let reply = Response::from_string(format!("Add mainNode {}", address));
                            request.respond(reply);
                        },
                        "/contract/get-all" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::GetAll(([0u8; 32], 0, 9999999)),
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
                        },
                        "/contract/sync-chain" => {
                            let (answer_tx, answer_rx) = channel::bounded(1);
                            let handle = Handle {
                                message: Message::SyncChain,
                                answer_channel: Some(answer_tx),
                            };
                            rc.contract_channel.send(handle);
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
                                            respond_result!(request, false, format!("contract query fails {}", reason));
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    respond_result!(request, false, format!("contract channel broken"));
                                    return;
                                },
                            };
                            respond_result!(request, true, format!("{:?}", chain_len));
                        },
                        _ => {
                            println!("all other option {:?}", url.path());
                        }
                    }

                    
                });

                
            }     
        });
    }
}


