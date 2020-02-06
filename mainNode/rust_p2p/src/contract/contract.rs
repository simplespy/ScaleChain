use super::primitive::hash::{H256};
use super::primitive::block::{MainNodeBlock};

use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::Address;
use web3::futures::Future;

use std::thread;
use crossbeam::channel::{self, Sender, Receiver};
use super::interface::{Handle, Message, Response};

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
    pub fn new(account: Account) -> (Contract, Sender<Handle>) {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();

        let web3 = web3::api::Web3::new(http);
        //let contract = EthContract::new(web3.eth(), account.contract_address, );
        
        let (tx, rx) = channel::unbounded();

        let contract = Contract{
            //contract: ,
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
                        };
                    },
                    Err(e) => {
                        panic!("contract query channel");
                    }, 
                }
            }
        });
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

    pub fn count_main_nodes(&self, handle: Handle) -> Result<usize> {
        println!("count_main_nodes");
        let response = Response::Success(0);
        handle.answer_channel.unwrap().send(response);
        Ok(0)
    }

    pub fn add_main_node(&self) -> Result<()> {
        unimplemented!()
    }

    pub fn get_curr_state(&self) -> Result<ContractState> {
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
