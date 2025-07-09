use solana_sdk::{bs58, signature::Keypair};

pub trait KeypairBase58 {
    fn from_base58(private_key: &str) -> anyhow::Result<Keypair>;
}

impl KeypairBase58 for Keypair {
    fn from_base58(private_key: &str) -> anyhow::Result<Keypair> {
        let buf = bs58::decode(private_key.to_string()).into_vec()?;
        let keypair = Keypair::try_from(&buf[..])?;
        Ok(keypair)
    }
}
