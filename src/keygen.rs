use pcg_rand::Pcg64;
use rand::Rng;
use rand::SeedableRng;
use ripemd::{Digest, Ripemd160};

use secp256k1::{PublicKey, SecretKey};

pub fn generate_curve_keys(seed: u64) -> (SecretKey, PublicKey) {
    let context = secp256k1::Secp256k1::new();
    let mut rng = Pcg64::seed_from_u64(seed);
    let random_val = &rng.gen::<[u8; 32]>();
    let secret_key = SecretKey::from_slice(random_val).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&context, &secret_key);
    let _private_key = secret_key.display_secret();
    // println!("Private key: {:?}", private_key);
    // println!("Public key compressed: {:?}", public_key.serialize());
    // println!(
    //     "Public key uncompressed: {:?}",
    //     public_key.serialize_uncompressed()
    // );
    (secret_key, public_key)
}

pub fn address(public_key: PublicKey) -> Option<String> {
    // Sha256 the public key
    let sha256 = sha256::digest_bytes(&public_key.serialize());
    // Ripemd160 the sha256
    let mut ripemd_hasher = Ripemd160::new();
    ripemd_hasher.update(sha256);
    let result = ripemd_hasher.finalize();
    // Compute checksum by double-sha256ing the first 4 bytes
    let mut checksum = hex::decode(sha256::digest(sha256::digest_bytes(&result[0..4]))).ok()?;
    // Concat result and checksum
    let mut new_result = result.to_vec();
    new_result.append(&mut checksum);
    // b58 encode the byte address.
    let encoded = bs58::encode(new_result).into_string();
    Some(encoded)
}
