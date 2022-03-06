mod blockchain;
mod db;
mod keygen;
mod transactions;
mod verifiers;
mod wallet;

use std::collections::HashSet;

use blockchain::*;
use db::*;
use hex::decode;
use transactions::*;
use wallet::*;

use crate::verifiers::TxVerifier;

pub const MINING_REWARD: u64 = 250;
pub const TXS_BY_BLOCK: usize = 4;
pub fn genesis_hash() -> Vec<u8> {
    decode(sha256::digest("GENESIS")).expect("Couldn't decode properly")
}

fn main() {
    test_verifier();
    test_split_brain();
}

fn test_verifier() -> Option<()> {
    let mut db = Db::default();
    let mut wallet_1 = Wallet::from(1337);
    let mut verifier = TxVerifier::default();

    let mut input = Input::new(&genesis_hash(), 0, Some(0), &mut wallet_1);
    let output = Output::new(wallet_1.public_key, 250, &[&input.hash().expect("AJKSLD")]);
    let mut tx = Tx::new(&[input], &[output]);

    let fee = verifier.verify(&tx, &db).expect("No fee!");
    assert_eq!(fee, 0);

    let txhash_outhash_pairs = txhash_outhash_pairs(&mut tx);
    db.unspent_txs_by_address
        .insert(wallet_1.public_key, txhash_outhash_pairs);

    db.tx_by_hash.insert(tx.hash().unwrap(), tx.clone());

    let mut input_2 = Input::new(&tx.hash().unwrap(), 0, Some(0), &mut wallet_1);
    let output_2 = Output::new(wallet_1.public_key, 250, &[&input_2.hash()?]);
    let tx_2 = Tx::new(&[input_2], &[output_2]);
    let fee = verifier.verify(&tx_2, &db).unwrap();
    assert_eq!(fee, 0);
    println!("Verifier Successful!");

    Some(())
}

fn txhash_outhash_pairs(tx: &mut Tx) -> HashSet<(Vec<u8>, Vec<u8>)> {
    let mut txhash_outhash_pairs = HashSet::new();
    let mut output_hashes = vec![];
    for output in tx.outputs.iter_mut() {
        let output_hash = output.hash().expect("No output Hash");
        output_hashes.push(output_hash);
    }
    let tx_hash = tx.hash().expect("No input Hash");
    for output_hash in output_hashes {
        txhash_outhash_pairs.insert((tx_hash.clone(), output_hash));
    }
    txhash_outhash_pairs
}

fn test_split_brain() {
    let wallet_1 = Wallet::from(1337);
    let db_1 = Db::default();
    let wallet_2 = Wallet::from(420);
    let db_2 = Db::default();

    let mut chain_1 = Blockchain::new(db_1, wallet_1);
    chain_1.genesis_block();

    let mut chain_2 = Blockchain::new(db_2, wallet_2);
    if let Some(head_block_chain_1) = chain_1.head() {
        chain_2.add_block(head_block_chain_1);
    } else {
        println!("Failed getting chain 1 head");
    }

    chain_1.force_block();
    chain_2.force_block();
    chain_1.add_block(chain_2.head().unwrap());
    chain_2.force_block();
    let added_2 = chain_1.add_block(chain_2.head().unwrap());
    assert_eq!(added_2, true);
    println!("Split brain successful!");
}

/*
TODOs for later once I care:

- Rollback ability
- Http endpoint for each node
- Yew Frontend
*/
