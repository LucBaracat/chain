use std::time::SystemTime;

use hex::decode;
use rs_merkle::{algorithms::Sha256, MerkleTree};
use secp256k1::{ecdsa::Signature, PublicKey};
use sha256::digest;

use crate::wallet::Wallet;

#[derive(Debug, Clone)]
pub struct Input {
    pub previous_tx_hash: Vec<u8>,
    pub output_idx: usize,
    pub address: String,
    pub idx: u32,
    pub hash: Option<Vec<u8>>,
    pub signature: Signature,
    pub amount: u64,
}
impl Input {
    pub fn new(
        previous_tx_hash: &[u8],
        output_idx: usize,
        index: Option<u32>,
        wallet: &mut Wallet,
    ) -> Self {
        let previous_tx_hash = previous_tx_hash.to_vec();
        let output_idx = output_idx;
        let address = wallet.address.to_string();
        let idx = index.unwrap_or_default();
        let hash_string = digest(format!(
            "{:?}{}{}{}",
            previous_tx_hash, output_idx, address, idx
        ));
        let content = decode(hash_string).expect("Couldn't decode");
        let signature = wallet.sign(&content);
        Self {
            previous_tx_hash,
            output_idx,
            address,
            idx,
            hash: None,
            signature,
            amount: 0,
        }
    }

    pub fn hash(&mut self) -> Option<Vec<u8>> {
        if let Some(_hash) = &self.hash {
            return self.hash.clone();
        }

        let hash_string = format!(
            "{:?}{}{}{:?}{}",
            self.previous_tx_hash, self.output_idx, self.address, self.signature, self.idx
        );

        let hash = decode(sha256::digest(sha256::digest(hash_string))).expect("Coudln't decode");
        self.hash = Some(hash);
        return self.hash.clone();
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    pub address: PublicKey,
    pub idx: usize,
    pub amount: u64,
    pub input_hash: Vec<u8>,
    pub hash: Option<Vec<u8>>,
}
impl Output {
    pub fn new(address: PublicKey, amount: u64, input_hashes: &[&[u8]]) -> Self {
        let mut input_hash_str: String = "".to_string();
        for input_hash in input_hashes.iter() {
            input_hash_str += &format!("{:?}", input_hash);
        }
        let input_hash = decode(digest(input_hash_str)).expect("Couldn't decode");
        Self {
            address,
            idx: 0,
            amount,
            input_hash,
            hash: None,
        }
    }

    pub fn hash(&mut self) -> Option<Vec<u8>> {
        if let Some(_hash) = &self.hash {
            return self.hash.clone();
        }
        let hash_string = format!(
            "{}{}{}{:?}",
            self.amount, self.idx, self.address, self.input_hash,
        );

        self.hash =
            Some(decode(sha256::digest(sha256::digest(hash_string))).expect("Couldn't decode"));

        return self.hash.clone();
    }
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: SystemTime,
    pub hash: Option<Vec<u8>>,
}

impl Tx {
    pub fn new(inputs: &[Input], outputs: &[Output]) -> Self {
        Self {
            inputs: inputs.clone().to_vec(),
            outputs: outputs.clone().to_vec(),
            time: SystemTime::now(),
            hash: None,
        }
    }

    pub fn hash(&mut self) -> Option<Vec<u8>> {
        // Return hash if we have it already
        if self.hash.is_some() {
            return self.hash.clone();
        }
        // Convert out current timestamp to seconds since `UNIX_EPOCH`
        let seconds = self
            .time
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_secs();

        // Create an input has string from all the inputs
        let mut input_hash_string: String = "".to_string();
        for input in &self.inputs {
            input_hash_string += &format!("{:?}", input);
        }
        input_hash_string += &seconds.to_string();

        let mut hash_string: String = "".to_string();

        assert_eq!(self.inputs.len(), self.outputs.len());
        for input in self.inputs.iter() {
            hash_string += &format!("{:?}", input);
        }
        for output in self.outputs.iter() {
            hash_string += &format!("{:?}", output);
        }
        hash_string += &seconds.to_string();
        let hash = decode(sha256::digest(sha256::digest(hash_string))).expect("Coudln't decode");

        Some(hash)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub txs: Vec<Tx>,
    pub previous_hash: Vec<u8>,
    pub index: u32,
    pub nonce: u32,
    pub time: SystemTime,
    pub merkel_root: Option<[u8; 32]>, // Danke Merkel
}

impl Block {
    pub fn new(txs: &[Tx], index: u32, previous_hash: &[u8], nonce: Option<u32>) -> Self {
        Self {
            txs: txs.to_vec(),
            previous_hash: previous_hash.to_vec(),
            index,
            nonce: nonce.unwrap_or(0),
            time: SystemTime::now(),
            merkel_root: None,
        }
    }

    pub fn build_merkel_tree(&mut self) -> Option<[u8; 32]> {
        if self.merkel_root.is_some() {
            return self.merkel_root;
        }
        let leaves: Vec<Vec<u8>> = self.txs.iter_mut().map(|tx| tx.hash().unwrap()).collect();
        let mut new_leaves = vec![];
        for leaf in leaves {
            let leaf_slice: [u8; 32] = leaf[0..32].try_into().expect("slice with incorrect length");
            new_leaves.push(leaf_slice);
        }
        let merkle_tree = MerkleTree::<Sha256>::from_leaves(&new_leaves);
        merkle_tree
            .root()
            .ok_or("couldn't get the merkle root")
            .ok()
    }

    pub fn hash(&mut self, nonce: Option<u32>) -> Option<Vec<u8>> {
        if let Some(nonce) = nonce {
            self.nonce = nonce;
        }
        let merkel_tree = self.build_merkel_tree();
        if let Some(merkel_tree) = merkel_tree {
            let block_string: String = format!(
                "{}{:?}{:?}{}{:?}",
                self.nonce, merkel_tree, self.previous_hash, self.index, self.time
            );
            let result = decode(digest(block_string)).expect("Couldn't decode properly");
            return Some(result);
        }
        None
    }
}
