use super::primitive::block::{EthBlkTransaction, ContractState, Block};
use super::primitive::hash::{H256};
use super::network::message::{ServerSignal, TaskRequest};
use super::network::message::Message as ServerMessage;
use super::mempool::mempool::{Mempool};
use super::blockchain::blockchain::{BlockChain};
use super::db::blockDb::{BlockDb};
use super::interface::{Handle, Message, Response, Answer};
use super::utils::*;

use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::types::{Address, Bytes, U256, TransactionReceipt, CallRequest, H160};
use web3::futures::Future;

use crypto::digest::Digest;
use crypto::sha2::Sha256;

use std::sync::{Arc, Mutex};
use std::{time};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufReader, BufRead, Error};

use crossbeam::channel::{Sender, Receiver};
use serde::{Serialize, Deserialize};
use ethereum_tx_sign::RawTransaction;

use mio_extras::channel::Sender as MioSender;

use requests::{ToJson};
use log::{info, warn, error};

const ETH_CHAIN_ID: u32 = 3;


pub struct Contract {
    contract: EthContract<web3::transports::Http>,
    my_account: Account,
    key: BLSKey,
    contract_state: ContractState,
    contract_handle: Receiver<Handle>,
    mempool: Arc<Mutex<Mempool>>,
    performer_sender: Sender<TaskRequest>,
    server_control_sender: MioSender<ServerSignal>,
    web3: web3::api::Web3<web3::transports::Http>,
    chain: Arc<Mutex<BlockChain>>,
    block_db: Arc<Mutex<BlockDb>>,
    ip_addr: String
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
        key: BLSKey,
        performer_sender: Sender<TaskRequest>,
        server_control_sender: MioSender<ServerSignal>,
        contract_handle: Receiver<Handle>, 
        mempool: Arc<Mutex<Mempool>>,
        chain: Arc<Mutex<BlockChain>>,
        block_db: Arc<Mutex<BlockDb>>,
        ip_addr: String
    ) -> Contract {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();

        let web3 = web3::api::Web3::new(http);
        //let contract = EthContract::new(web3.eth(), account.contract_address, );
        let contract = EthContract::from_json(web3.eth(), account.contract_address, include_bytes!("./abi.json")).unwrap();

        let contract = Contract{
            contract,
            key,
            performer_sender,
            server_control_sender,
            my_account: account, 
            contract_state: ContractState::genesis(),
            contract_handle,
            web3,
            mempool,
            chain,
            block_db,
            ip_addr
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
                                Message::SendBlock(block) => {
                                    self.send_block(block);
                                },
                                Message::SubmitVote(header, sigx, sigy, bitset) => {
                                    let header = _generate_random_header();
                                    let (sigx, sigy) = _sign_bls(header.clone(), "node1".to_string());
                                    let (sigx2, sigy2) = _sign_bls(header.clone(), "node2".to_string());
                                    let (sigx3, sigy3) = _sign_bls(header.clone(), "node3".to_string());
                                    let (sigx, sigy) = _aggregate_sig(sigx, sigy, sigx2, sigy2);
                                    let (sigx, sigy) = _aggregate_sig(sigx, sigy, sigx3, sigy3);
                                    self.submit_vote(header, U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(26))
                                },
                                Message::AddScaleNode(id, ip) => {
                                    let file = File::open(format!("accounts/account{}", id)).unwrap();
                                    let key_file = File::open(format!("keyfile/node{}", id)).unwrap();
                                    let account: Account = serde_json::from_reader(file).expect("deser account");
                                    let key_str: BLSKeyStr = serde_json::from_reader(key_file).expect("deser key file");
                                    let key = BLSKey::new(key_str);
                                    self.add_scale_node(account.address, ip, key.pkx1, key.pkx2, key.pky1, key.pky2);
                                },
                                Message::CountScaleNodes => {
                                    self.count_scale_nodes(handle);
                                },
                                Message::GetCurrState => {
                                    self.get_curr_state(handle); 
                                },
                                Message::GetScaleNodes => {
                                    self.get_scale_nodes(handle);
                                },
                                Message::GetTxReceipt(tx_hash) => {
                                    self.get_tx_receipt(tx_hash);
                                },
                                Message::GetAll((_init_hash, start, end)) => {
                                    self.get_all(handle, start, end);
                                },
                                Message::SyncChain => {
                                    self.sync_etherchain(handle);
                                },
                                Message::EstimateGas(block) => {
                                    self.estimate_gas(block);
                                }
                                //...
                                _ => {
                                    warn!("Unrecognized Message");
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

    pub fn get_curr_state(&self, handle: Handle) {
        let curr_state = self._get_curr_state();
        let response = Response::GetCurrState(curr_state);
        let answer = Answer::Success(response);
        handle.answer_channel.unwrap().send(answer);
    }

    pub fn get_prev_blocks(&self, start: usize, end: usize) -> Vec<EthBlkTransaction> {
        unimplemented!()
    }

    pub fn get_scale_nodes(&self, handle: Handle) {
        let n = self._count_scale_nodes();
        let mut nodes = Vec::new();
        for i in {0..n} {
            let address = self._get_scale_node(i);
            nodes.push(address);
        }
        info!("scale nodes list = {:?}", nodes);
        let response = Response::ScaleNodesList(nodes);
        let answer = Answer::Success(response);
        match handle.answer_channel.as_ref() {
            Some(ch) => (*ch).send(answer).unwrap(),
            None => panic!("contract get scale nodes list without answer channel"),
        }

    }

    pub fn count_scale_nodes(&self, handle: Handle){
        let num_scale_node = self._count_scale_nodes();
        info!("count_scale_nodes = {:?}", num_scale_node);
        let response = Response::CountScaleNode(num_scale_node);
        let answer = Answer::Success(response);
        match handle.answer_channel.as_ref() {
            Some(ch) => (*ch).send(answer).unwrap(),
            None => panic!("contract count scale node without answer channel"),
        }
    }

    pub fn add_scale_node(&self, address: Address, ip_addr: String, x1: U256, x2: U256, y1: U256, y2: U256) {
        let nonce = self._transaction_count();
        let function_abi = _encode_addScaleNode(address, ip_addr, x1, x2, y1, y2);
        let gas = self._estimate_gas(function_abi.clone());

        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000),
            gas: _convert_u256(gas),
            data: function_abi
        };

        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);
        if self.get_tx_receipt(tx_hash) {
            println!("tx_hash = {:?}", tx_hash);
        }
    }

    pub fn submit_vote(&self, header: String, sigx: U256, sigy: U256, bitset: U256) {
        let nonce = self._transaction_count();
        let private_key = _get_key_as_vec(self.my_account.private_key.clone());
        let function_abi = _encode_submitVote(header, sigx, sigy, bitset);
        //let gas = self._estimate_gas(function_abi.clone());
        //  println!("{:?}", gas);
        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000),
            gas: ethereum_types::U256::from(750000),
            data: function_abi
        };


        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);
        if self.get_tx_receipt(tx_hash) {
            println!("tx_hash = {:?}", tx_hash);
        }
    }

    pub fn send_block(&self, block: Block)  {
        let str_block= _block_to_str(block.clone());
        let nonce = self._transaction_count();
        let blk_id = self._get_blk_id();
        let private_key = _get_key_as_vec(self.my_account.private_key.clone());
        let signature = _sign_block(str_block.as_str(), &private_key);
        let function_abi = _encode_sendBlock(str_block, signature, blk_id + 1);


        let gas = self._estimate_gas(function_abi.clone());
        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000),
            gas: _convert_u256(gas),
            data: function_abi
        };


        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);

        if self.get_tx_receipt(tx_hash) {
            let curr_state = self._get_curr_state();

            // update local blockchain
            let mut chain = self.chain.lock().unwrap();
            chain.append(&curr_state);
            drop(chain);
            // update db blockchain
            let mut block_db = self.block_db.lock().unwrap();
            block_db.insert(&block);
            drop(block_db);
            // broadcast to peers
            info!("broadcast to peer");
            self.send_p2p(curr_state, block);
        } else {
            // return transaction back to mempool
            warn!("get_tx_receipt fail");
            let mut mempool = self.mempool.lock().expect("api change mempool size");
            mempool.return_block(block);
            drop(mempool);
        }
    }

    pub fn estimate_gas(&self, block: Block) -> U256 {
        let mut file = OpenOptions::new().append(true).open("gas_history.csv").unwrap();
        let str_block= _block_to_str(block.clone());
        let nonce = self._transaction_count();
        let blk_id = self._get_blk_id();
        let private_key = _get_key_as_vec(self.my_account.private_key.clone());
        let signature = _sign_block(str_block.as_str(), &private_key);
        let function_abi = _encode_sendBlock(str_block, signature, blk_id + 1);
        let gas = self._estimate_gas(function_abi.clone());
        file.write_all(format!("{}\n ", gas).as_bytes());
        return gas;
    }

    fn send_p2p(&self, curr_state: ContractState, block: Block) {
        let main_block = EthBlkTransaction {
            contract_state: curr_state,
            block: block,
        };

        let server_message = ServerMessage::SyncBlock(main_block);
        let p2p_message = ServerSignal::ServerBroadcast(server_message);
        self.server_control_sender.send(p2p_message); 
    }

    pub fn get_tx_receipt(&self, tx_hash: web3::types::H256) -> bool {
        let now = time::Instant::now();
        let mut receipt = self._transaction_receipt(tx_hash.clone());
        while receipt.is_none() {
            receipt = self._transaction_receipt(tx_hash.clone());
            if now.elapsed().as_secs() > 60 {
                warn!("Transaction TimeOut");
                return false;
            }
        }
        return true;
    }

    // pull function to get updated, return number of state change, 0 for no change 
    pub fn sync_etherchain(&self, handle: Handle) {
        let mut transactions = self._get_all([0 as u8;32], 0, std::usize::MAX);
        let states: Vec<ContractState> = transactions.
            iter().
            map(|item| {
                item.contract_state.clone()   
            }).collect();
        let chain_len: usize = states.len();

        let blocks: Vec<Block> = transactions.
            into_iter().
            map(|item| {
                item.block   
            }).collect();


        let mut chain = self.chain.lock().unwrap();
        chain.replace(states);
        drop(chain);

        let mut block_db = self.block_db.lock().unwrap();
        block_db.replace(blocks);
        drop(block_db);

        let response = Response::SyncChain(chain_len);
        let answer = Answer::Success(response);
        handle.answer_channel.unwrap().send(answer);
    }

    // [start, end)
    pub fn get_all(&self, handle: Handle, start: usize, end: usize) {
        let transactions = self._get_all([0 as u8; 32], 0, std::usize::MAX);
        let req_transactions: Vec<EthBlkTransaction> = if end != 0 {
            transactions[start..end].to_vec()  
        } else {
            transactions[start..].to_vec()
        };

        let response = Response::GetAll(req_transactions);
        let answer = Answer::Success(response);
        handle.answer_channel.unwrap().send(answer);
    }

    pub fn _get_all(&self, init_hash: [u8;32], start: usize, end: usize) -> (Vec<EthBlkTransaction>) {
        let mut curr_hash = init_hash;
        let func_sig = "ae8d0145";
        let mut block_list: Vec<Block> = Vec::new();
        let mut state_list: Vec<ContractState> = Vec::new();
        let request_url = format!("https://api-ropsten.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}",
                                  address = "0xE5C10A1E39fA1fAF25E4fD5ce2C4e2ec5A7aB926",
                                  start = start,
                                  end = end,
                                  apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX");

        let response = requests::get(request_url).unwrap();
        let data = response.json().unwrap();
        let txs = data["result"].clone();

        let mut transactions: Vec<EthBlkTransaction> = vec![];

        for tx in txs.members() {
            if tx["input"].as_str().unwrap().len() < 10 {continue;}
            let sig = &tx["input"].as_str().unwrap()[2..10];
            let isError = tx["isError"].as_str().unwrap().parse::<i32>().unwrap();
            if sig == func_sig && isError == 0 {
                let input = &tx["input"].as_str().unwrap()[10..];
                let (block_ser, block_id) = _decode_sendBlock(input);
                
                let mut hasher = Sha256::new();
                hasher.input_str(&block_ser);
                let mut block_hash = [0u8;32];
                hasher.result(&mut block_hash);
                let concat_str = [curr_hash, block_hash].concat();
                let mut hasher = Sha256::new();
                hasher.input(&concat_str);
                hasher.result(&mut curr_hash);

                let state = ContractState{
                    curr_hash: H256(curr_hash),
                    block_id,
                };
                let block = match hex::decode(&block_ser) {
                    Ok(bytes) => {
                        match bincode::deserialize(&bytes[..]) {
                            Ok(block) => block,
                            Err(e) => {
                                let mut block = Block::default();
                                block.header.height = block_id;
                                block.update_hash();
                                block
                            },
                        }
                    },
                    Err(e) => {
                        let mut block = Block::default();
                        block.header.height = block_id;
                        block.update_hash();
                        block
                    }
                };
                
                transactions.push(
                    EthBlkTransaction{
                        contract_state: state,
                        block: block,
                    }
                 );
            } 
        }

        return transactions;
    }

    fn _get_blk_id(&self) -> U256 {
        self.contract
            .query("getBlockID", (), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }

    fn _transaction_count(&self) -> U256 {
        self.web3.eth()
            .transaction_count(self.my_account.address, None)
            .wait()
            .unwrap()
    }

    fn _get_curr_hash(&self) -> web3::types::H256 {
        self.contract
            .query("getCurrentHash", (), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }

    fn _get_curr_state(&self) -> ContractState {
        let hash = self._get_curr_hash();
        let blk_id = self._get_blk_id();
        ContractState {
            curr_hash: hash.into(),
            block_id: blk_id.as_usize(),
        }
    }

    pub fn _count_scale_nodes(&self) -> usize {
        let cnt: U256 = self.contract
            .query("scaleNodesCount", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        cnt.as_usize()
    }

    fn _get_scale_node(&self, index: usize) -> Address {
        self.contract
            .query("getScaleNode", (web3::types::U256::from(index), ), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }

    pub fn _get_scale_id(&self, addr: Address) -> U256 {
        self.contract
            .query("getScaleID", (addr), None, EthOption::default(), None)
            .wait()
            .unwrap()

    }

    pub fn _get_scale_pub_key(&self, addr: Address) -> (U256, U256, U256, U256) {
        self.contract
            .query("getScalePubKey", (addr), None, EthOption::default(), None)
            .wait()
            .unwrap()

    }

    fn _send_transaction(&self, signed_tx: Vec<u8>) -> web3::types::H256 {
        self.web3.eth()
            .send_raw_transaction(Bytes::from(signed_tx))
            .wait()
            .unwrap()
    }

    fn _transaction_receipt(&self, tx_hash: web3::types::H256) -> Option<TransactionReceipt> {
        self.web3.eth()
            .transaction_receipt(tx_hash)
            .wait()
            .unwrap()
    }

    fn _estimate_gas(&self, data: Vec<u8>) -> U256 {
        let call_request = CallRequest {
            from: Some(H160::from(self.my_account.address.0)),
            to: H160::from(self.my_account.contract_address.0),
            gas_price: Some(U256::from(1000000000u64)),
            gas: Some(U256::zero()),
            data: Some(Bytes::from(data)),
            value: Some(U256::zero())
        };

        let gas =
            self.web3.eth()
                .estimate_gas(call_request, None)
                .wait()
                .unwrap();
        gas
    }


}
