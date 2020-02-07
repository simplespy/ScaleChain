use crossbeam::channel::{Sender};
use super::contract::{ContractState};

pub struct Handle {
    pub message: Message,
    pub answer_channel: Option<Sender<Answer>>,
}

pub enum Response {
    SendBlock,
    GetCurrState(ContractState),
    CountMainNode(usize), 
    AddMainNode,
    MainNodesList(Vec<web3::types::Address>),
}

pub enum Answer {
    Success(Response),
    Fail(String),
}

pub enum Message {
    SendBlock(Vec<u8>),
    GetCurrState,
    CountMainNodes,
    AddMainNode(web3::types::Address),
    GetMainNodes,
}
