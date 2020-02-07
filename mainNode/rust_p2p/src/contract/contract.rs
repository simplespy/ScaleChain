use super::primitive::hash::{H256};
use super::primitive::block::{MainNodeBlock};

use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::Address;
use web3::futures::Future;

use std::thread;
use std::sync::mpsc;
use super::network::message::TaskRequest;
use crossbeam::channel::{self, Sender, Receiver};
use super::interface::{Handle, Message, Response};
use super::interface::Answer;

use std::io::{Result};
use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Copy, Clone)]
pub struct ContractState {
    pub curr_hash: H256,
    pub block_id: usize,
}

impl ContractState {
    pub fn genesis () -> ContractState {
        ContractState {
            curr_hash: H256::zero(),
            block_id: 0,
        }
    }
}


pub struct Contract {
    //contract: EthContract<web3::transports::Http>,
    my_account: Account, 
    contract_state: ContractState,
    contract_handle: Receiver<Handle>,
    performer_sender: mpsc::Sender<TaskRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    rpc_url: String,
    contract_address: String,
    address: Address,
    private_key: String,
    // java script 
}

impl Contract {
    // TODO https://github.com/tomusdrw/rust-web3/blob/master/examples/contract.rs
    pub fn new(
        account: Account, 
        performer_sender: mpsc::Sender<TaskRequest>
    ) -> (Contract, Sender<Handle>) {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();

        let web3 = web3::api::Web3::new(http);
        //let contract = EthContract::new(web3.eth(), account.contract_address, );
        
        let (tx, rx) = channel::unbounded();

        let contract = Contract{
            //contract: ,
            performer_sender: performer_sender,
            my_account: account, 
            contract_state: ContractState::genesis(),
            contract_handle: rx,
        };

        return (contract, tx);
    }

    pub fn start(mut self) {
        let _ = std::thread::spawn(move || {
            loop {
                match self.contract_handle.recv() {
                    //let _ = std::thread::spawn(move || {
                        Ok(handle) => {
                            match handle.message {
                                Message::SendBlock => {
                                    println!("send block");
                                    //self.send_block(); 
                                },
                                Message::AddMainNode => {
                                    println!("add man node");
                                    //self.add_main_node();
                                },
                                Message::CountMainNodes => {
                                    self.count_main_nodes(handle);
                                },
                                Message::GetCurrState => {
                                    self.get_curr_state(handle); 
                                },
                                //...
                            };
                        },
                        Err(e) => {
                            panic!("contract query channel");
                        }, 
                    //});
                }
            }
        });
    }

    // TODO
    fn get_curr_state(&self, handle: Handle) {
        // ethereum rust to get contract state
        let curr_state = ContractState {
            curr_hash: H256::default(),
            block_id: 0,   
        };

        let response = Response::GetCurrState(curr_state);
        let answer = Answer::Success(response);
        handle.answer_channel.unwrap().send(answer);
         
    }

    pub fn sync(&self) -> ContractState {
        // javascript wrapper function
        unimplemented!()
    }

    pub fn get_prev_blocks(&self, start: usize, end: usize) -> Vec<MainNodeBlock> {
        unimplemented!()
    }

    pub fn get_main_nodes(&self) -> Result<Vec<H256>> {
        unimplemented!()
    }

    pub fn count_main_nodes(&self, handle: Handle){
        println!("count_main_nodes");
        let num_main_node = 0;
        let response = Response::CountMainNode(num_main_node);
        let answer = Answer::Success(response);
        match handle.answer_channel.as_ref() {
            Some(ch) => (*ch).send(answer).unwrap(),
            None => panic!("contract count main node without answer channel"),
        }
    }

    pub fn add_main_node(&self) -> Result<()> {
        unimplemented!()
    }

    pub fn send_block(&self) -> Result<()> {
        unimplemented!()
    }

    // pull function to get updated, return number of state change, 0 for no change 
    pub fn sync_etherchain(&self) -> Result<usize> {
         unimplemented!()   
    }
}
