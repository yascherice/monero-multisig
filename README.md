# monero-multisig

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A command-line tool for creating and managing Monero M-of-N multisig wallets. Build cooperative custody setups where multiple parties must agree before funds can move.

## Features

- **Flexible M-of-N configurations** — 2-of-3, 3-of-5, or any valid threshold
- **Multi-round key exchange** — handles the full Monero multisig setup protocol
- **Transaction co-signing** — build, partially sign, and submit multisig transactions
- **Balance synchronization** — export/import partial key images between co-signers
- **Daemon RPC integration** — communicates directly with `monerod` via JSON-RPC
- **Persistent state** — wallet setup progress is saved locally between sessions

## Architecture

```
┌──────────────────────────────────────────────────┐
│                    CLI (clap)                     │
│  create-wallet · exchange-keys · sign-tx · ...   │
├──────────────┬──────────────────┬────────────────┤
│   wallet.rs  │  transaction.rs  │   config.rs    │
│              │                  │                 │
│  Key setup   │  Build, sign,    │  RPC client,   │
│  & exchange  │  & submit txs    │  settings       │
├──────────────┴──────────────────┴────────────────┤
│              Monero Daemon (monerod)              │
│                 JSON-RPC API                      │
└──────────────────────────────────────────────────┘
```

### Module Overview

| Module | Purpose |
|---|---|
| `main.rs` | CLI argument parsing and command dispatch |
| `wallet.rs` | Multisig wallet creation, key exchange rounds, state persistence |
| `transaction.rs` | Unsigned tx building, partial signing, submission, balance sync |
| `config.rs` | Daemon RPC connection, JSON config loading, JSON-RPC client |

## Prerequisites

- **Rust** 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Monero daemon** (`monerod`) running with RPC enabled
- **Monero wallet RPC** (`monero-wallet-rpc`) for each participant

## Build

```bash
git clone https://github.com/yascherice/monero-multisig.git
cd monero-multisig
cargo build --release
```

The binary will be at `target/release/monero-multisig`.

## Usage

### 1. Create wallets (each participant)

```bash
# Participant A
monero-multisig create-wallet --threshold 2 --participants 3 --label "shared-fund"
# → outputs multisig info string A

# Participant B (on their machine)
monero-multisig create-wallet --threshold 2 --participants 3 --label "shared-fund"
# → outputs multisig info string B

# Participant C
monero-multisig create-wallet --threshold 2 --participants 3 --label "shared-fund"
# → outputs multisig info string C
```

### 2. Exchange keys

Each participant collects the info strings from all others and runs:

```bash
# Participant A exchanges with B and C's info
monero-multisig exchange-keys --info "<B_info>" "<C_info>"
```

For M > 2, multiple rounds are required — the tool will prompt you to share updated info strings after each round.

### 3. Synchronize balances

Before building a transaction, all participants must share partial key images:

```bash
# Each participant exports their info
monero-multisig export-info
# → share the output with all co-signers

# Each participant imports the others' info
monero-multisig import-info --info "<peer1_info>" "<peer2_info>"
```

### 4. Build and sign a transaction

```bash
# One participant builds the unsigned transaction
monero-multisig build-tx --address "4..." --amount 1000000000000 --priority 1
# → outputs multisig tx set data

# Each required co-signer signs it
monero-multisig sign-tx --tx-data "<tx_set_hex>"
# → outputs updated tx set with their signature applied
```

### 5. Submit

Once the threshold number of signatures is collected:

```bash
monero-multisig submit-tx --tx-data "<fully_signed_tx_hex>"
# → Transaction submitted! Hash: abc123...
```

### Configuration

Pass a JSON config file with `--config`:

```json
{
  "network": "stagenet",
  "daemon": {
    "host": "127.0.0.1",
    "port": 38081,
    "tls": false
  },
  "data_dir": "/home/user/.monero-multisig"
}
```

Or use CLI flags for quick overrides:

```bash
monero-multisig --daemon-host node.example.com --daemon-port 18081 create-wallet ...
```

## Multisig Protocol Overview

Monero multisig works through a multi-step protocol:

1. **Prepare** — Each participant generates their multisig key material
2. **Key Exchange** — Participants share info strings in one or more rounds:
   - 2-of-N: single `make_multisig` round
   - M-of-N (M > 2): (M − 1) rounds of `exchange_multisig_keys`
3. **Sync** — Before transacting, participants exchange partial key images so each wallet can compute the correct balance
4. **Sign** — A transaction is built by one party and passed to M co-signers
5. **Submit** — The fully signed transaction is broadcast to the network

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -am 'Add my feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a pull request

## License

MIT — see [LICENSE](LICENSE) for details.
