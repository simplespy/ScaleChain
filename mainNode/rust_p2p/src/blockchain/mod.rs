use super::primitive::{hash, block};


pub mod blockchain;
pub mod fork;

pub const GENESIS: hash::H256 = hash::H256([0 as u8;32]);


