use std::io::{self, Read, Write};
use std::path::Path;

/// Read a multisig data blob from a file or stdin.
pub fn read_multisig_data(path: Option<&Path>) -> anyhow::Result<String> {
    match path {
        Some(p) => {
            let data = std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", p.display()))?;
            Ok(data.trim().to_string())
        }
        None => {
            eprintln!("Reading multisig data from stdin (paste and press Ctrl+D)...");
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf.trim().to_string())
        }
    }
}

/// Write multisig data to a file or stdout.
pub fn write_multisig_data(path: Option<&Path>, data: &str) -> anyhow::Result<()> {
    match path {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, data)?;
            eprintln!("Wrote multisig data to {}", p.display());
        }
        None => {
            io::stdout().write_all(data.as_bytes())?;
            io::stdout().write_all(b"\n")?;
        }
    }
    Ok(())
}

/// Prompt the user for confirmation before a destructive action.
pub fn confirm(prompt: &str) -> bool {
    eprint!("{prompt} [y/N] ");
    io::stderr().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Abbreviate a hex string for display (first 8 + last 8 chars).
pub fn abbreviate_hex(hex: &str) -> String {
    if hex.len() <= 20 {
        hex.to_string()
    } else {
        format!("{}...{}", &hex[..8], &hex[hex.len() - 8..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abbreviate_short() {
        assert_eq!(abbreviate_hex("abcdef"), "abcdef");
    }

    #[test]
    fn test_abbreviate_long() {
        let long = "a]".repeat(20);
        let result = abbreviate_hex(&long);
        assert!(result.contains("..."));
        assert_eq!(result.len(), 8 + 3 + 8);
    }
}
