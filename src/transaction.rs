use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::RpcClient;

/// A destination for an outgoing transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    /// Recipient Monero address.
    pub address: String,
    /// Amount in atomic units (1 XMR = 1e12 piconero).
    pub amount: u64,
}

/// Priority level for transaction fee estimation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Default = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

/// An unsigned multisig transaction awaiting co-signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedMultisigTx {
    /// Hex-encoded unsigned transaction data from the wallet RPC.
    pub tx_data_hex: String,
    /// Transaction hash (available after construction).
    pub tx_hash: String,
    /// Fee in atomic units.
    pub fee: u64,
}

/// A partially signed multisig transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartiallySignedTx {
    /// Hex-encoded transaction data with at least one co-signature applied.
    pub tx_data_hex: String,
    /// Transaction hash.
    pub tx_hash: String,
    /// Number of signatures collected so far.
    pub signatures_count: u32,
    /// Number of signatures required to broadcast.
    pub signatures_required: u32,
}

/// The result of submitting a fully signed transaction to the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResult {
    /// Transaction hash as confirmed by the daemon.
    pub tx_hash: String,
}

// ── Wallet RPC response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TransferResponse {
    tx_hash: String,
    fee: u64,
    multisig_txset: String,
}

#[derive(Debug, Deserialize)]
struct SignMultisigResponse {
    tx_hash_list: Vec<String>,
    tx_data_hex: String,
}

#[derive(Debug, Deserialize)]
struct SubmitMultisigResponse {
    tx_hash_list: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExportMultisigInfoResponse {
    info: String,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Export this wallet's partial key images so co-signers can see the correct
/// balance. Must be called (and results shared) before building transactions.
pub async fn export_multisig_info(rpc: &RpcClient) -> Result<String> {
    let resp: ExportMultisigInfoResponse = rpc
        .request("export_multisig_info", &serde_json::json!({}))
        .await
        .context("export_multisig_info RPC call failed")?;

    Ok(resp.info)
}

/// Import partial key images from co-signers to synchronize balance state.
pub async fn import_multisig_info(rpc: &RpcClient, info: &[String]) -> Result<()> {
    let _: serde_json::Value = rpc
        .request(
            "import_multisig_info",
            &serde_json::json!({ "info": info }),
        )
        .await
        .context("import_multisig_info RPC call failed")?;

    Ok(())
}

/// Build an unsigned multisig transaction.
///
/// Requires that multisig info has been exchanged between all participants via
/// [`export_multisig_info`] / [`import_multisig_info`] so the wallet has an
/// accurate view of the available balance.
pub async fn build_unsigned_tx(
    rpc: &RpcClient,
    destinations: &[Destination],
    priority: Priority,
) -> Result<UnsignedMultisigTx> {
    let dest_params: Vec<_> = destinations
        .iter()
        .map(|d| {
            serde_json::json!({
                "address": d.address,
                "amount": d.amount,
            })
        })
        .collect();

    let resp: TransferResponse = rpc
        .request(
            "transfer",
            &serde_json::json!({
                "destinations": dest_params,
                "priority": priority as u32,
                "get_tx_hex": false,
                "do_not_relay": true,
            }),
        )
        .await
        .context("transfer RPC call failed")?;

    Ok(UnsignedMultisigTx {
        tx_data_hex: resp.multisig_txset,
        tx_hash: resp.tx_hash,
        fee: resp.fee,
    })
}

/// Apply this participant's signature to a multisig transaction set.
///
/// Each co-signer calls this with the same `tx_data_hex` received from the
/// transaction builder. Once enough signatures are collected, the transaction
/// can be submitted.
pub async fn sign_multisig_tx(
    rpc: &RpcClient,
    tx_data_hex: &str,
) -> Result<PartiallySignedTx> {
    let resp: SignMultisigResponse = rpc
        .request(
            "sign_multisig",
            &serde_json::json!({
                "tx_data_hex": tx_data_hex,
            }),
        )
        .await
        .context("sign_multisig RPC call failed")?;

    let tx_hash = resp
        .tx_hash_list
        .into_iter()
        .next()
        .unwrap_or_default();

    Ok(PartiallySignedTx {
        tx_data_hex: resp.tx_data_hex,
        tx_hash,
        signatures_count: 0,  // actual count tracked externally
        signatures_required: 0,
    })
}

/// Submit a fully signed multisig transaction to the Monero network.
pub async fn submit_multisig_tx(
    rpc: &RpcClient,
    tx_data_hex: &str,
) -> Result<SubmitResult> {
    let resp: SubmitMultisigResponse = rpc
        .request(
            "submit_multisig",
            &serde_json::json!({
                "tx_data_hex": tx_data_hex,
            }),
        )
        .await
        .context("submit_multisig RPC call failed")?;

    let tx_hash = resp
        .tx_hash_list
        .into_iter()
        .next()
        .unwrap_or_default();

    Ok(SubmitResult { tx_hash })
}

/// Format an atomic-unit amount as a human-readable XMR string.
pub fn format_xmr(piconero: u64) -> String {
    let whole = piconero / 1_000_000_000_000;
    let frac = piconero % 1_000_000_000_000;
    format!("{whole}.{frac:012}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_xmr_whole() {
        assert_eq!(format_xmr(1_000_000_000_000), "1.000000000000");
    }

    #[test]
    fn test_format_xmr_fractional() {
        assert_eq!(format_xmr(1_500_000_000), "0.001500000000");
    }

    #[test]
    fn test_format_xmr_zero() {
        assert_eq!(format_xmr(0), "0.000000000000");
    }
}
