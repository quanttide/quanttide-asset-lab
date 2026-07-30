use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    name: String,
    #[arg(default_value = "")]
    output_dir: Option<String>,
}

pub fn run(args: Args) -> Result<()> {
    let name = &args.name;
    let out_dir = match &args.output_dir {
        Some(d) if !d.is_empty() => PathBuf::from(d).join(name),
        _ => PathBuf::from(name),
    };

    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create output dir {:?}", out_dir))?;

    let space_id = get_space_id(name)?;
    println!("知识库: {} (space_id: {})", name, space_id);

    println!("正在递归获取全部节点...");
    let nodes = get_all_nodes(&space_id, None)?;
    println!("共 {} 个节点", nodes.len());

    fs::write(
        out_dir.join("nodes.json"),
        serde_json::to_string_pretty(&nodes)?,
    )?;

    let by_token: HashMap<&str, &Value> = nodes
        .iter()
        .filter_map(|n| n.get("node_token").and_then(|t| t.as_str()).map(|t| (t, n)))
        .collect();

    let mut downloaded = 0usize;
    for (i, node) in nodes.iter().enumerate() {
        let title = node
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled");
        let obj_token = node
            .get("obj_token")
            .or_else(|| node.get("node_token"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if obj_token.is_empty() {
            continue;
        }

        let dir = dir_path(node, &by_token, &out_dir);
        let outpath = dir.join(format!("{}.md", safe_name(title)));

        match download_doc(obj_token, &outpath) {
            Ok(()) => {
                if outpath.exists() {
                    downloaded += 1;
                }
                println!("  [{}/{}] {}", i + 1, nodes.len(), title);
            }
            Err(e) => println!("  [{}/{}] {} (失败: {})", i + 1, nodes.len(), title, e),
        }
    }

    println!("\n完成。{} 个节点下载至 {:?}/", downloaded, out_dir);
    Ok(())
}

fn get_space_id(name: &str) -> Result<String> {
    let data = crate::lark::run(&["wiki", "+space-list"])?;
    let spaces = data
        .pointer("/data/spaces")
        .and_then(|v| v.as_array())
        .context("unexpected response: /data/spaces not found")?;

    for space in spaces {
        if space.get("name").and_then(|v| v.as_str()) == Some(name) {
            return space
                .get("space_id")
                .and_then(|v| v.as_str().map(String::from))
                .context("space_id missing");
        }
    }

    anyhow::bail!("未找到知识库: {}", name);
}

fn list_nodes(space_id: &str, parent_token: Option<&str>) -> Result<Vec<Value>> {
    let mut args = vec!["wiki", "+node-list", "--space-id", space_id];
    if let Some(pt) = parent_token {
        args.extend(["--parent-node-token", pt]);
    }
    let data = crate::lark::run(&args)?;
    let items = data
        .pointer("/data/items")
        .or_else(|| data.pointer("/data/nodes"))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    Ok(items)
}

fn get_all_nodes(space_id: &str, parent_token: Option<&str>) -> Result<Vec<Value>> {
    let nodes = list_nodes(space_id, parent_token)?;
    let mut all = Vec::new();
    for node in &nodes {
        all.push(node.clone());
        let nt = node
            .get("node_token")
            .or_else(|| node.get("obj_token"))
            .and_then(|v| v.as_str());
        if let Some(nt) = nt {
            all.extend(get_all_nodes(space_id, Some(nt))?);
        }
    }
    Ok(all)
}

fn safe_name(title: &str) -> String {
    title
        .replace('/', "-")
        .replace('\\', "-")
        .replace(':', "：")
}

fn dir_path(node: &Value, by_token: &HashMap<&str, &Value>, base: &Path) -> PathBuf {
    let title = node
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("untitled");
    let parent = node.get("parent_node_token").and_then(|v| v.as_str());

    if let Some(pt) = parent {
        if let Some(parent_node) = by_token.get(pt) {
            let mut p = dir_path(parent_node, by_token, base);
            p.push(safe_name(title));
            return p;
        }
    }

    base.join(safe_name(title))
}

fn download_doc(obj_token: &str, outpath: &Path) -> Result<()> {
    if outpath.exists() {
        return Ok(());
    }
    if let Some(parent) = outpath.parent() {
        fs::create_dir_all(parent)?;
    }

    let max_retries = 5;
    for attempt in 0..max_retries {
        match try_fetch_doc(obj_token, outpath) {
            Ok(true) => return Ok(()),
            Ok(false) => return Ok(()),
            Err(e) => {
                let err_str = format!("{}", e);
                if attempt + 1 < max_retries && should_retry(&err_str) {
                    let delay = 2u64.pow(attempt as u32) * 2;
                    println!(
                        "    限流，{}s 后重试 ({}/{})",
                        delay,
                        attempt + 1,
                        max_retries
                    );
                    std::thread::sleep(std::time::Duration::from_secs(delay));
                    continue;
                }
                if is_unsupported_type(&err_str) {
                    fs::write(
                        outpath,
                        format!("> 文档类型无法转换为 markdown。\n> 错误: {}\n", err_str),
                    )?;
                    return Ok(());
                }
                return Err(e);
            }
        }
    }
    Ok(())
}

fn should_retry(err: &str) -> bool {
    err.contains("rate_limit") || err.contains("99991400") || err.contains("retryable")
}

fn is_unsupported_type(err: &str) -> bool {
    err.contains("Unsupported document type")
}

fn try_fetch_doc(obj_token: &str, outpath: &Path) -> Result<bool> {
    let output = std::process::Command::new("lark-cli")
        .args([
            "docs",
            "+fetch",
            "--doc",
            obj_token,
            "--doc-format",
            "markdown",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined = format!("{}\n{}", stdout, stderr);

    if !output.status.success() {
        anyhow::bail!("{}", combined.trim());
    }

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
        return Ok(false);
    }

    let body = if body.starts_with("[page ") {
        body.lines().skip(1).collect::<Vec<_>>().join("\n")
    } else {
        body.to_string()
    };

    let data: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };

    let content = data
        .pointer("/data/document/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if content.is_empty() {
        return Ok(false);
    }

    fs::write(outpath, content)?;
    Ok(true)
}
