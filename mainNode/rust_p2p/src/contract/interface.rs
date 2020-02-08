use crossbeam::channel::{Sender};
use super::primitive::block::{ContractState, Block};
use web3::types::{Address, H256, TransactionReceipt};

#[derive(Clone)]
pub struct Handle {
    pub message: Message,
    pub answer_channel: Option<Sender<Answer>>,
}
#[derive(Clone)]
pub enum Response {
    SendBlock,
    GetCurrState(ContractState),
    CountMainNode(usize), 
    AddMainNode,
    MainNodesList(Vec<Address>),
    TxReceipt(TransactionReceipt),
}
#[derive(Clone)]
pub enum Answer {
    Success(Response),
    Fail(String),
}
#[derive(Clone)]
pub enum Message {
    SendBlock(Block),
    GetCurrState,
    CountMainNodes,
    AddMainNode(Address),
    GetMainNodes,
    GetTxReceipt(H256),
}
