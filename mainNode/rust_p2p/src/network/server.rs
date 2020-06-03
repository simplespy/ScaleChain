extern crate log;
use mio::{Events, Poll, Ready, PollOpt, Token};
use mio::tcp::{TcpListener, TcpStream};
use std::net::{SocketAddr};
use std::collections::{HashMap};
use std::{thread, time};
use std::io::{self, Read, Write};
use super::peer::{PeerContext, PeerDirection};
use super::MSG_BUF_SIZE;
use super::message::{Message, ServerSignal, ConnectResult, ConnectHandle, TaskRequest};
use mio_extras::channel::{self, Receiver};
use crossbeam::channel as cbchannel;

use log::{info, warn};

// refer to https://sergey-melnychuk.github.io/2019/08/01/rust-mio-tcp-server/ 
// for context
const LISTENER: Token = Token(0);
const CONTROL: Token = Token(1);
const NETWORK_TOKEN: usize = 0;
const LOCAL_TOKEN: usize = 1;

const EVENT_CAP: usize = 1024;

pub struct Context {
    poll: mio::Poll,
    peers: HashMap<Token, PeerContext>,
    token_counter: usize,
    task_sender: cbchannel::Sender<TaskRequest>,
    response_receiver: HashMap<Token, channel::Receiver<Message>>,
    api_receiver: channel::Receiver<ServerSignal>,
    local_addr: SocketAddr,
    is_scale_node: bool,
}

impl Context {
    pub fn new(
        task_sender: cbchannel::Sender<TaskRequest>, 
        addr: SocketAddr, 
        is_scale_node: bool,
    ) -> (Context, channel::Sender<ServerSignal>) {
        let (control_tx, control_rx) = channel::channel();
        let context = Context{
            poll: Poll::new().unwrap(),
            peers: HashMap::new(),
            token_counter: 2, // 0, 1 token are reserved
            task_sender: task_sender,
            response_receiver: HashMap::new(),
            api_receiver: control_rx,
            local_addr: addr,
            is_scale_node: is_scale_node,
        };
        (context, control_tx)
    }
    
    // start a server, spawn a process
    pub fn start(mut self) {
        let _handler = thread::spawn(move || {
            self.listen(); 
        });
        info!("listener started");
    }

