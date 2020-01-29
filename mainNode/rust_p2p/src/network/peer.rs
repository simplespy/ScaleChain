use std::net::{TcpListener, SocketAddr};
use mio::net::TcpStream;
use std::sync::mpsc::{self, Sender, Receiver, channel};
use super::message::{Message};
use mio_extras::channel::{self};
use std::collections::{VecDeque};
use super::message::{ApiMessage, ConnectResult, ConnectHandle, PeerHandle};
use log::{warn, info};
use super::MSG_BUF_SIZE;


pub struct PeerContext {
    pub addr: SocketAddr,
    pub stream: mio::net::TcpStream,
    pub peer_handle: PeerHandle,
    pub request: Message,
    pub connect_handle: Option<ConnectHandle>,
    pub is_connected: bool,
    pub direction: PeerDirection,
}

impl PeerContext {
    pub fn new(
        stream: mio::net::TcpStream,
        direction: PeerDirection,
    ) -> (PeerContext, channel::Receiver<Message>) {
        let (sender, receiver) = channel::channel();
        let peer_context = PeerContext {
            addr: stream.peer_addr().unwrap(),
            stream: stream,
            peer_handle: PeerHandle{response_sender: sender}, 
            request: Message::Ping("Default".to_string()),
            connect_handle: None,   
            is_connected: false,
            direction: direction,
        };
        (peer_context, receiver)
    }

    pub fn insert(&mut self, request: &[u8], len: usize) -> bool {
        if len == 0 {
            warn!("current request is empty"); 
            return false;
        }
        let mut request_with_size: Vec<u8> = vec![0; len];
        request_with_size.copy_from_slice(&request[0..len]);
        let decoded_msg: Message = bincode::deserialize(&request_with_size).expect("unable to decode msg");
        self.request = decoded_msg;
        true
    }

    pub fn send(&self, msg: Message) {
        self.peer_handle.response_sender.send(msg).expect("peer handle unable to send response"); 
    }
}

pub enum PeerDirection {
    Incoming,
    Outgoing,
}
