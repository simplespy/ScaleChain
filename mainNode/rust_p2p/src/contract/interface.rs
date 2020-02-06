use crossbeam::channel::{Sender};

pub struct Handle {
    pub message: Message,
    pub answer_channel: Option<Sender<Response>>,
}

pub enum Response {
    Success(usize),
    Fail(String),
}

pub enum Message {
    SendBlock,
    AddMainNode, 
    CountMainNodes,
}
