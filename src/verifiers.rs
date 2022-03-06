use crate::transactions::Block;
use crate::Db;

use hex::decode;
use secp256k1::hashes::sha256 as secpsha;
use secp256k1::{Message, Secp256k1};
use sha256::digest;

use crate::{genesis_hash, transactions::Tx, MINING_REWARD};

#[derive(Debug, Default)]
pub struct TxVerifier {}

impl TxVerifier {
    pub fn verify(&mut self, tx: &Tx, db: &Db) -> Option<u64> {
        let mut total_amount_in: u64 = 0;
        let mut total_amount_out: u64 = 0;

        // println!("Unspent txs: {:?}", self.unspent_txs_by_address);
        for (idx, input) in tx.inputs.iter().enumerate() {
            if input.previous_tx_hash == genesis_hash() && idx == 0 {
                total_amount_in = MINING_REWARD;
                continue;
            }
            let out = db
                .tx_by_hash
                .get(&input.previous_tx_hash)
                .map(|tx| tx.outputs[input.output_idx as usize].clone())?;
            let out_hash = out.hash?;

            total_amount_in += out.amount;

            if db
                .unspent_txs_by_address
                .get(&out.address)
                .map_or(false, |set| {
                    set.contains(&(input.previous_tx_hash.clone(), out_hash))
                })
            {
                let hash = decode(digest(format!(
                    "{:?}{}{}{}",
                    input.previous_tx_hash, input.output_idx, input.address, input.idx
                )))
                .expect("Coudln't decode");
                let secp = Secp256k1::new();
                let message: Message = Message::from_hashed_data::<secpsha::Hash>(&hash);

                match secp.verify_ecdsa(&message, &input.signature, &out.address) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("ECDSA Verification failed! {:#?}", e);
                        return None;
                    }
                }
            } else {
                println!("We already spent the output of the transaction!");
                return None;
            }
        }
        for output in tx.outputs.iter() {
            total_amount_out += output.amount;
        }
        if total_amount_in < total_amount_out {
            println!("Insufficient funds");
            return None;
        }
        Some(total_amount_in - total_amount_out)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BlockVerificationState {
    Success,
    WrongDifficulty,
    WrongRewardSum,
    WrongIdx,
    WrongHead,
    WrongTime,
}
pub struct BlockVerifier {
    db: Db,
    tx_verifier: TxVerifier,
}
impl BlockVerifier {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            tx_verifier: TxVerifier {},
        }
    }
    pub fn verify(&mut self, head: &mut Block, block: &mut Block) -> BlockVerificationState {
        let mut total_reward: u64 = MINING_REWARD;

        // Verify block Hash (For the Difficult)
        let hash = block.hash(None).unwrap();
        if !(hash[0] == 0x00 && hash[1] <= 0x0f) {
            println!("Hash is {:?}, which doesn't match the difficulty.", hash);
            return BlockVerificationState::WrongDifficulty;
        }

        // Veryify Txs in a block
        for tx in block.txs[1..block.txs.len()].iter() {
            let fee = self.tx_verifier.verify(tx, &self.db);
            total_reward += fee.unwrap();
        }

        let mut total_reward_out = 0;
        for out in block.txs[0].outputs.iter() {
            total_reward_out += out.amount;
        }

        // Verify the block reward
        if total_reward_out != total_reward {
            println!(
                "Total reward {total_reward} doesn't match total reward out {total_reward_out}!"
            );
            return BlockVerificationState::WrongRewardSum;
        }

        // Veryify rest
        if head.index >= block.index {
            println!("Block index number is wrong!");
            return BlockVerificationState::WrongIdx;
        }
        if head.hash(None).unwrap() != block.previous_hash {
            println!("New block is not pointed at the head!");
            return BlockVerificationState::WrongHead;
        }
        if head.time > block.time {
            println!("This is a block from the past.");
            return BlockVerificationState::WrongTime;
        }

        BlockVerificationState::Success
    }
}
