use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

pub fn run(args: &[&str]) -> Result<Value> {
    let output = Command::new("lark-cli")
        .args(args)
        .output()
        .with_context(|| format!("failed to run lark-cli {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "lark-cli failed (exit {}): {}",
            output.status,
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: String = stdout
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("tip:") && !t.starts_with("Found")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let body = body.trim();
    if body.is_empty() {
        return Ok(Value::Null);
    }

    let body = if body.starts_with("[page ") {
        body.lines().skip(1).collect::<Vec<_>>().join("\n")
    } else {
        body.to_string()
    };

    serde_json::from_str(&body).context("failed to parse lark-cli output as JSON")
}

#[allow(dead_code)]
pub fn run_raw(args: &[&str]) -> Result<String> {
    let output = Command::new("lark-cli")
        .args(args)
        .output()
        .with_context(|| format!("failed to run lark-cli {}", args.join(" ")))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
