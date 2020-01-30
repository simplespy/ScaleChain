use super::primitive::hash::{H256};
use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::Address;
use web3::futures::Future;

#[derive(Debug, Default, Copy, Clone)]
pub struct ContractState {
    pub curr_hash: H256,
    pub block_id: usize,
}

pub struct Contract {
    contract: EthContract<web3::transports::Http>,
    contract_address: Address,
    rpc_url: String,
    my_account: Account, 
    contract_state: ContractState,
}

#[derive(Debug)]
pub struct Account {
    account: Address,
    //private_key: 
}

impl Contract {
    // TODO https://github.com/tomusdrw/rust-web3/blob/master/examples/contract.rs
    pub fn new() -> Contract {
        let rpc_url = "hello".to_string();
        let (eloop, http) = web3::transports::Http::new(&rpc_url).unwrap();
        eloop.into_remote();
        unimplemented!()
        //Contract()
    }
}