    // register tcp in the event loop
    // network read token i
    // local event token i + 1
    // token starts at 2
    pub fn register_peer(&mut self, socket: TcpStream, direction: PeerDirection) -> io::Result<Token> {
        let peer_addr = socket.peer_addr().unwrap();
        let network_token = Token(self.token_counter);
        self.token_counter += 1;
        
        self.poll.register(
            &socket, 
            network_token.clone(),
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        // create a peer context
        let (peer_context, event_rx) = PeerContext::new(socket, direction);
        let local_token = Token(self.token_counter);
        self.token_counter += 1;

        self.peers.insert(network_token, peer_context);

        self.poll.register(
            &event_rx,
            local_token,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        self.response_receiver.insert(local_token, event_rx);
        info!("{} registered peer {}, peer token {}, local token {}", self.local_addr, peer_addr, network_token.0, local_token.0); 

        Ok(network_token)
    }

    // create tcp stream for each peer
    pub fn connect(&mut self, connect_handle: ConnectHandle) -> io::Result<()> {
        let addr: SocketAddr = connect_handle.dest_addr.parse().unwrap();
        let timeout = time::Duration::from_millis(3000);
        let tcp_stream = match std::net::TcpStream::connect_timeout(&addr, timeout) {
            Ok(s) => s,
            Err(e) => {
                connect_handle.result_sender.send(ConnectResult::Fail);
                return Ok(());
            }
        };
        let stream = TcpStream::from_stream(tcp_stream)?;
        info!("{} connected to {} : {}", self.local_addr, addr, stream.local_addr().unwrap());
        let network_token = self.register_peer(stream, PeerDirection::Outgoing).unwrap();
        let mut peer = self.peers.get_mut(&network_token).unwrap();
        connect_handle.result_sender.send(ConnectResult::Success).expect("send connection result back");
        Ok(())
    }

    // polling events
    pub fn listen(&mut self) {
        let listener = TcpListener::bind(&self.local_addr).unwrap(); 
       
        self.poll.register(&listener, 
            LISTENER,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        self.poll.register(&self.api_receiver, 
            CONTROL,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        let mut events = Events::with_capacity(EVENT_CAP);
        let mut buf = [0; MSG_BUF_SIZE];
        loop {
            self.poll.poll(&mut events, None).expect("unable to poll events"); 
            for event in &events {
                match event.token() {
                    LISTENER => {
                        loop {
                            match listener.accept() { 
                                Ok((socket, socket_addr)) => {
                                    info!("tcp registered {} due to listen", socket_addr);
                                    self.register_peer(socket, PeerDirection::Incoming).expect("cannot register peer");
                                },
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    break; 
                                },
                                e => panic!("err={:?}", e),
                            }
                        }
                    },
                    CONTROL => {
                        loop {
                            let api_msg = self.api_receiver.try_recv();
                            match api_msg { 
                                Ok(msg) => {
                                    match msg {
                                        ServerSignal::ServerConnect(connect_handle) => {
                                            self.connect(connect_handle);
                                        },
                                        ServerSignal::ServerBroadcast(network_message) => {
                                            for (token, peer) in self.peers.iter() {
                                                match peer.direction {
                                                    PeerDirection::Incoming => (),
                                                    PeerDirection::Outgoing => {
                                                        peer.send(network_message.clone()); 
                                                    },
                                                }
                                            }
                                        },
                                        ServerSignal::ServerUnicast((socket, network_message)) => {
                                            for (token, peer) in self.peers.iter() { 
                                                if peer.addr == socket {
                                                    match peer.direction {
                                                        PeerDirection::Incoming => (),
                                                        PeerDirection::Outgoing => {
                                                            //info!("server unicast to {:?}", peer.addr);
                                                            peer.send(network_message.clone()); 
                                                        },
                                                    }
                                                }
                                            }
                                        },
                                        _ => warn!("ServerSignal not implemented yet"),
                                    }
                                    break;
                                },
                                e => warn!("api receiver Err {:?}", e),
                            }
                        }
                    },
                    token if event.readiness().is_readable() => {
                        let token_type: usize = token.0 % 2;
                        match token_type {
                            NETWORK_TOKEN => { 
                                loop {
                                    let mut peer = self.peers.get_mut(&token).expect("get peer fail"); 
                                    let read = peer.stream.read(&mut buf);
                                    match read {
                                        Ok(0) => {
                                            break; 
                                        }
                                        Ok(len) => {
                                            peer.insert(&buf, len);
                                            // send task request to performer
                                            let performer_task = TaskRequest{
                                                peer: Some(peer.peer_handle.clone()), 
                                                msg: peer.request.clone()
                                            };
                                            self.task_sender.send(performer_task).expect("send request to performer");
                                        },
                                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            break; 
                                        },
                                        e => {
                                            warn!("Connect fail. could not connect to {:?}", peer.addr);
                                            match peer.connect_handle {
                                                Some(ref handler) => {
                                                    //handler.result_sender.send(ConnectResult::Fail).expect("result sender fail");
                                                }
                                                _ => {
                                                    warn!("cannot find handler");
                                                },
                                            }
                                        },
                                    }
                                }
                            },
                            LOCAL_TOKEN => {
                                let peer_token = Token(token.0 - 1);
                                let peer = self.peers.get(&peer_token).expect("cannot get peer with local token"); 
                                self.poll.reregister(
                                        &peer.stream,
                                        peer_token,
                                        Ready::writable(),
                                        PollOpt::edge() | PollOpt::oneshot()
                                ).unwrap();
                            },
                            _ => unreachable!(),
                        }
                    },
                    token if event.readiness().is_writable() => {
                        let peer = self.peers.get_mut(&token).expect("writable cannot get peer"); 
                        let peer_token = Token(token.0 + 1);
                        let receiver = self.response_receiver.get(&peer_token).expect("response_receiver empty");
                        // check response receiver for message to write
                        loop {
                            let response_msg = receiver.try_recv();
                            match response_msg {
                                Ok(msg) => {
                                    let mut peer_stream = &peer.stream;
                                    let encoded_msg = bincode::serialize(&msg).expect("unable to encode msg");
                                    //let decoded_msg: Message = bincode::deserialize(&encoded_msg).expect("unable to encode msg");
                                    //println!("{:?}", decoded_msg);
                                    match peer_stream.write_all(encoded_msg.as_slice()) {
                                        Ok(()) => (), //println!("write ok"),
                                        _ => println!("unable to write stream"),
                                    };

                                    self.poll.reregister(
                                        peer_stream,
                                        token,
                                        Ready::readable(),
                                        PollOpt::edge()
                                    ).expect("unable to reregister");  
                                    break;
                                },
                                Err(e) => warn!("write try receive fails "),
                            }
                        }
                        
                    },
                    _ => unreachable!(),
                }
            }
        }
    }
}


