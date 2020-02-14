use std::process::Command;
use web3::types::{Address, U256};

use crypto::sha3::Sha3;
use crypto::digest::Digest;
use secp256k1::{Secp256k1, SecretKey};
use crate::primitive::block::Block;

pub fn _encode_sendBlock(block: String, signature: String, new_blk_id: U256) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi.json sendBlock -p {} -p {} -p {}", block, signature, new_blk_id);
    let output = Command::new("sh").arg("-c")
        .arg(command)
        .output().unwrap();

    let function_abi = hex::decode(std::str::from_utf8(&output.stdout).unwrap().trim()).unwrap();
    return function_abi;
}

pub fn _encode_addMainNode(address: Address) -> Vec<u8> {
    let command = format!("ethabi encode function --lenient ./abi.json addMainNode -p {}", address);
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



pub fn _hash_message(message: &[u8], result: &mut [u8]) {
    let s = String::from("\x19Ethereum Signed Message:\n32");
    let prefix = s.as_bytes();
    let prefixed_message = [prefix, message].concat();
    let mut hasher = Sha3::keccak256();
    hasher.input(&prefixed_message);
    hasher.result(result);
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

pub fn _block_to_str(block: Block) -> String {
    let block_vec: Vec<u8> = block.clone().ser();
    let block_ref: &[u8] = block_vec.as_ref();
    hex::encode(block_ref)
}