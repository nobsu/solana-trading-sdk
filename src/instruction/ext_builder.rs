use crate::instruction::builder::{PriorityFee, TipFee};
use crate::{common::accounts::PUBKEY_WSOL, dex::types::CreateATA};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
    system_instruction,
};

#[derive(Debug, Clone, Copy)]
pub struct NonceInfo {
    pub nonce_account: Pubkey,
    pub nonce_authority: Pubkey,
}

pub fn build_transaction_ext(
    payer: &Keypair,
    instructions: Vec<Instruction>,
    blockhash: Hash,
    fee: Option<PriorityFee>,
    tip: Option<TipFee>,
    other_signers: Option<Vec<&Keypair>>,
    nonce_info: Option<NonceInfo>,
) -> anyhow::Result<VersionedTransaction> {
    let mut insts = vec![];

    // add AdvanceNonce
    if let Some(nonce) = &nonce_info {
        insts.push(system_instruction::advance_nonce_account(
            &nonce.nonce_account,
            &nonce.nonce_authority,
        ));
    }

    if let Some(fee) = fee {
        insts.push(ComputeBudgetInstruction::set_compute_unit_price(fee.unit_price));
        insts.push(ComputeBudgetInstruction::set_compute_unit_limit(fee.unit_limit));
    }

    if let Some(tip) = tip {
        insts.push(solana_sdk::system_instruction::transfer(&payer.pubkey(), &tip.tip_account, tip.tip_lamports));
    }

    insts.extend(instructions);

    let v0_message: v0::Message = v0::Message::try_compile(&payer.pubkey(), &insts, &[], blockhash)?;
    let versioned_message: VersionedMessage = VersionedMessage::V0(v0_message);
    let signers = vec![payer].into_iter().chain(other_signers.unwrap_or_default().into_iter()).collect::<Vec<_>>();
    let transaction = VersionedTransaction::try_new(versioned_message, &signers)?;

    Ok(transaction)
}
