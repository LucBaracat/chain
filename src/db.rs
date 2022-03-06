use crate::Tx;
use secp256k1::PublicKey;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Default, Clone)]
pub struct Db {
    pub block_index: u32,
    pub tx_by_hash: HashMap<Vec<u8>, Tx>,
    pub unspent_txs_by_address: HashMap<PublicKey, HashSet<(Vec<u8>, Vec<u8>)>>,
    pub unspent_outputs_amount: HashMap<PublicKey, HashMap<Vec<u8>, u64>>,
}
