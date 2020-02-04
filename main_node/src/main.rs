use web3::contract::{Contract, Options};
use web3::types::{Address, Bytes};
use web3::futures::Future;
use crypto::sha3::Sha3;
use crypto::digest::Digest;
use secp256k1::{Secp256k1, SecretKey, Message};
use serde_json::{Value};
use std::fs::File;
use std::io::Read;
use ethereum_tx_sign::RawTransaction;
//use ethereum_types::{H160,H256,U256};
use web3::types::{H160, H256, U256};
//use ethabi_contract::use_contract;



const ETH_CHAIN_ID: u32 = 3;
fn read_json(path: &str) -> Value {
    let mut file = File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    return serde_json::from_str(&data).unwrap();
}

fn sign_block(message: &[u8], private_key: &[u8]) -> String {
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(private_key).unwrap();
    let msg = Message::from_slice(message).unwrap();
    let sig = secp.sign_recoverable(&msg, &sk);
    let (v, data) = sig.serialize_compact();
    let mut r: [u8; 32] = [0; 32];
    let mut s: [u8; 32] = [0; 32];
    r.copy_from_slice(&data[0..32]);
    s.copy_from_slice(&data[32..64]);
    return format!("0x{}{}{}", hex::encode(r), hex::encode(s), hex::encode([v.to_i32() as u8 + 27]));
}

fn hash_message(message: &[u8], result: &mut [u8]) {
    let s = String::from("\x19Ethereum Signed Message:\n32");
    let prefix = s.as_bytes();
    let prefixed_message = [prefix, message].concat();
    let mut hasher = Sha3::keccak256();
    hasher.input(&prefixed_message);
    hasher.result(result);
}

fn convert_u256(value: web3::types::U256) -> U256 {
    let web3::types::U256(ref arr) = value;
    let mut ret = [0; 4];
    ret[0] = arr[0];
    ret[1] = arr[1];
    U256(ret)
}

fn convert_account(value: Address) -> H160 {
    let ret = H160::from(value.0);
    ret
}

fn get_private_key(key: &str) -> H256 {
    // Remember to change the below
    let private_key = hex::decode(key).unwrap();

    return H256(to_array(private_key.as_slice()));
}

fn to_array(bytes: &[u8]) -> [u8; 32] {
    let mut array = [0; 32];
    let bytes = &bytes[..array.len()];
    array.copy_from_slice(bytes);
    array
}
fn main() {
  //  let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let (_eloop, transport) = web3::transports::Http::new(
        "https://ropsten.infura.io/v3/fd7fd90847cd4ca99ce886d4bffdccf8"
        ).unwrap();
    let web3 = web3::Web3::new(transport);  
    let mut hasher = Sha3::keccak256();
    
    let config: Value = read_json("./mainNode-config_1.json");

    
    let address: Address = config["mainNode"]["address"].as_str().unwrap().parse().unwrap();
    let address2: Address = config["mainNode_to_add"]["address"].as_str().unwrap().parse().unwrap();
    let contract_address: Address = config["contract_address"].as_str().unwrap().parse().unwrap();
    let private_key = hex::decode(config["mainNode"]["private_key"].as_str().unwrap()).unwrap();
//    let private_key = get_private_key(config["mainNode"]["private_key"].as_str().unwrap());

  //  use_contract!(scalechain, "./abi.json");

//    let accounts = web3.eth().accounts().wait().unwrap();
//    println!("{:?}", accounts);
//    let address = accounts[0];
//    let address2 = accounts[1];
//    let private_key = get_private_key("d58ff6daa397d7cc1aa6d156d8adf99895894b39e64793a68dd3f84310912eb3");

    let nonce = web3.eth().transaction_count(address, None).wait().unwrap();
    println!("Number of transactions sent from {:?}: {:?}", address, nonce);
    let contract = Contract::from_json(web3.eth(), contract_address, include_bytes!("./abi.json")).unwrap();

//send raw transactions
/*    let balance_before = web3.eth().balance(address, None).wait().unwrap();
    let accounts = web3.eth().accounts().wait().unwrap();

    let tx = RawTransaction {
        nonce: convert_u256(nonce),
        to: Some(convert_account(address2)),
        value: U256::from(10000002),
        gas_price: U256::from(1000000000),
        gas: U256::from(210000),
        data: Vec::new()
    };

    let signed_tx = tx.sign(&private_key, &ETH_CHAIN_ID);
    let tx_hash = web3.eth().send_raw_transaction(Bytes::from(signed_tx)).wait().unwrap();


    let balance_after = web3.eth().balance(address, None).wait().unwrap();
    let receipt = web3.eth().transaction_receipt(tx_hash).wait().unwrap();

    println!("TX Hash: {:?}", tx_hash);
    println!("Balance before: {}", balance_before);
    println!("Balance after: {}", balance_after);
    println!("Receipt: {:?}", receipt);
*/
    
    let cnt_query = contract.query("mainNodesCount", (), None, Options::default(), None);
    let cnt: U256 = cnt_query.wait().unwrap();
    let hash_query = contract.query("getCurrentHash", (), None, Options::default(), None);
    let hash: H256 = hash_query.wait().unwrap();
    let id_query = contract.query("getBlockID", (), None, Options::default(), None);
    let blk_id: U256 = id_query.wait().unwrap();

    println!("mainNodesCount = {}", cnt);
    println!("currentHash = {}", hash);
    println!("blockID = {}", blk_id);
    
    let block = "hello";
    hasher.input_str(block);
    let mut message = [0; 32];
    hasher.result(&mut message);
    let mut result = [0; 32];
    hash_message(&message, &mut result);
    let signature = sign_block(&result, &private_key);
    
    let send = contract.call("sendBlock", (String::from(block), signature, blk_id+1), address, Options::default()).wait().err();
    let id_query = contract.query("getBlockID", (), None, Options::default(), None);
    println!("send result = {:?}", send);
    let blk_id: U256 = id_query.wait().unwrap();
    println!("blockID = {}", blk_id);

/*
    let event_future = web3
        .eth_subscribe()
        .then(|sub| {
            sub.unwrap().for_each(|log| {
                println!("got log: {:?}", log);
                Ok(())
            })
        })
        .map_err(|_| ());

    let call_future = contract.call("sendBlock", (String::from(block), signature, blk_id+1), address2, Options::default()).then(|tx| {
        println!("got tx: {:?}", tx);
        Ok(())
    });
    event_future.join(call_future);
    

*/
}
