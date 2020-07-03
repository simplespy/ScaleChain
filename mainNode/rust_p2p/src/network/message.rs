use serde::{Serialize, Deserialize};
use mio_extras::channel::{self, Sender};
use std::sync::mpsc::{self};
use super::primitive::block::{EthBlkTransaction};
use chain::transaction::Transaction;
use super::scheduler::Token;
use std::net::{SocketAddr};
use chain::{BlockHeader}; 
use super::cmtda::{Block, H256, BLOCK_SIZE, HEADER_SIZE, read_codes};
use ser::{deserialize, serialize};
use primitives::bytes::{Bytes};
use chain::decoder::{Symbol};
use chain::big_array::{BigArray};
use super::primitive::block::ContractState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkReply {
    pub header: Vec<u8>,
    pub symbols: Vec<Vec<Symbol>>,
    pub idx: Vec<Vec<u64>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Ping(String),
    Pong(String),
    SyncBlock(EthBlkTransaction),
    SendTransaction(Vec<u8>), //Transaction
    PassToken(Token),
    ProposeBlock((Vec<u8>, usize)), //BlockHeader block_id //sender is client
    ScaleReqChunks(Vec<u32>), //(id), // sender is scalenode
    ScaleReqChunksReply(ChunkReply),
    MySign(String, usize, usize, String, String, usize),
    ScaleGetAllChunks(ContractState), // blockheader
    ScaleGetAllChunksReply((Option<ChunkReply>, usize)),
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
    pub addr: SocketAddr,
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
