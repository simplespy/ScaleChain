mod utils;
use web3::types::{Address, Bytes, U256, H256, TransactionReceipt, CallRequest, H160, U64};
use std::fs::File;
use serde::{Serialize, Deserialize};
use web3::contract::Contract as EthContract;
use web3::contract::Options as EthOption;
use web3::futures::Future;
use utils::*;
use ethereum_tx_sign::RawTransaction;
use std::{time};

use crypto::digest::Digest;
use crypto::sha2::Sha256;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    rpc_url: String,
    contract_address: Address,
    address: Address,
    private_key: String,
    ip_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BLSKeyStr{
    pub sk: String,
    pub pkx1: String,
    pub pkx2: String,
    pub pky1: String,
    pub pky2: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BLSKey{
    pub sk: U256,
    pub pkx1: U256,
    pub pkx2: U256,
    pub pky1: U256,
    pub pky2: U256,
}

impl BLSKey {
    pub fn new(key: BLSKeyStr) -> Self {
        BLSKey {
            sk :U256::from_dec_str(key.sk.as_ref()).unwrap(),
            pkx1 :U256::from_dec_str(key.pkx1.as_ref()).unwrap(),
            pkx2 :U256::from_dec_str(key.pkx2.as_ref()).unwrap(),
            pky1 :U256::from_dec_str(key.pky1.as_ref()).unwrap(),
            pky2 :U256::from_dec_str(key.pky2.as_ref()).unwrap(),
        }
    }
}

pub struct Contract {
    contract: EthContract<web3::transports::Http>,
    my_account: Account,
    web3: web3::api::Web3<web3::transports::Http>,

}
const ETH_CHAIN_ID: u32 = 3;

impl Contract {
    pub fn new(
        account: Account,
    ) -> Contract {
        let (eloop, http) = web3::transports::Http::new(&account.rpc_url).unwrap();
        eloop.into_remote();
        let web3 = web3::api::Web3::new(http);
        let contract = EthContract::from_json(web3.eth(), account.contract_address, include_bytes!("./abi.json")).unwrap();

        let contract = Contract {
            contract,
            my_account: account,
            web3
        };

        return contract;
    }
    pub fn add_scale_node(&self, address: Address, ip_addr: String, x1: U256, x2: U256, y1: U256, y2: U256) {
        let nonce = self._transaction_count();
        let function_abi = _encode_addScaleNode(address, ip_addr, x1, x2, y1, y2);
        let gas = self._estimate_gas(function_abi.clone());

        println!("{:?}", gas);

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
        println!("{:?}", tx_hash);
        if self.get_tx_receipt(tx_hash) {
            println!("{:?}", tx_hash);
        }
    }

    pub fn add_side_node(&self, sid: U256, address: Address, ip_addr: String) {
        let nonce = self._transaction_count();
        let function_abi = _encode_addSideNode(sid, address, ip_addr);
        let gas = self._estimate_gas(function_abi.clone());

        println!("{:?}", gas);

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
        println!("{:?}", tx_hash);
        if self.get_tx_receipt(tx_hash) {
            println!("{:?}", tx_hash);
        }
    }

    pub fn delete_side_node(&self, sid: U256, tid: U256) {
        let nonce = self._transaction_count();
        let function_abi = _encode_deleteSideNode(sid, tid);
        //let gas = self._estimate_gas(function_abi.clone());

        //println!("{:?}", gas);

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
        println!("{:?}", tx_hash);
        if self.get_tx_receipt(tx_hash) {
            println!("{:?}", tx_hash);
        }
    }




    pub fn _transaction_count(&self) -> U256 {
        self.web3.eth()
            .transaction_count(self.my_account.address, None)
            .wait()
            .unwrap()
    }
    fn _send_transaction(&self, signed_tx: Vec<u8>) -> web3::types::H256 {
        self.web3.eth()
            .send_raw_transaction(Bytes::from(signed_tx))
            .wait()
            .unwrap()
    }
    pub fn _count_scale_nodes(&self) -> usize {
        let cnt: U256 = self.contract
            .query("scaleNodesCount", (), None, EthOption::default(), None)
            .wait()
            .unwrap();
        cnt.as_usize()
    }
    pub fn _count_main_nodes(&self) -> usize {
        let cnt: U256 = self.contract
            .query("mainNodesCount", (), None, EthOption::default(), None)
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
    fn _get_blk_id(&self, sid: usize) -> U256 {
        self.contract
            .query("getBlockID", (web3::types::U256::from(sid),), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }
    fn _get_side_node_id(&self, sid: usize, addr: Address) -> U256 {
        self.contract
            .query("getSideNodeID", (web3::types::U256::from(sid), addr), None, EthOption::default(), None)
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

    pub fn submit_vote(&self, str_block: String, sid: U256, bid: U256, sigx: U256, sigy: U256, bitset: U256) {
        let nonce = self._transaction_count();
        let private_key = _get_key_as_vec(self.my_account.private_key.clone());
        let function_abi = _encode_submitVote(str_block, sid, bid, sigx, sigy, bitset);
        //let gas = self._estimate_gas(function_abi.clone());
        //println!("{:?}", gas);
        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000)*200,
           //gas: _convert_u256(gas),
            gas: ethereum_types::U256::from(750000),
            data: function_abi
        };


        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);
        println!("{:?}", tx_hash);
        if self.get_tx_receipt(tx_hash) {
            println!("success");
        }
    }

    pub fn send_block(&self, str_block: String)  {
        let nonce = self._transaction_count();
        let blk_id = self._get_blk_id(0);
        let private_key = _get_key_as_vec(self.my_account.private_key.clone());
        let signature = _sign_block(str_block.as_str(), &private_key);
        let function_abi = _encode_sendBlock(str_block, signature, blk_id + 1);

        let gas = self._estimate_gas(function_abi.clone());
        println!("{:?}", gas);

     /*   let tx = RawTransaction {
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
        }*/
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

    pub fn sort(&self, s: usize) {
        let nonce = self._transaction_count();
        let function_abi = _encode_sort(U256::from(s));
        let gas = self._estimate_gas(function_abi.clone());

         println!("{:?}", gas);

        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000)*150,
            gas: _convert_u256(gas),
            data: function_abi
        };
        let now = time::Instant::now();
        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);
        if self.get_tx_receipt(tx_hash) {
            println!("{:?},{:?}", tx_hash, now.elapsed().as_secs());
        }
    }

    pub fn get_tx_receipt(&self, tx_hash: web3::types::H256) -> bool {
        let now = time::Instant::now();
        let mut receipt = self._transaction_receipt(tx_hash.clone());
        while receipt.is_none() {
            receipt = self._transaction_receipt(tx_hash.clone());
            if now.elapsed().as_secs() > 60 {
                return false;
            }
        }
        if receipt.unwrap().status.unwrap() == U64::from(1) {
            return true;
        }
        return false;
    }

    fn _transaction_receipt(&self, tx_hash: web3::types::H256) -> Option<TransactionReceipt> {
        self.web3.eth()
            .transaction_receipt(tx_hash)
            .wait()
            .unwrap()
    }
    fn _get_curr_hash(&self, sid: usize) -> web3::types::H256 {
        self.contract
            .query("getCurrentHash", (web3::types::U256::from(sid),), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }
    fn _get_pub_key(&self, address: Address) -> (web3::types::U256, web3::types::U256, web3::types::U256, web3::types::U256) {
        self.contract
            .query("getScalePubKey", (address,), None, EthOption::default(), None)
            .wait()
            .unwrap()
    }
    fn reset_chain(&self, sid: usize)  {
        let nonce = self._transaction_count();

        let function_abi = _encode_resetSideChain(U256::from(sid));
        let gas = self._estimate_gas(function_abi.clone());

        println!("{:?}", nonce);

        let tx = RawTransaction {
            nonce: _convert_u256(nonce),
            to: Some(ethereum_types::H160::from(self.my_account.contract_address.0)),
            value: ethereum_types::U256::zero(),
            gas_price: ethereum_types::U256::from(1000000000)*300,
            gas: _convert_u256(gas),
            data: function_abi
        };
        let now = time::Instant::now();
        let key = _get_key_as_H256(self.my_account.private_key.clone());
        let signed_tx = tx.sign(&key, &ETH_CHAIN_ID);
        let tx_hash = self._send_transaction(signed_tx);
        if self.get_tx_receipt(tx_hash) {
            println!("{:?},{:?}", tx_hash, now.elapsed().as_secs());
        }
    }

}

