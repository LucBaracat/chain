use super::keygen::*;
// use bitcoin_hashes::sha256;
use secp256k1::hashes::sha256 as secpsha;
use secp256k1::{ecdsa::Signature, Message, PublicKey, SecretKey};

#[derive(Debug, Clone)]
pub struct Wallet {
    secret_key: SecretKey,
    pub public_key: PublicKey,
    pub address: String,
}

impl From<u64> for Wallet {
    fn from(seed: u64) -> Self {
        let (secret, public) = generate_curve_keys(seed);
        let address = address(public).unwrap();

        Self {
            secret_key: secret,
            public_key: public,
            address,
        }
    }
}

impl Wallet {
    pub fn sign(&mut self, hash: &[u8]) -> Signature {
        let context = secp256k1::Secp256k1::new();
        let message: Message = Message::from_hashed_data::<secpsha::Hash>(hash);
        context.sign_ecdsa(&message, &self.secret_key)
    }
}
