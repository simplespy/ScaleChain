use serde::{Serialize, Deserialize};
use mio_extras::channel::{self, Sender};
use std::sync::mpsc::{self};
use super::primitive::block::{Block, Transaction, EthBlkTransaction};
use super::scheduler::Token;
use std::net::{SocketAddr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    SyncBlock(EthBlkTransaction),
    SendTransaction(Transaction),
    PassToken(Token),
    ScaleProposeBlock(Block),
    ScaleReqChunks,
    ScaleGetChunks,
    ScaleGetAllChunks,
}


#[derive(Debug, Clone)]
pub struct ConnectHandle {
    pub result_sender: mpsc::Sender<ConnectResult>,
    pub dest_addr: String,
}

#[derive(Debug, Clone)]
pub enum ServerSignal {
    ServerConnect(ConnectHandle),
    ServerDisconnect, 
    ServerStop,
    ServerStart,
    ServerBroadcast(Message),
    ServerUnicast((SocketAddr, Message)),
}

#[derive(Clone)]
pub struct PeerHandle {
    pub response_sender: channel::Sender<Message>,   
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectResult {
    Success,
    Fail,
}

pub struct TaskRequest {
    pub peer: Option<PeerHandle>,
    pub msg: Message,
}