fn print_id(contract: &Contract, addr: Address) {
    let id = contract._get_scale_id(addr);
    println!("scale_nodes[{:?}] = {:?}", id, addr);
}

pub fn _generate_random_header() -> String {
    let mut header = String::new();
    for i in {0..16} {
        header = format!("{}{}", header, hex::encode(web3::types::H256::random().as_bytes()))
    }
    header

}
pub fn _count_sig(x: usize) -> usize {
    let mut cnt = 0;
    let mut t = x;
    while t > 0 {
        if t.clone() % 2 == 1 {
            cnt += 1;
        }
        t /= 2;
    }
    cnt
}
fn main() {
/*    let file = File::open("account_test").unwrap();
    let account: Account = serde_json::from_reader(file).expect("deser account");
    let contract = Contract::new(account.clone());
    for i in {0..100} {
        contract.sort(3);
    }
*/


       let file_admin = File::open("account_admin").unwrap();
       let file_1 = File::open("account_1").unwrap();
       let file_2 = File::open("account_2").unwrap();

       let key_admin = File::open("keyfile/key_admin").unwrap();
       let key_1 = File::open("keyfile/key_1").unwrap();
       let key_2 = File::open("keyfile/key_2").unwrap();


       let account_admin: Account = serde_json::from_reader(file_admin).expect("deser account");
       let account_1: Account = serde_json::from_reader(file_1).expect("deser account");
       let account_2: Account = serde_json::from_reader(file_2).expect("deser account");

       let bls_key_str_admin: BLSKeyStr = serde_json::from_reader(key_admin).expect("deser key file");
       let bls_key_str_1: BLSKeyStr = serde_json::from_reader(key_1).expect("deser key file");
       let bls_key_str_2: BLSKeyStr = serde_json::from_reader(key_2).expect("deser key file");

       let bls_key_admin = BLSKey::new(bls_key_str_admin);
       let bls_key_1 = BLSKey::new(bls_key_str_1);
       let bls_key_2 = BLSKey::new(bls_key_str_2);


        let contract = Contract::new(account_1.clone());


       // contract.add_side_node(U256::from(0), account_admin.address, account_admin.ip_address);
  //  contract.add_side_node(U256::from(0), account_1.address, account_1.ip_address);
   // contract.add_side_node(U256::from(0), account_2.address, account_2.ip_address);
        contract.delete_side_node(U256::from(0), U256::from(0));
        let bid = contract._get_side_node_id(0, account_2.address);
        println!("side node id = {}",bid);

/*
    for i in {3..7} {
        let file = File::open(format!("account_{}", i)).unwrap();
        let key = File::open(format!("keyfile/node{}", i)).unwrap();
        let account: Account = serde_json::from_reader(file).expect("deser account");
        let bls_key_str: BLSKeyStr = serde_json::from_reader(key).expect("deser key file");
        let bls_key = BLSKey::new(bls_key_str);
        contract.add_scale_node(account.address, account.ip_address, bls_key.pkx1, bls_key.pkx2, bls_key.pky1, bls_key.pky2);
        let cnt = contract._count_scale_nodes();
        println!("nodes num = {}",cnt);

    }

    let mut xx = 4;
    for i in {3..7} {
        let file = File::open(format!("account_{}", i)).unwrap();
        let key = File::open(format!("keyfile/node{}", i)).unwrap();
        let account: Account = serde_json::from_reader(file).expect("deser account");
        let bls_key_str: BLSKeyStr = serde_json::from_reader(key).expect("deser key file");
        let bls_key = BLSKey::new(bls_key_str);
        let contract = Contract::new(account.clone());
        let (sigx1, sigy1) = _sign_bls("deadbeef".to_string(), "key_1".to_string());
        let (sigx2, sigy2) = _sign_bls("deadbeef".to_string(), format!("node{}", i));
        let (sigx, sigy) = _aggregate_sig(sigx1, sigy1, sigx2, sigy2);
        xx *= 2;
        let bid = contract._get_blk_id(0);
        contract.submit_vote("deadbeef".to_string(), U256::from(0), U256::from(bid + 1), U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(2+xx));

        let bid = contract._get_blk_id(0);
        println!("block id = {}",bid);
    }
*/
    /*  for i in {0..3} {
         let addr = contract._get_scale_node(i);
         let pub_key = contract._get_scale_pub_key(addr);
         println!("{:?},{:?}", addr, pub_key);
     }*/


     //  contract.send_block(_generate_random_header());

   //    contract.add_scale_node(account_1.address, account_1.ip_address, bls_key_1.pkx1, bls_key_1.pkx2, bls_key_1.pky1, bls_key_1.pky2);
    //   contract.add_scale_node(account_2.address, account_2.ip_address, bls_key_2.pkx1, bls_key_2.pkx2, bls_key_2.pky1, bls_key_2.pky2);

    //   let pub_key = contract._get_scale_pub_key(account_2.address);
    //   println!("pub_key of account2 is {:?}", pub_key);

    /*
           let bid = contract._get_blk_id(0);
           println!("block id = {}",bid);

        //   contract.send_block("deadbeef".to_string());

           let (sigx1, sigy1) = _sign_bls("deadbeef".to_string(), "key_1".to_string());
           let (sigx2, sigy2) = _sign_bls("deadbeef".to_string(), "key_2".to_string());
           let (sigx, sigy) = _aggregate_sig(sigx1, sigy1, sigx2, sigy2);

        //   println!("signature = {:?}, {:?}", sigx, sigy);
           contract.submit_vote("deadbeef".to_string(), U256::from(0), U256::from(bid + 1), U256::from_dec_str(sigx.as_ref()).unwrap(), U256::from_dec_str(sigy.as_ref()).unwrap(), U256::from(6));
        //    contract.reset_chain(0);
           let bid = contract._get_blk_id(0);

           println!("{:?}", contract._get_curr_hash(0));
           println!("block id = {}", bid);

    */
   // println!("{:?}", contract._get_pub_key(account_2.address));

  //  println!("{:?}",_count_sig(26));
   /* let mut hasher = Sha256::new();
    hasher.input(&hex::decode("deadbeef").unwrap());
    let mut block_hash = [0u8;32];
    let mut curr_hash = [0u8;32];
    hasher.result(&mut block_hash);
    let concat_str = [curr_hash, block_hash].concat();
    let mut hasher = Sha256::new();
    hasher.input(&concat_str);
    hasher.result(&mut curr_hash);
    println!("{:?}", hex::encode(curr_hash));*/





}
