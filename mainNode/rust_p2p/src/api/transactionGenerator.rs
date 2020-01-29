use super::blockchain::{BlockChain};
use super::hash::{H256};
use super::block::{Transaction, Input, Output};

use std::sync::{Mutex, Arc};

pub struct TransactionGenerator {
    my_addr: Vec<H256>,
    to_addr: Vec<H256>,
}

impl TransactionGenerator {
    pub fn new() -> TransactionGenerator {
        TransactionGenerator {
            my_addr: vec![H256::new()],
            to_addr: vec![H256::new()],
        }
    }

    pub fn generate_trans(&self, num: usize) -> Vec<Transaction>  {
        let mut transactions: Vec<Transaction> = vec![];
        for _ in 0..num {
            transactions.push(self.create_transaction()); 
        }
        transactions
    } 

    fn create_transaction(&self)-> Transaction {
        let input = Input {
            tx_hash: H256::new(), 
            index: 0,
            unlock: H256::new(),
        };

        let output = Output {
            address: self.my_addr[0],
            value: 10,
        };
        let mut transaction = Transaction {
            inputs: vec![input], 
            outputs: vec![output], 
            is_coinbase: true,
            hash: H256::default(), 
        };
        transaction.update_hash();
        
        transaction
    }
}
