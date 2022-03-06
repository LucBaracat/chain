use std::collections::{HashMap, HashSet};

use crate::{
    db::Db,
    genesis_hash,
    transactions::{Block, Input, Output, Tx},
    verifiers::*,
    Wallet, MINING_REWARD, TXS_BY_BLOCK,
};

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub db: Db,
    pub wallet: Wallet,
    pub unconfirmed_txs: HashSet<(Vec<u8>, u64)>,
    pub current_block_txs: HashSet<(Vec<u8>, u64)>,

    pub chain: Vec<Block>,
    pub fork_blocks: HashMap<Vec<u8>, Block>,
}

impl Blockchain {
    pub fn new(db: Db, wallet: Wallet) -> Self {
        Self {
            db,
            wallet,
            unconfirmed_txs: Default::default(),
            current_block_txs: Default::default(),
            chain: Default::default(),
            fork_blocks: Default::default(),
        }
    }

    pub fn genesis_block(&mut self) {
        let tx = self.free_tx(None);
        let mut block = Block::new(&[tx], 0, &[0x00], None);
        self.mine_block(&mut block);
    }

    pub fn free_tx(&mut self, fee: Option<u64>) -> Tx {
        let mut input = Input::new(&genesis_hash(), 0, Some(0), &mut self.wallet);
        let output = Output::new(
            self.wallet.public_key,
            MINING_REWARD + fee.unwrap_or_default(),
            &[&input.hash().expect("Input hash failed in free tx")],
        );
        Tx::new(&[input], &[output])
    }

    fn mine_block(&mut self, block: &mut Block) {
        for nonce in 0..u32::MAX {
            let hash = block.hash(Some(nonce)).unwrap();
            // We use this as a stand-in for difficulty.
            if hash[0] == 0x00 && hash[1] <= 0x0f {
                self.add_block(block);
                self.rollover_block(block);
                println!(
                    "Block has been mined at nonce {nonce} and Hash looks like {:04X?}",
                    hash
                );
                break;
            }
        }
    }

    pub fn add_block(&mut self, block: &mut Block) -> bool {
        // Get the last block of the chain and check if the added block has identical hash
        if let Some(head) = self.chain.last_mut() {
            if head.hash(None) == block.hash(None) {
                println!("Duplicate block!");
                return false;
            }
            match BlockVerifier::new(self.db.clone()).verify(head, block) {
                BlockVerificationState::Success => {
                    self.chain.push(block.clone());
                    self.fork_blocks.clear();
                    return true;
                }
                BlockVerificationState::WrongIdx
                | BlockVerificationState::WrongHead
                | BlockVerificationState::WrongTime => {
                    if block.previous_hash == head.previous_hash {
                        println!("Split Brain detected!");
                        self.fork_blocks
                            .insert(block.hash(None).unwrap(), block.clone());
                        return false;
                    }
                    let mut blocks_to_add: Vec<Block> = vec![];
                    for (fork_block_hash, fork_block) in self.fork_blocks.iter() {
                        if block.previous_hash == *fork_block_hash {
                            println!("Split brain situation detected, picking longer brain.");
                            self.rollback_block();
                            blocks_to_add.push(fork_block.clone());
                            blocks_to_add.push(block.clone());
                            break;
                        }
                    }
                    if !blocks_to_add.is_empty() {
                        self.chain.extend(blocks_to_add);
                        self.fork_blocks.clear();
                        return true;
                    } else {
                        println!("Second split brain detected. Not fixing this because it's super unlikely.");
                        return false;
                    }
                }
                BlockVerificationState::WrongDifficulty
                | BlockVerificationState::WrongRewardSum => {
                    println!("Block verification failed");
                    return false;
                }
            }
        }
        self.chain.push(block.clone());
        self.fork_blocks.clear();
        true
    }

    pub fn _add_tx(&mut self, tx: &mut Tx) -> bool {
        let tx_hash = tx.hash().unwrap();
        if self.db.tx_by_hash.get(&tx_hash).is_some() {
            return false;
        }
        let mut verifier = TxVerifier::default();
        if let Some(fee) = verifier.verify(tx, &self.db) {
            self.db.tx_by_hash.insert(tx.hash().unwrap(), tx.clone());
            self.unconfirmed_txs.insert((tx_hash, fee));
            return true;
        }
        false
    }

    pub fn force_block(&mut self) {
        let mut a = self
            .unconfirmed_txs
            .clone()
            .into_iter()
            .collect::<Vec<(Vec<u8>, u64)>>();
        a.sort_by(|a, b| b.1.cmp(&a.1));
        let block = a.iter().rev().take(TXS_BY_BLOCK);
        self.current_block_txs = HashSet::from_iter(block.cloned());
        println!(
            "Current block transactions are {:?}",
            self.current_block_txs
        );
        let total_fee: u64 = self.current_block_txs.iter().map(|x| x.1).sum();
        let mut txs: Vec<Tx> = vec![self.free_tx(Some(total_fee))];
        for (hash, _fee) in &self.current_block_txs {
            let tx = self.db.tx_by_hash.get(hash).unwrap();
            txs.push(tx.clone());
        }
        let new_index = if let Some(head) = self.head() {
            head.index + 1
        } else {
            0
        };
        let previous_hash: Vec<u8> = self
            .head()
            .map_or([0x00].to_vec(), |block| block.hash(None).unwrap());
        let mut block = Block::new(&txs, new_index, &previous_hash, None);
        self.mine_block(&mut block);
    }

    fn rollback_block(&self) {
        // TODO: Add rollback
    }

    fn rollover_block(&mut self, block: &mut Block) {
        for current_block_txs in self.current_block_txs.iter() {
            self.unconfirmed_txs.remove(current_block_txs);
        }
        self.db.block_index = block.index;
        for tx in block.txs.iter() {
            self.db
                .tx_by_hash
                .insert(tx.clone().hash().unwrap(), tx.clone());
            for output in tx.outputs.iter() {
                self.db
                    .unspent_txs_by_address
                    .entry(output.address)
                    .and_modify(|hash_set| {
                        hash_set
                            .insert((tx.clone().hash().unwrap(), output.clone().hash().unwrap()));
                    })
                    .or_insert(HashSet::default());
                let entry = self
                    .db
                    .unspent_outputs_amount
                    .entry(output.address)
                    .or_insert(HashMap::default());
                entry.insert(output.clone().hash().unwrap(), output.amount);
            }
            for input in tx.inputs.iter() {
                if input.previous_tx_hash == genesis_hash() {
                    continue;
                }
                let prev_output = self
                    .db
                    .tx_by_hash
                    .get(&input.previous_tx_hash)
                    .unwrap()
                    .clone()
                    .outputs[input.output_idx]
                    .clone();
                self.db
                    .unspent_txs_by_address
                    .entry(prev_output.address)
                    .and_modify(|set| {
                        set.remove(&(
                            input.previous_tx_hash.clone(),
                            prev_output.clone().hash().unwrap(),
                        ));
                    });
                // let a = self.db.unspent_outputs_amount.entry(prev_output.address)
                let entry = self
                    .db
                    .unspent_outputs_amount
                    .entry(prev_output.address)
                    .or_default();
                entry.remove(&prev_output.clone().hash().unwrap());
            }
        }
        self.current_block_txs.clear();
    }

    pub fn head(&mut self) -> Option<&mut Block> {
        self.chain.last_mut()
    }
}
