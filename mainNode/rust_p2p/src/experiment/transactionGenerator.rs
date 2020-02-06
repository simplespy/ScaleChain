use super::mempool::mempool::Mempool;
use super::hash::{H256};
use super::block::{Transaction, Input, Output};

use std::sync::{Mutex, Arc};
use std::thread;
use crossbeam::channel::{self, Sender, Receiver, TryRecvError};


pub enum TxGenSignal {
    Start,
    Stop,
    Step(usize),
}

pub enum State {
    Continuous,
    Pause,
    Step(usize),
}

pub struct TransactionGenerator {
    control: channel::Receiver<TxGenSignal>,
    mempool: Arc<Mutex<Mempool>>,
    my_addr: Vec<H256>,
    to_addr: Vec<H256>,
    state: State,
    total_tx: usize,
}

impl TransactionGenerator {
    pub fn new(mempool: Arc<Mutex<Mempool>>) -> (TransactionGenerator, Sender<TxGenSignal>) {
        let (tx, rx) = channel::unbounded();
        let transaction_gen = TransactionGenerator {
            mempool: mempool,
            my_addr: vec![H256::new()],
            to_addr: vec![H256::new()],
            control: rx,
            state: State::Pause,
            total_tx: 0,
        };
        (transaction_gen, tx)
    }

    pub fn start(mut self) {
        let _ = thread::spawn(move || {
            loop {
                match self.state {
                    State::Pause => {
                        let signal = self.control.recv().expect("Tx Gen control signal");
                        self.handle_signal(signal); 
                    },
                    State::Step(num_tx) => {
                        let transactions = self.generate_trans(num_tx);
                        self.send_to_mempool(transactions);

                        self.state = State::Pause;
                    },
                    State::Continuous => {
                        // create transaction according to some distribution
                        match self.control.try_recv() {
                            Ok(signal) => {
                                self.handle_signal(signal);
                            },
                            Err(TryRecvError::Empty) => {
                                let transaction = self.generate_trans(1);
                                self.send_to_mempool(transaction);
                            },
                            Err(TryRecvError::Disconnected) => panic!("disconnected tx_gen control signal"),
                        }
                    }
                }
            }
        });
    }

    fn send_to_mempool(&mut self, transactions: Vec<Transaction>) {
        let mut mempool = self.mempool.lock().expect("tx gen lock mempool");
        for tx in transactions {
            mempool.insert(tx);
        }
        drop(mempool);
    }

    pub fn handle_signal(&mut self, signal: TxGenSignal) {
        match signal {
            TxGenSignal::Start => {
                self.state = State::Continuous;
            },
            TxGenSignal::Stop => {
                self.state = State::Pause;
            },
            TxGenSignal::Step(num) => {
                self.state = State::Step(num);
                println!("rx step {}", num);
            },
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
