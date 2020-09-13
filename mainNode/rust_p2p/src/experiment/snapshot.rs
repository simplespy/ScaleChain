use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicUsize, AtomicU64, AtomicU32, Ordering, AtomicBool};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

lazy_static! {
    pub static ref PERFORMANCE_COUNTER: Counter = { Counter::default() };
}

#[derive(Default)]
pub struct Counter {
    generated_transactions: AtomicUsize,
    confirmed_transactions: AtomicUsize,
    chain_depth: AtomicUsize,
    token: AtomicBool,

    propose_block: AtomicUsize, // side node block id
    sign_block: AtomicUsize, // scale node # block id 0 for idle
    submit_block: AtomicUsize,   // is submitting blocks id 0 for idle

    propose_sec: AtomicU64,
    propose_millis: AtomicU32,
    propose_num: AtomicUsize,

    sign_sec: AtomicU64,
    sign_millis: AtomicU32,
    sign_num: AtomicU64,

    submit_sec: AtomicU64,
    submit_millis: AtomicU32,
    submit_num: AtomicU64,

    propose_latency: AtomicUsize,
    sign_latency: AtomicUsize,
    submit_latency: AtomicUsize,   // time taken to submit

    gas: AtomicUsize,

}

impl Counter {
    pub fn record_generated_transaction(&self) {
        self.generated_transactions.fetch_add(1, Ordering::Relaxed); 
    }
    pub fn record_confirmeded_transaction(&self) {
        self.confirmed_transactions.fetch_add(1, Ordering::Relaxed); 
    }

    pub fn record_generated_transactions(&self, num: usize) {
        self.generated_transactions.fetch_add(num, Ordering::Relaxed); 
    }

    pub fn record_confirmeded_transactions(&self, num: usize) {
        self.confirmed_transactions.fetch_add(num, Ordering::Relaxed); 
    }

    pub fn record_chain_update(&self) {
        self.chain_depth.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_gas_update(&self, gas: usize ) {
        self.gas.fetch_add(gas, Ordering::Relaxed);
    }

    // should not be used later
    pub fn store_chain_depth(&self, chain_len: usize) {
        self.chain_depth.store(chain_len, Ordering::Relaxed);
    }

    pub fn record_token_update(&self, new_flag: bool) {
        self.token.store(new_flag, Ordering::Relaxed);
    }

    fn get_times(&self) -> (u64, u32) {
        let dur = SystemTime::now().
            duration_since(SystemTime::UNIX_EPOCH).
            unwrap();
        let sec = dur.as_secs();   
        let millis = dur.subsec_millis();
        (sec, millis)
    }

    fn subtract_times(&self, sec: u64, millis: u32, psec: u64, pmillis: u32) -> usize {
        ((sec - psec) * 1000) as usize + millis as usize - pmillis as usize
    }

    pub fn record_propose_block_update(&self, id: u64) {
        self.propose_block.store(id as usize, Ordering::Relaxed);
        let (sec, millis) = self.get_times();
        self.propose_sec.store(sec, Ordering::Relaxed);
        self.propose_millis.store(millis, Ordering::Relaxed); 
    }
    
    pub fn record_propose_block_stop(&self) {
        let (sec, millis) = self.get_times();
        let psec = self.propose_sec.load(Ordering::Relaxed);
        let pmillis = self.propose_millis.load(Ordering::Relaxed);
        let lat = self.subtract_times(sec, millis, psec, pmillis);
        self.propose_latency.fetch_add(lat, Ordering::Relaxed);
        self.propose_num.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_sign_block_update(&self, id: u64) {
        self.sign_block.store(id as usize, Ordering::Relaxed);
        let (sec, millis) = self.get_times();
        self.sign_sec.store(sec, Ordering::Relaxed);
        self.sign_millis.store(millis, Ordering::Relaxed); 
    }

    pub fn record_sign_block_stop(&self) {
        let (sec, millis) = self.get_times();
        let psec = self.sign_sec.load(Ordering::Relaxed);
        let pmillis = self.sign_millis.load(Ordering::Relaxed);
        let lat = self.subtract_times(sec, millis, psec, pmillis);
        self.sign_latency.fetch_add(lat, Ordering::Relaxed);
        self.sign_num.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_submit_block_update(&self, id: u64) {
        self.submit_block.store(id as usize, Ordering::Relaxed);
        let (sec, millis) = self.get_times();
        self.submit_sec.store(sec, Ordering::Relaxed);
        self.submit_millis.store(millis, Ordering::Relaxed); 
    }

    pub fn record_submit_block_stop(&self) {
        let (sec, millis) = self.get_times();
        let psec = self.submit_sec.load(Ordering::Relaxed);
        let pmillis = self.submit_millis.load(Ordering::Relaxed);
        let lat = self.subtract_times(sec, millis, psec, pmillis);
        self.submit_latency.fetch_add(lat, Ordering::Relaxed);
        self.submit_num.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            generated_transactions: self.generated_transactions.load(Ordering::Relaxed),
            confirmed_transactions: self.confirmed_transactions.load(Ordering::Relaxed),
            chain_depth: self.chain_depth.load(Ordering::Relaxed),
            token: self.token.load(Ordering::Relaxed),
            propose_block: self.propose_block.load(Ordering::Relaxed),
            sign_block: self.sign_block.load(Ordering::Relaxed),
            submit_block: self.submit_block.load(Ordering::Relaxed),
            propose_latency: self.propose_latency.load(Ordering::Relaxed),
            sign_latency: self.sign_latency.load(Ordering::Relaxed),
            submit_latency: self.submit_latency.load(Ordering::Relaxed),
            gas: self.gas.load(Ordering::Relaxed),
            propose_num: self.propose_num.load(Ordering::Relaxed),
            sign_num: self.sign_num.load(Ordering::Relaxed) as usize,
            submit_num: self.submit_num.load(Ordering::Relaxed) as usize,
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshot {
    generated_transactions:  usize,
    confirmed_transactions:  usize,
    chain_depth:             usize,
    token:                   bool,

    propose_block:           usize,
    sign_block:              usize, // scale node # block id 0 for idle
    submit_block:            usize, 

    propose_latency:         usize,
    sign_latency:            usize,
    submit_latency:          usize,

    gas:                     usize,

    propose_num:             usize,
    sign_num:                usize,
    submit_num:              usize,
}
