use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("failed to parse config: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("missing required field: {0}")]
    MissingField(String),
}

/// Connection settings for a Monero daemon RPC endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonRpc {
    /// Hostname or IP address of the Monero daemon.
    pub host: String,
    /// RPC port (default: 18081 for mainnet, 28081 for testnet).
    pub port: u16,
    /// Use TLS for the RPC connection.
    pub tls: bool,
    /// Optional username for digest authentication.
    pub username: Option<String>,
    /// Optional password for digest authentication.
    pub password: Option<String>,
}

impl Default for DaemonRpc {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 18081,
            tls: false,
            username: None,
            password: None,
        }
    }
}

impl DaemonRpc {
    /// Build the full RPC URL from the connection settings.
    pub fn url(&self) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        format!("{scheme}://{}:{}/json_rpc", self.host, self.port)
    }
}

/// Top-level configuration for the multisig wallet tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Monero network to operate on.
    pub network: Network,
    /// Daemon RPC connection settings.
    pub daemon: DaemonRpc,
    /// Directory for storing wallet files and key exchange data.
    pub data_dir: PathBuf,
}

/// The Monero network variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Stagenet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Stagenet => write!(f, "stagenet"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("monero-multisig");

        Self {
            network: Network::Mainnet,
            daemon: DaemonRpc::default(),
            data_dir,
        }
    }
}

impl Config {
    /// Load configuration from a JSON file, falling back to defaults.
    pub fn load(path: Option<&PathBuf>) -> Result<Self, ConfigError> {
        match path {
            Some(p) => {
                let contents = std::fs::read_to_string(p)?;
                let config: Config = serde_json::from_str(&contents)?;
                Ok(config)
            }
            None => Ok(Self::default()),
        }
    }

    /// Persist the current configuration to a JSON file.
    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// A lightweight JSON-RPC client for communicating with the Monero daemon.
#[derive(Debug, Clone)]
pub struct RpcClient {
    client: reqwest::Client,
    url: String,
}

impl RpcClient {
    /// Create a new RPC client from daemon connection settings.
    pub fn new(daemon: &DaemonRpc) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: daemon.url(),
        }
    }

    /// Send a JSON-RPC request and deserialize the result.
    pub async fn request<P, R>(&self, method: &str, params: &P) -> anyhow::Result<R>
    where
        P: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "0",
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let rpc_response: JsonRpcResponse<R> = response.json().await?;

        match rpc_response.result {
            Some(result) => Ok(result),
            None => {
                let err = rpc_response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "unknown RPC error".to_string());
                Err(anyhow::anyhow!("RPC error: {err}"))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}
