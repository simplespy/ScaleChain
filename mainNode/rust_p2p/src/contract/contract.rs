use super::primitive::block::{MainNodeBlock, ContractState, Block};
use super::primitive::hash::{H256};
use super::network::message::{ServerSignal, TaskRequest};
use super::network::message::Message as ServerMessage;
use super::mempool::mempool::{Mempool};
use super::interface::{Handle, Message, Response, Answer};

use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::{Address, Bytes};
use web3::futures::Future;

use crypto::sha3::Sha3;
use crypto::digest::Digest;
use crypto::sha2::Sha256;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{time};
use std::io::{Result};
use std::process::Command;

use crossbeam::channel::{Receiver};
use serde::{Serialize, Deserialize};
use ethereum_tx_sign::RawTransaction;

use secp256k1::{Secp256k1, SecretKey};
use mio_extras::channel::Sender as MioSender;
use requests::ToJson;

pub struct Contract {
    contract: EthContract<web3::transports::Http>,
    my_account: Account, 
    contract_state: ContractState,
    contract_handle: Receiver<Handle>,
    mempool: Arc<Mutex<Mempool>>,
    performer_sender: mpsc::Sender<TaskRequest>,
    server_control_sender: MioSender<ServerSignal>,
    web3: web3::api::Web3<web3::transports::Http>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    rpc_url: String,
    contract_address: Address,
    address: Address,
    private_key: String,
}

impl Contract {
    pub fn new(
        account: Account, 
        performer_sender: mpsc::Sender<TaskRequest>,
        server_control_sender: MioSender<ServerSignal>,
        contract_handle: Receiver<Handle>, 
        mempool: Arc<Mutex<Mempool>>,
    ) -> Contract {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();

        let web3 = web3::api::Web3::new(http);
        //let contract = EthContract::new(web3.eth(), account.contract_address, );
        let contract = EthContract::from_json(web3.eth(), account.contract_address, include_bytes!("./abi.json")).unwrap();

        let contract = Contract{
            contract,
            performer_sender,
            server_control_sender,
            my_account: account, 
            contract_state: ContractState::genesis(),
            contract_handle,
            web3,
            mempool,
        };

        return contract;
    }

