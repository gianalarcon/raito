//! Transaction formatting utilities for terminal display.
//!
//! Provides ASCII art visualization of Bitcoin transactions similar to block explorers.

use bitcoin::absolute::LockTime;
use bitcoin::{Address, Amount, Network, Transaction, TxIn, TxOut};

/// Configuration for transaction formatting
pub struct FormatConfig {
    /// Network to use for address generation
    pub network: Network,
    /// Show detailed information (currently unused but kept for future extensions)
    #[allow(dead_code)]
    pub verbose: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            network: Network::Bitcoin,
            verbose: false,
        }
    }
}

/// Format a Bitcoin transaction for terminal display
pub fn format_transaction(tx: &Transaction, config: &FormatConfig) -> String {
    let mut output = String::new();

    output.push_str("\n");

    // Header - make even wider to accommodate full TXID and longer addresses
    output.push_str("┌─ Bitcoin Transaction ───────────────────────────────────────────────────────────────────────────────────────────────────────────────┐\n");
    output.push_str(&format!(
        "│ \x1b[33mTXID:\x1b[0m {:<125} │\n",
        tx.compute_txid()
    ));
    output.push_str("├─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤\n");

    // Two-column layout: inputs on left, outputs on right
    let inputs_section = format_inputs(&tx.input, config);
    let outputs_section = format_outputs(&tx.output, config);

    // Split sections into lines for side-by-side display
    let input_lines: Vec<&str> = inputs_section.lines().collect();
    let output_lines: Vec<&str> = outputs_section.lines().collect();
    let max_lines = input_lines.len().max(output_lines.len());

    for i in 0..max_lines {
        let left = input_lines.get(i).unwrap_or(&"");
        let right = output_lines.get(i).unwrap_or(&"");

        // Handle line formatting with proper truncation and padding - make left column wider for full TXID
        let left_formatted = format_column_content(left, 64);
        let right_formatted = format_column_content(right, 64);

        output.push_str(&format!("│ {} │ {} │\n", left_formatted, right_formatted));
    }

    output.push_str("├─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤\n");

    // Details section
    let details = format_transaction_details(tx, config);
    for line in details.lines() {
        output.push_str(&format!("│ {:<131} │\n", line));
    }

    output.push_str("└─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘\n");

    output
}

/// Format transaction inputs
fn format_inputs(inputs: &[TxIn], config: &FormatConfig) -> String {
    let mut output = String::new();
    output.push_str("\x1b[33mINPUTS:\x1b[0m\n");

    for input in inputs.iter() {
        let address = format_input_address(input, config);
        output.push_str(&format!("{}\n\n", address));
    }

    if inputs.is_empty() {
        output.push_str("  (no inputs)\n");
    }

    output
}

/// Format transaction outputs
fn format_outputs(outputs: &[TxOut], config: &FormatConfig) -> String {
    let mut output = String::new();
    output.push_str("\x1b[33mOUTPUTS:\x1b[0m\n");

    for txout in outputs.iter() {
        let address = format_output_address(txout, config);
        let amount_btc = Amount::from_sat(txout.value.to_sat()).to_btc();

        output.push_str(&format!("{}        {:.8} BTC\n", address, amount_btc));

        // Add script with each opcode on separate line
        let script_asm = txout.script_pubkey.to_asm_string();
        if !script_asm.is_empty() {
            let opcodes: Vec<&str> = script_asm.split_whitespace().collect();
            for opcode in opcodes {
                output.push_str(&format!("\x1b[90m  {}\x1b[0m\n", opcode));
            }
            // Add padding between outputs
            output.push_str("\n");
        }
    }

    if outputs.is_empty() {
        output.push_str("  (no outputs)\n");
    }

    output
}

/// Format transaction details card
fn format_transaction_details(tx: &Transaction, _config: &FormatConfig) -> String {
    let mut output = String::new();
    output.push_str("\x1b[33mDETAILS:\x1b[0m\n");

    // Calculate total output value
    let total_output: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
    output.push_str(&format!(
        "Total Output: {:.8} BTC\n",
        Amount::from_sat(total_output).to_btc()
    ));

    // Format locktime if set
    if tx.lock_time != LockTime::ZERO {
        let locktime_desc = match tx.lock_time {
            LockTime::Blocks(height) => format!("Block height {}", height),
            LockTime::Seconds(timestamp) => {
                // Convert Unix timestamp to readable format
                format!(
                    "Unix timestamp {} ({})",
                    timestamp,
                    format_unix_timestamp(timestamp.to_consensus_u32())
                )
            }
        };
        output.push_str(&format!("Locktime: {}\n", locktime_desc));
    }

    // Additional details
    output.push_str(&format!("Version: {}\n", tx.version));
    output.push_str(&format!("Size: {} bytes\n", tx.total_size()));
    output.push_str(&format!("Weight: {} WU\n", tx.weight()));
    output.push_str(&format!("Virtual Size: {} vB\n", tx.vsize()));

    output
}

/// Get address string for a transaction input
fn format_input_address(input: &TxIn, _config: &FormatConfig) -> String {
    // For inputs, we can try to extract address from script_sig, but it's not always possible
    // In many cases, we'd need the previous transaction output to know the address
    if input.previous_output.is_null() {
        "Coinbase".to_string()
    } else {
        // Show the TXID on one line and output index on the next line
        format!(
            "{}\nvout = {}",
            input.previous_output.txid, input.previous_output.vout
        )
    }
}

