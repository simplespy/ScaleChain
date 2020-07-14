use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicUsize, Ordering};



lazy_static! {
    pub static ref PERFORMANCE_COUNTER: Counter = { Counter::default() };
}

#[derive(Default)]
pub struct Counter {
    generated_transactions: AtomicUsize,
    chain_depth: AtomicUsize,
}

impl Counter {
    pub fn record_generated_transactions(&self) {
        self.generated_transactions.fetch_add(1, Ordering::Relaxed); 
    }

    pub fn record_chain_update(&self) {
        self.chain_depth.fetch_add(1, Ordering::Relaxed);
    }

    // should not be used later
    pub fn store_chain_depth(&self, chain_len: usize) {
        self.chain_depth.store(chain_len, Ordering::Relaxed);
    }



    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            generated_transactions: self.generated_transactions.load(Ordering::Relaxed),
            chain_depth: self.chain_depth.load(Ordering::Relaxed),
        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshot {
    generated_transactions:  usize,
    chain_depth:             usize,
}