    pub fn start(mut self) {
        let _ = std::thread::spawn(move || {
            loop {
                match self.contract_handle.recv() {
                    //let _ = std::thread::spawn(move || {
                        Ok(handle) => {
                            match handle.message {
                                Message::SendBlock(_) => {
                                    println!("send block");
                                    self.send_block(handle);
                                },
                                Message::AddMainNode(address) => {
                                    println!("add man node");
                                    self.add_main_node(address);
                                },
                                Message::CountMainNodes => {
                                    self.count_main_nodes(handle);
                                },
                                Message::GetCurrState => {
                                    self.get_curr_state(handle); 
                                },
                                Message::GetMainNodes => {
                                    self.get_main_nodes(handle);
                                },
                                Message::GetTxReceipt(tx_hash) => {
                                    self.get_tx_receipt(handle, tx_hash);
                                },
                                Message::GetAll => {
                                    self.test_get_all();
                                },
                                //...
                                _ => {
                                    println!("Unrecognized Message");
                                }
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

    fn _get_curr_state(&self) -> ContractState {
        let hash: web3::types::H256 = self.contract
            .query("getCurrentHash", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        let blk_id: web3::types::U256 = self.contract
            .query("getBlockID", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        ContractState {
            curr_hash: hash.into(),
            block_id: blk_id.as_usize(),
        }
    }

    fn get_curr_state(&self, handle: Handle) {
        let curr_state = self._get_curr_state();
        let response = Response::GetCurrState(curr_state);
        let answer = Answer::Success(response);
        handle.answer_channel.unwrap().send(answer);
    }

    pub fn sync(&self) -> ContractState {
        let hash: web3::types::H256 = self.contract
            .query("getCurrentHash", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        let blk_id: web3::types::U256 = self.contract
            .query("getBlockID", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        ContractState {
            curr_hash: hash.into(),
            block_id: blk_id.as_usize()
        }
    }

    pub fn get_prev_blocks(&self, start: usize, end: usize) -> Vec<MainNodeBlock> {
        unimplemented!()
    }

    fn get_main_node(&self, index: usize) -> Address {
        let address: Address = self.contract
            .query("getMainNode", (web3::types::U256::from(index), ), None, EthOption::default(), None)
            .wait()
            .unwrap();
        return address;
    }

    pub fn get_main_nodes(&self, handle: Handle) {
        let n = self._count_main_nodes().unwrap();
        let mut nodes = Vec::new();
        for i in {0..n} {
            let address = self.get_main_node(i);
            nodes.push(address);
        }
        println!("main nodes list = {:?}", nodes);
        let response = Response::MainNodesList(nodes);
        let answer = Answer::Success(response);
        match handle.answer_channel.as_ref() {
            Some(ch) => (*ch).send(answer).unwrap(),
            None => panic!("contract get main nodes list without answer channel"),
        }

    }

    fn _count_main_nodes(&self) -> Result<usize> {
        let cnt: web3::types::U256 = self.contract
            .query("mainNodesCount", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        Ok(cnt.as_usize())
    }

    pub fn count_main_nodes(&self, handle: Handle){

        let num_main_node = self._count_main_nodes().unwrap();
        println!("count_main_nodes = {:?}", num_main_node);
        let response = Response::CountMainNode(num_main_node);
        let answer = Answer::Success(response);
        match handle.answer_channel.as_ref() {
            Some(ch) => (*ch).send(answer).unwrap(),
            None => panic!("contract count main node without answer channel"),
        }
    }

    pub fn add_main_node(&self, address: Address) {
        const ETH_CHAIN_ID: u32 = 3;
        let nonce = self.web3.eth()
            .transaction_count(self.my_account.address, None)
            .wait()
            .unwrap();
        let blk_id: web3::types::U256 = self.contract
            .query("getBlockID", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        let command = format!("ethabi encode function --lenient ./abi.json addMainNode -p {}", address);
        let output = Command::new("sh").arg("-c")
            .arg(command)
            .output().unwrap();

        let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
        let tx = RawTransaction {
            nonce: self.convert_u256(nonce),
            to: Some(self.convert_account(self.my_account.contract_address)),
            value: ethereum_types::U256::from(0),
            gas_price: ethereum_types::U256::from(1000000000),
            gas: ethereum_types::U256::from(100000),
            data: function_abi
        };
        let pkey = self.my_account.private_key.replace("0x", "");
        let key = self.get_private_key(pkey.as_str());

        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash: web3::types::H256 = self.web3.eth()
            .send_raw_transaction(Bytes::from(signed_tx))
            .wait()
            .unwrap();
        println!("tx_hash = {:?}", tx_hash);


    }

    pub fn convert_u256(&self, value: web3::types::U256) -> ethereum_types::U256 {
        let web3::types::U256(ref arr) = value;
        let mut ret = [0; 4];
        ret[0] = arr[0];
        ret[1] = arr[1];
        ethereum_types::U256(ret)
    }

    pub fn convert_account(&self, value: Address) -> ethereum_types::H160 {
        let ret = ethereum_types::H160::from(value.0);
        ret
    }

    fn to_array(&self, bytes: &[u8]) -> [u8; 32] {
        let mut array = [0; 32];
        let bytes = &bytes[..array.len()];
        array.copy_from_slice(bytes);
        array
    }

    fn hash_message(&self, message: &[u8], result: &mut [u8]) {
        let s = String::from("\x19Ethereum Signed Message:\n32");
        let prefix = s.as_bytes();
        let prefixed_message = [prefix, message].concat();
        let mut hasher = Sha3::keccak256();
        hasher.input(&prefixed_message);
        hasher.result(result);
    }

    fn sign_block(&self, block: &str, private_key: &[u8]) -> String {
        let mut hasher = Sha3::keccak256();
        hasher.input_str(block);
        let mut message = [0; 32];
        hasher.result(&mut message);
        let mut result = [0u8; 32];
        self.hash_message(&message, &mut result);

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(private_key).unwrap();
        let msg = secp256k1::Message::from_slice(&result).unwrap();
        let sig = secp.sign_recoverable(&msg, &sk);
        let (v, data) = sig.serialize_compact();
        let mut r: [u8; 32] = [0; 32];
        let mut s: [u8; 32] = [0; 32];
        r.copy_from_slice(&data[0..32]);
        s.copy_from_slice(&data[32..64]);
        return format!("{}{}{}", hex::encode(r), hex::encode(s), hex::encode([v.to_i32() as u8 + 27]));
    }

    pub fn get_private_key(&self, key: &str) -> ethereum_types::H256 {
        let private_key = hex::decode(key).unwrap();

        return ethereum_types::H256(self.to_array(private_key.as_slice()));
    }

    pub fn send_block(&self, handle: Handle)  {
        let block = match handle.clone().message {
            Message::SendBlock(block) => block,
            _ => panic!("contract send block not receive block"), 
        };
        let block_vec: Vec<u8> = block.clone().ser();
        let block_ref: &[u8] = block_vec.as_ref();
        let str_block= hex::encode(block_ref);
        const ETH_CHAIN_ID: u32 = 3;
        let nonce = self.web3.eth()
            .transaction_count(self.my_account.address, None)
            .wait()
            .unwrap();
        let blk_id: web3::types::U256 = self.contract
            .query("getBlockID", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
       // let block = "hello";
        let pkey = self.my_account.private_key.replace("0x", "");
        let private_key = hex::decode(pkey.as_str()).unwrap();
        let signature = self.sign_block(str_block.as_str(), &private_key);

        let command = format!("ethabi encode function --lenient ./abi.json sendBlock -p {} -p {} -p {}", str_block, signature, blk_id+1);
        let output = Command::new("sh").arg("-c")
            .arg(command)
            .output().unwrap();

        let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
        let tx = RawTransaction {
            nonce: self.convert_u256(nonce),
            to: Some(self.convert_account(self.my_account.contract_address)),
            value: ethereum_types::U256::from(0),
            gas_price: ethereum_types::U256::from(1000000000),
            gas: ethereum_types::U256::from(100000),
            data: function_abi
        };
        let key = self.get_private_key(pkey.as_str());

        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash: web3::types::H256 = self.web3.eth()
            .send_raw_transaction(Bytes::from(signed_tx))
            .wait()
            .unwrap();
        println!("tx_hash = {:?}", tx_hash);
        if self.get_tx_receipt(handle, tx_hash) {
            // broadcast to peers
            let curr_state = self._get_curr_state();
            self.send_p2p(curr_state, block);
        } else {
            // return transaction back to mempool
            let mut mempool = self.mempool.lock().expect("api change mempool size");
            mempool.return_block(block);
            drop(mempool);           
        }
    }

    fn send_p2p(&self, curr_state: ContractState, block: Block) {
        let main_block = MainNodeBlock {
            contract_state: curr_state,
            block: block,
        };

        let server_message = ServerMessage::SyncBlock(main_block);
        let p2p_message = ServerSignal::ServerBroadcast(server_message);
        self.server_control_sender.send(p2p_message); 
    }

    pub fn get_tx_receipt(&self, handle: Handle, tx_hash: web3::types::H256) -> bool {
        let now = time::Instant::now();
        let mut receipt = self.web3.eth()
            .transaction_receipt(tx_hash)
            .wait()
            .unwrap();
        while receipt.is_none() {
            receipt = self.web3.eth()
                .transaction_receipt(tx_hash)
                .wait()
                .unwrap();
            if now.elapsed().as_secs() > 60 {
                println!("Transaction TimeOut");
                return false;
            }
        }
        return true;
    
    }

    // pull function to get updated, return number of state change, 0 for no change 
    pub fn sync_etherchain(&self) -> Result<usize> {
         unimplemented!()   
    }

    pub fn test_get_all(&self) {
        let res = self.get_all([0u8; 32], 0, 9999999);
        println!("{:#?}", res);

    }

    pub fn get_all(&self, init_hash: [u8;32], start: usize, end: usize) -> Vec<ContractState> {
        let mut curr_hash = init_hash;
        let func_sig = "ae8d0145";
        let mut block_list: Vec<String> = Vec::new();
        let mut state_list: Vec<ContractState> = Vec::new();
        let request_url = format!("https://api-ropsten.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}",
                                  address = "0xE5C10A1E39fA1fAF25E4fD5ce2C4e2ec5A7aB926",
                                  start = start,
                                  end = end,
                                  apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX");

        let response = requests::get(request_url).unwrap();
        let data = response.json().unwrap();
        for i in {0..data["result"].len()} {
            if data["result"][i]["input"].as_str().unwrap().len() < 10 {continue;}
            let sig = &data["result"][i]["input"].as_str().unwrap()[2..10];
            let isError = data["result"][i]["isError"].as_str().unwrap().parse::<i32>().unwrap();
            if sig == func_sig && isError == 0 {
                let input = &data["result"][i]["input"].as_str().unwrap()[10..];
                let command = format!("ethabi decode params -t string -t bytes -t uint256 {}", input);
                let output = Command::new("sh").arg("-c")
                    .arg(command)
                    .output().unwrap();
                let params = std::str::from_utf8(&output.stdout).unwrap().split("\n");
                let params: Vec<&str> = params.collect();
                let block = params[0].replace("string ", "");
                let block_id = params[2].replace("uint256 ", "");
                let block_id = usize::from_str_radix(&block_id, 16).unwrap();

                block_list.push(block.clone());
                let mut hasher = Sha256::new();
                hasher.input_str(&block);
                let mut block_hash = [0u8;32];
                hasher.result(&mut block_hash);
                let concat_str = [curr_hash, block_hash].concat();
                let mut hasher = Sha256::new();
                hasher.input(&concat_str);
                hasher.result(&mut curr_hash);
                state_list.push(ContractState{
                    curr_hash: H256(curr_hash),
                    block_id,
                })
            }
        }
        return state_list;
    }


}