/// Get address string for a transaction output
fn format_output_address(output: &TxOut, config: &FormatConfig) -> String {
    // Try to derive address from script_pubkey
    match Address::from_script(&output.script_pubkey, config.network) {
        Ok(address) => address.to_string(),
        Err(_) => {
            // If we can't parse as a standard address, show script type or raw script
            if output.script_pubkey.is_p2pk() {
                "P2PK".to_string()
            } else if output.script_pubkey.is_p2pkh() {
                "P2PKH".to_string()
            } else if output.script_pubkey.is_p2sh() {
                "P2SH".to_string()
            } else if output.script_pubkey.is_p2wpkh() {
                "P2WPKH".to_string()
            } else if output.script_pubkey.is_p2wsh() {
                "P2WSH".to_string()
            } else if output.script_pubkey.is_p2tr() {
                "P2TR".to_string()
            } else if output.script_pubkey.is_op_return() {
                "OP_RETURN".to_string()
            } else {
                "Unknown".to_string()
            }
        }
    }
}

/// Format content for a column with proper padding and truncation
fn format_column_content(content: &str, width: usize) -> String {
    // Remove ANSI color codes for length calculation
    let visible_content = strip_ansi_codes(content);
    let visible_len = visible_content.len();

    if visible_len <= width {
        // Content fits, pad with spaces
        let padding = width - visible_len;
        format!("{}{}", content, " ".repeat(padding))
    } else {
        // Content is too long, need to truncate
        if content.contains('\x1b') {
            // Content has ANSI codes, be careful about truncation
            let mut result = String::new();
            let mut current_visible_len = 0;
            let mut chars = content.chars();
            let target_len = width - 3; // Reserve space for "..."

            while current_visible_len < target_len {
                match chars.next() {
                    Some('\x1b') => {
                        // Copy ANSI escape sequence
                        result.push('\x1b');
                        while let Some(c) = chars.next() {
                            result.push(c);
                            if c == 'm' {
                                break;
                            }
                        }
                    }
                    Some(c) => {
                        result.push(c);
                        current_visible_len += 1;
                    }
                    None => break,
                }
            }

            // Add ellipsis and pad to exact width
            result.push_str("...");
            let final_visible_len = strip_ansi_codes(&result).len();
            if final_visible_len < width {
                result.push_str(&" ".repeat(width - final_visible_len));
            }
            result
        } else {
            // No ANSI codes, simple truncation
            let truncated = if content.len() > width - 3 {
                format!("{}...", &content[..width - 3])
            } else {
                content.to_string()
            };

            let final_len = truncated.len();
            if final_len < width {
                format!("{}{}", truncated, " ".repeat(width - final_len))
            } else {
                truncated
            }
        }
    }
}

/// Remove ANSI color codes from a string for length calculation
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip the ANSI escape sequence
            while let Some(next_c) = chars.next() {
                if next_c == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Format Unix timestamp to human-readable string
fn format_unix_timestamp(timestamp: u32) -> String {
    // Simple approximation - in a real implementation you'd use a proper datetime library
    use std::time::UNIX_EPOCH;

    let system_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
    match system_time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let days = duration.as_secs() / 86400;
            let years_since_epoch = days / 365;
            format!("~{} years after 1970", years_since_epoch)
        }
        Err(_) => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{Amount, ScriptBuf};

    #[test]
    fn test_format_transaction() {
        // Create a simple test transaction
        let tx = Transaction {
            version: bitcoin::transaction::Version(1),
            lock_time: LockTime::ZERO,
            input: vec![],
            output: vec![TxOut {
                value: Amount::from_btc(0.1).unwrap(),
                script_pubkey: ScriptBuf::new(),
            }],
        };

        let config = FormatConfig::default();
        let formatted = format_transaction(&tx, &config);

        assert!(formatted.contains("Bitcoin Transaction"));
        assert!(formatted.contains("OUTPUTS"));
        assert!(formatted.contains("0.10000000 BTC"));
    }

    #[test]
    fn test_format_transaction_display() {
        use bitcoin::OutPoint;

        // Create a more realistic test transaction with inputs and outputs
        let tx = Transaction {
            version: bitcoin::transaction::Version(2),
            lock_time: LockTime::from_height(800000).unwrap(),
            input: vec![TxIn {
                previous_output: OutPoint::new(
                    "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                        .parse()
                        .unwrap(),
                    0,
                ),
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::transaction::Sequence(0xfffffffd),
                witness: bitcoin::Witness::new(),
            }],
            output: vec![
                TxOut {
                    value: Amount::from_btc(0.5).unwrap(),
                    script_pubkey: ScriptBuf::new(),
                },
                TxOut {
                    value: Amount::from_btc(0.25).unwrap(),
                    script_pubkey: ScriptBuf::new(),
                },
            ],
        };

        let config = FormatConfig::default();
        let formatted = format_transaction(&tx, &config);

        // Print the formatted transaction to see how it looks
        println!("\n{}", formatted);

        assert!(formatted.contains("Bitcoin Transaction"));
        assert!(formatted.contains("INPUTS"));
        assert!(formatted.contains("OUTPUTS"));
        assert!(formatted.contains("0.50000000 BTC"));
        assert!(formatted.contains("0.25000000 BTC"));
        assert!(formatted.contains("Locktime"));
    }
}
