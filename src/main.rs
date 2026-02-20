use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use monero_multisig::config::{Config, RpcClient};
use monero_multisig::transaction;
use monero_multisig::wallet;

#[derive(Parser)]
#[command(
    name = "monero-multisig",
    about = "Monero M-of-N multisig wallet tool",
    version,
    long_about = "Create and manage Monero multisig wallets. Supports arbitrary M-of-N \
                  configurations, multi-round key exchange, and cooperative transaction signing."
)]
struct Cli {
    /// Path to a JSON configuration file.
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Monero daemon RPC host.
    #[arg(long, global = true, default_value = "127.0.0.1")]
    daemon_host: String,

    /// Monero daemon RPC port.
    #[arg(long, global = true, default_value_t = 18081)]
    daemon_port: u16,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new multisig wallet and output your multisig info for sharing.
    CreateWallet {
        /// Required number of signers (M).
        #[arg(short = 'm', long)]
        threshold: u32,

        /// Total number of participants (N).
        #[arg(short = 'n', long)]
        participants: u32,

        /// Human-readable wallet label.
        #[arg(short, long, default_value = "default")]
        label: String,
    },

    /// Perform a key exchange round with peer multisig info strings.
    ExchangeKeys {
        /// Multisig info strings from other participants (one per peer).
        #[arg(short, long, num_args = 1..)]
        info: Vec<String>,

        /// Wallet password.
        #[arg(short, long, default_value = "")]
        password: String,
    },

    /// Export multisig info for balance synchronization.
    ExportInfo,

    /// Import multisig info from co-signers before building transactions.
    ImportInfo {
        /// Multisig info strings from co-signers.
        #[arg(short, long, num_args = 1..)]
        info: Vec<String>,
    },

    /// Build an unsigned transaction and output the multisig tx set.
    BuildTx {
        /// Recipient address.
        #[arg(short, long)]
        address: String,

        /// Amount in atomic units (piconero).
        #[arg(short = 'x', long)]
        amount: u64,

        /// Transaction priority (0=default, 1=low, 2=medium, 3=high).
        #[arg(short, long, default_value_t = 0)]
        priority: u32,
    },

    /// Apply this participant's signature to a multisig transaction set.
    SignTx {
        /// Hex-encoded multisig transaction set data.
        #[arg(short, long)]
        tx_data: String,
    },

    /// Submit a fully signed multisig transaction to the network.
    SubmitTx {
        /// Hex-encoded fully signed transaction data.
        #[arg(short, long)]
        tx_data: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = Config::load(cli.config.as_ref())?;

    let mut daemon = config.daemon.clone();
    daemon.host = cli.daemon_host;
    daemon.port = cli.daemon_port;

    let rpc = RpcClient::new(&daemon);

    match cli.command {
        Command::CreateWallet {
            threshold,
            participants,
            label,
        } => {
            let params = wallet::MultisigParams::new(threshold, participants, label)?;
            println!(
                "Creating {}-of-{} multisig wallet \"{}\"...",
                params.threshold, params.total, params.label
            );

            let info = wallet::prepare_multisig(&rpc).await?;

            let state = wallet::WalletState::Created {
                wallet_path: config.data_dir.join("wallet"),
                params: wallet::SerializableParams::from(&params),
            };
            wallet::save_wallet_state(&config.data_dir, &state)?;

            println!("\nYour multisig info (share with all other participants):\n");
            println!("{info}");
        }

        Command::ExchangeKeys { info, password } => {
            let state = wallet::load_wallet_state(&config.data_dir)
                .context("load wallet state")?;

            let threshold = match &state {
                wallet::WalletState::Created { params, .. }
                | wallet::WalletState::KeyExchangeInProgress { params, .. } => params.threshold,
                wallet::WalletState::Ready { .. } => {
                    anyhow::bail!("wallet is already fully set up");
                }
            };

            println!("Performing key exchange round...");
            let result = wallet::exchange_keys(&rpc, &info, threshold, &password).await?;

            match result {
                wallet::KeyExchangeResult::Partial { next_info } => {
                    println!("\nKey exchange round complete. More rounds needed.");
                    println!("Share this info with peers for the next round:\n");
                    println!("{next_info}");
                }
                wallet::KeyExchangeResult::Complete { address } => {
                    let state = wallet::WalletState::Ready {
                        wallet_path: config.data_dir.join("wallet"),
                        address: address.clone(),
                        params: match state {
                            wallet::WalletState::Created { params, .. }
                            | wallet::WalletState::KeyExchangeInProgress { params, .. } => params,
                            _ => unreachable!(),
                        },
                    };
                    wallet::save_wallet_state(&config.data_dir, &state)?;

                    println!("\nMultisig wallet is ready!");
                    println!("Address: {address}");
                }
            }
        }

        Command::ExportInfo => {
            let info = transaction::export_multisig_info(&rpc).await?;
            println!("Multisig info (share with co-signers):\n");
            println!("{info}");
        }

        Command::ImportInfo { info } => {
            transaction::import_multisig_info(&rpc, &info).await?;
            println!("Multisig info imported successfully. Balance is now synchronized.");
        }

        Command::BuildTx {
            address,
            amount,
            priority,
        } => {
            let priority = match priority {
                1 => transaction::Priority::Low,
                2 => transaction::Priority::Medium,
                3 => transaction::Priority::High,
                _ => transaction::Priority::Default,
            };

            let destinations = vec![transaction::Destination { address, amount }];

            println!("Building unsigned multisig transaction...");
            let unsigned = transaction::build_unsigned_tx(&rpc, &destinations, priority).await?;

            println!("\nTransaction built successfully:");
            println!("  Hash: {}", unsigned.tx_hash);
            println!("  Fee:  {} XMR", transaction::format_xmr(unsigned.fee));
            println!("\nMultisig tx set (share with co-signers):\n");
            println!("{}", unsigned.tx_data_hex);
        }

        Command::SignTx { tx_data } => {
            println!("Signing multisig transaction...");
            let signed = transaction::sign_multisig_tx(&rpc, &tx_data).await?;

            println!("\nSignature applied:");
            println!("  Hash: {}", signed.tx_hash);
            println!("\nUpdated tx set (share with remaining co-signers or submit):\n");
            println!("{}", signed.tx_data_hex);
        }

        Command::SubmitTx { tx_data } => {
            println!("Submitting fully signed transaction...");
            let result = transaction::submit_multisig_tx(&rpc, &tx_data).await?;

            println!("\nTransaction submitted successfully!");
            println!("  Hash: {}", result.tx_hash);
        }
    }

    Ok(())
}
