extern crate tiny_http;

use super::{TxGenSignal};
use super::mempool::mempool::{Mempool};
use super::contract::interface::{Message, Handle};
use super::contract::interface::Response as ContractResponse;
use std::sync::mpsc::{self};
use crossbeam::channel::{self, Sender};
use std::thread;
use tiny_http::{Server, Response, IncomingRequests};
use url::Url;
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};

pub struct ApiServer {
    addr: SocketAddr,
    tx_gen_control: Sender<TxGenSignal>,
    contract_channel: Sender<Handle>,
}

pub struct RequestContext {
    tx_control: Sender<TxGenSignal>,
    mempool: Arc<Mutex<Mempool>>,
    contract_channel: Sender<Handle>,
}

impl ApiServer {
    pub fn start(socket: SocketAddr, 
                 tx_control: Sender<TxGenSignal>, 
                 mempool: Arc<Mutex<Mempool>>,
                 contract_channel: Sender<Handle>) {
        let server = Server::http(&socket).unwrap();
        let _handler = thread::spawn(move || {
            for request in server.incoming_requests() {
                let rc = RequestContext {
                    tx_control: tx_control.clone(),
                    mempool: mempool.clone(),
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
                                    let response = Response::from_string("missing step");
                                    request.respond(response);                      
                                    return;
                                },
                            };
                            let step = match step.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    let response = Response::from_string("step needs to be numeric");
                                    request.respond(response);
                                    return;
                                },
                            };
                            rc.tx_control.send(TxGenSignal::Step(step));
                            let response = Response::from_string("Done");
                            request.respond(response);
                        },
                        "/mempool/change-size" => {
                            let mut pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
                            let size = match pairs.get("size") {
                                Some(s) => s,
                                None => {
                                    let response = Response::from_string("missing size ");
                                    request.respond(response);                      
                                    return;
                                },
                            };
                            let size = match size.parse::<usize>() {
                                Ok(s) => s,
                                Err(_) => {
                                    let response = Response::from_string("needs to be numeric");
                                    request.respond(response);
                                    return;
                                },
                            };
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            mempool.change_mempool_size(size);
                            drop(mempool);
                            let response = Response::from_string("changed mempool size");
                            request.respond(response);
                        },
                        "/mempool/num-transaction" => {
                            let mut mempool = rc.mempool.lock().expect("api change mempool size");
                            let num = mempool.get_num_transaction();
                            drop(mempool);
                            let response = Response::from_string(num.to_string());
                            request.respond(response);
                        },
                        "/mempool/send-block" => {
                            unimplemented!()
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
                                Ok(response) => {
                                    match response {
                                        ContractResponse::Success(n) => n,
                                        ContractResponse::Fail(reason) => {
                                            let reply = Response::from_string(format!("contract query fails {}", reason));
                                            request.respond(reply);
                                            return;
                                        },
                                    }
                                },
                                Err(e) => {
                                    let reply = Response::from_string("contract channel broken");
                                    request.respond(reply);
                                    return;
                                },
                            };
                            let reply = Response::from_string(format!("{}", num_node));
                            request.respond(reply);
                        },
                        _ => {
                            println!("all other option");
                        }

                    }

                    
                });

                
            }     
        });
    }
}


