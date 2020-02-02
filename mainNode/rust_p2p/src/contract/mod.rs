use super::primitive::hash::{H256};
use super::primitive::block::{MainNodeBlock};

use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::Address;
use web3::futures::Future;

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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    rpc_url: String,
    contract_address: String,
    address: Address,
    private_key: String,
}

impl Contract {
    // TODO https://github.com/tomusdrw/rust-web3/blob/master/examples/contract.rs
    pub fn new(account: Account) -> Contract {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();

        let web3 = web3::api::Web3::new(http);
        //let contract = EthContract::new(web3.eth(), account.contract_address, );

        Contract{
            //contract: ,
            my_account: account, 
            contract_state: ContractState::genesis(),
        }
    }

    pub fn sync() -> ContractState {
        unimplemented!()
    }

    pub fn get_prev_blocks(start: usize, end: usize) -> Vec<MainNodeBlock> {
        unimplemented!()
    }

    pub fn get_main_nodes() -> Result<Vec<H256>> {
        unimplemented!()
    }

    pub fn count_main_nodes() -> Result<usize> {
        unimplemented!()
    }

    pub fn add_main_node() -> Result<()> {
        unimplemented!()
    }

    pub fn get_curr_state() -> Result<ContractState> {
        unimplemented!()
    }

    pub fn send_block() -> Result<()> {
        unimplemented!()
    }

    // pull function to get updated, return number of state change, 0 for no change 
    pub fn sync_etherchain() -> Result<usize> {
         unimplemented!()   
    }
}
