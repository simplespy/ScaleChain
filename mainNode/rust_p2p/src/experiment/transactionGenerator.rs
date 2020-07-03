use super::mempool::mempool::Mempool;
use super::hash::{H256};
//use super::block::{Transaction, Input, Output};

use std::sync::{Mutex, Arc};
use std::thread;
use std::fs::File;
use std::io::{Write, BufReader, BufRead, Error};
use crossbeam::channel::{self, Sender, Receiver, TryRecvError};
use super::snapshot::PERFORMANCE_COUNTER;
use chain::transaction::{Transaction, TransactionInput, TransactionOutput, OutPoint};
use primitives::bytes::Bytes;

use requests::{ToJson};
use rand::Rng;

pub enum TxGenSignal {
    Start,
    Stop,
    Step(usize),
    Simulate,
}

pub enum State {
    Continuous,
    Pause,
    Step(usize),
    Simulate,
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
                    },
                    State::Simulate => {
                        let transactions= self.generate_transaction_from_history();
                        self.estimate_gas(transactions);
                        self.state = State::Pause;
                    }
                }
            }
        });
    }

    fn estimate_gas(&mut self, transactions: Vec<Transaction>) {
        let mut mempool = self.mempool.lock().expect("tx gen lock mempool");
        for tx in transactions {
            mempool.estimate_gas(tx);
        }
        drop(mempool);
    }

    fn send_to_mempool(&mut self, transactions: Vec<Transaction>) {
        let mut mempool = self.mempool.lock().expect("tx gen lock mempool");
        info!("insert {}  transaction", transactions.len());
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
            },
            TxGenSignal::Simulate => {
                self.state = State::Simulate;
                println!("Start simulation");
            }
        }
    }

    pub fn generate_trans(&self, num: usize) -> Vec<Transaction>  {
        let mut transactions: Vec<Transaction> = vec![];
        for _ in 0..num {
            transactions.push(self.create_transaction()); 
            PERFORMANCE_COUNTER.record_generated_transactions();
        }
        transactions
    }

    fn create_transaction(&self)-> Transaction {
        let mut rng = rand::thread_rng();
        let bytes_size = 128;
        let input = TransactionInput {
            previous_output: OutPoint::default(),
            script_sig: Bytes::new_with_len(bytes_size), //magic
            sequence: 0,
            script_witness: vec![],
        };


        let output = TransactionOutput {
            value: rng.gen(),
            script_pubkey: Bytes::new_with_len(bytes_size),
        };


        let tx = Transaction {
            version: 0,
            inputs: vec![input],
            outputs: vec![output] ,
            lock_time: rng.gen(),
        };
        tx
    }



    fn generate_transaction_from_history(&self) -> Vec<Transaction> {
        // please change

        //let mut file = File::create("gas_history.csv").unwrap();
        let mut transactions: Vec<Transaction> = vec![];
        let request_url = format!("http://api.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}&page={page}&offset={offset}",
                                  address = "0x06012c8cf97bead5deae237070f9587f8e7a266d",//"0x732de7495deecae6424c3fc3c46e47d6b4c5374e",
                                  start = 5752558,
                                  end = 9463322,
                                  apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX",
                                  page = 1,
                                  offset = 1000);
        ////let request_url = format!("http://api.etherscan.io/api?module=account&action=txlist&address={address}&startblock={start}&endblock={end}&sort=asc&apikey={apikey}&page={page}&offset={offset}",
                                  ////address = "0x1985365e9f78359a9B6AD760e32412f4a445E862",
                                  ////start = 8752558,
                                  ////end = 9478235,
                                  ////apikey = "UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX",
                                  ////page = 1,
                                  ////offset = 1000);
        //println!("{:?}", request_url);
        let response = requests::get(request_url).unwrap();
        let data = response.json().unwrap();
        let txs = data["result"].clone();
        let mut i = 0;
        for tx in txs.members() {
            let isError = tx["isError"].as_str().unwrap().parse::<i32>().unwrap();

            if isError == 0 && tx["to"].as_str().unwrap() == "0x06012c8cf97bead5deae237070f9587f8e7a266d" {
                ////if isError == 0 && tx["to"].as_str().unwrap() == "0x1985365e9f78359a9b6ad760e32412f4a445e862" {


                let mut transaction = Transaction::default();
                let content = String::from(tx["input"].as_str().unwrap()).replace("0x", "");
                let mut txinput = TransactionInput::coinbase(Bytes::from(hex::decode(content.as_str()).expect("decode error")));
                transaction.inputs.push(txinput);
                //let tx_hash = String::from(tx["hash"].as_str().unwrap());
                //let content = String::from(tx["input"].as_str().unwrap()).replace("0x", "");
                //let address = String::from(tx["from"].as_str().unwrap());
                //let gas_used = String::from(tx["gas"].as_str().unwrap());
                //let gas_used = usize::from_str_radix(&gas_used, 10).unwrap();
                i += 1;
                //file.write_all(format!("{},{}\n",i,gas_used).as_bytes());
                //let mut tx = Transaction{
                    //inputs: vec![Input {
                        //tx_hash: H256::from(tx_hash),
                        //index: 0,
                        //unlock: H256::new(),
                        //content: hex::decode(&content).unwrap()
                    //}],
                    //outputs: vec![Output {
                        //address: self.my_addr[0],
                        //value: 10
                    //}],
                    //is_coinbase: false,
                    //hash: H256::default()
                //};
                //tx.update_hash();
                transactions.push(transaction);
            }
        }
        println!("generate {} txs from history", transactions.len());
        return transactions;

    }
}
