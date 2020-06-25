use std::process::Command;
use web3::types::{Address, U256};

use crypto::sha3::Sha3;
use crypto::digest::Digest;
use secp256k1::{Secp256k1, SecretKey};
use bincode::{deserialize};
use std::fs::File;
use std::io::Write;


pub fn _encode_addScaleNode(address: Address, ip_addr: String, x1: U256, x2: U256, y1: U256, y2: U256) -> Vec<u8> {
    let addr = hex::encode(address.as_bytes());
    //let addr = addr.replace("0x", "");
    let command = format!("ethabi encode function --lenient ./abi3.json addScaleNode -p {} -p {} -p {} -p {} -p {} -p {}", addr, ip_addr, x1, x2, y1, y2);
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();
    println!("{:?}", output);

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}

pub fn _encode_resetSideChain(sid: U256) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi.json resetSideChain -p {}", sid);
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();
    println!("{:?}", output);

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}

pub fn _encode_sendBlock(block: String, signature: String, new_blk_id: U256) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi3.json proposeBlock -p {} -p {} -p {}", block, signature, new_blk_id);
    println!("command {}", command.clone());
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}
pub fn _encode_sort(s: U256) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi.json sort -p {}", s);
   // println!("command {}", command.clone());
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}
pub fn _encode_submitVote(block: String, sid: U256, bid: U256, sigx: U256, sigy: U256, bitset: U256) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi.json submitVote -p {:?} -p {} -p {} -p {} -p {} -p {}", block, sid, bid, sigx, sigy, bitset);
    println!("command {}", command.clone());
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}


pub fn _decode_sendBlock(input: &str) -> (String, usize) {
    let command = format!("ethabi decode params -t string -t bytes -t uint256 {}", input);
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();
    let params = std::str::from_utf8(&output.stdout).unwrap().split("\n");
    let params: Vec<&str> = params.collect();
    //println!("ethabu output {:?}", params);
    let block = params[0].replace("string ", "");
    let block_id = params[2].replace("uint256 ", "");
    let block_id = usize::from_str_radix(&block_id, 16).unwrap();
    (block, block_id)
}

pub fn _sign_bls(msg: String, key_file: String) -> (String, String) {
    let command = format!("./sign -msg={} -key=keyfile/{}", msg, key_file);
    println!("command {}", command.clone());
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    //let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    let sig = std::str::from_utf8(&output.stdout).unwrap().split("\n");
    let sig: Vec<&str> = sig.collect();
    return (sig[0].to_string(), sig[1].to_string());

}


pub fn _aggregate_sig(x1: String, y1: String, x2: String, y2: String)-> (String, String) {
    let command = format!("./aggregate -x1={} -y1={} -x2={} -y2={}", x1, y1, x2, y2);
    println!("command {}", command.clone());
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    //let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    let sig = std::str::from_utf8(&output.stdout).unwrap().split("\n");
    let sig: Vec<&str> = sig.collect();
    return (sig[0].to_string(), sig[1].to_string());
}

pub fn _sign_block(block: &str, private_key: &[u8]) -> String {
    let mut hasher = Sha3::keccak256();
    hasher.input_str(block);
    let mut message = [0; 32];
    hasher.result(&mut message);
    let mut result = [0u8; 32];
    _hash_message(&message, &mut result);

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



pub fn _hash_message(message: &[u8], result: &mut [u8]) {
    let s = String::from("\x19Ethereum Signed Message:\n32");
    let prefix = s.as_bytes();
    let prefixed_message = [prefix, message].concat();
    let mut hasher = Sha3::keccak256();
    hasher.input(&prefixed_message);
    hasher.result(result);
}



pub fn _convert_u256(value: U256) -> ethereum_types::U256 {
    let U256(ref arr) = value;
    let mut ret = [0; 4];
    ret[0] = arr[0];
    ret[1] = arr[1];
    ethereum_types::U256(ret)
}

pub fn _get_key_as_H256(key: String) -> ethereum_types::H256 {
    let private_key = _get_key_as_vec(key);
    ethereum_types::H256(_to_array(private_key.as_slice()))
}

pub fn _get_key_as_vec(key: String) -> Vec<u8> {
    let key = key.replace("0x", "");
    hex::decode(&key).unwrap()
}

pub fn _to_array(bytes: &[u8]) -> [u8; 32] {
    let mut array = [0; 32];
    let bytes = &bytes[..array.len()];
    array.copy_from_slice(bytes);
    array
}


