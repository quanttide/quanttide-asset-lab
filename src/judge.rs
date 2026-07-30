use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    #[arg(long, required = true)]
    tree: String,
    #[arg(long, required = true)]
    dir: String,
    #[arg(long, default_value = "judge_manifest.json")]
    manifest: String,
    #[arg(long)]
    resume: bool,
    #[arg(long)]
    batch: Option<String>,
    #[arg(long)]
    preview: bool,
}

#[derive(Deserialize, Serialize, Clone)]
struct ManifestEntry {
    title: String,
    obj_token: String,
    node_token: String,
    has_child: bool,
    path: String,
    space_id: String,
    judgment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Deserialize)]
struct BatchAnswer {
    path: String,
    judgment: String,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
struct TreeNode {
    title: String,
    #[serde(default)]
    obj_token: String,
    #[serde(default)]
    node_token: String,
    #[serde(default)]
    has_child: bool,
    #[serde(default)]
    children: Vec<TreeNode>,
}

#[derive(Deserialize)]
struct TreeData {
    space_id: String,
    nodes: Vec<TreeNode>,
}

#[derive(Deserialize, Serialize)]
struct FlatNode {
    title: String,
    obj_token: String,
    node_token: String,
    has_child: bool,
    path: String,
}

pub fn run(args: Args) -> Result<()> {
    let tree_data: TreeData = {
        let content = fs::read_to_string(&args.tree)?;
        serde_json::from_str(&content)?
    };

    let flat_nodes = collect_nodes(&tree_data.nodes, "");
    println!(
        "知识库: {}  文档数: {}",
        tree_data.space_id,
        flat_nodes.len()
    );

    if args.preview {
        preview(&flat_nodes, &args.dir, &tree_data.space_id)
    } else if let Some(batch_path) = &args.batch {
        batch(
            &flat_nodes,
            &args.dir,
            &tree_data.space_id,
            &args.manifest,
            batch_path,
        )
    } else {
        interactive(
            &flat_nodes,
            &args.dir,
            &tree_data.space_id,
            &args.manifest,
            args.resume,
        )
    }
}

fn collect_nodes(nodes: &[TreeNode], parent_path: &str) -> Vec<FlatNode> {
    let mut result = Vec::new();
    for n in nodes {
        let safe = n.title.replace('/', "／");
        let cur = if parent_path.is_empty() {
            safe.clone()
        } else {
            format!("{}/{}", parent_path, safe)
        };
        result.push(FlatNode {
            title: n.title.clone(),
            obj_token: n.obj_token.clone(),
            node_token: n.node_token.clone(),
            has_child: n.has_child || !n.children.is_empty(),
            path: cur.clone(),
        });
        if !n.children.is_empty() {
            result.extend(collect_nodes(&n.children, &cur));
        }
    }
    result
}

fn file_path(base_dir: &str, item: &FlatNode) -> PathBuf {
    let p = PathBuf::from(base_dir);
    if item.has_child {
        p.join(&item.path).join("README.md")
    } else {
        p.join(format!("{}.md", &item.path))
    }
}

fn is_empty(content: &str) -> bool {
    content.lines().filter(|l| !l.trim().is_empty()).count() <= 2
}

fn show_content(content: &str) {
    let lines: Vec<&str> = content.lines().collect();
    let step = 40;
    let mut off = 0;
    while off < lines.len() {
        let end = std::cmp::min(off + step, lines.len());
        println!("{}", lines[off..end].join("\n"));
        off += step;
        if off < lines.len() {
            print!("-- 回车继续 ({}/{}) 行, q 退出 -- ", off, lines.len());
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            if input.trim().eq_ignore_ascii_case("q") {
                return;
            }
        }
    }
}

fn make_doc_url(node_token: &str) -> String {
    format!("https://bytedance.feishu.cn/wiki/{}", node_token)
}

fn preview(nodes: &[FlatNode], base_dir: &str, _space_id: &str) -> Result<()> {
    for (i, item) in nodes.iter().enumerate() {
        let fp = file_path(base_dir, item);
        if !fp.exists() {
            println!("[{}/{}] [{}] 文件不存在", i + 1, nodes.len(), item.path);
            continue;
        }
        let content = fs::read_to_string(&fp)?;
        if is_empty(&content) {
            println!("[{}/{}] [{}] 空文档", i + 1, nodes.len(), item.path);
            continue;
        }
        println!("\n{}", "=".repeat(60));
        println!("[{}/{}] {}", i + 1, nodes.len(), item.path);
        println!("    文件: {}", fp.display());
        println!("    原文: {}", make_doc_url(&item.node_token));
        println!("{}", "=".repeat(60));
        show_content(&content);
        print!("-- 按回车继续下一篇 -- ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
    }
    println!("\n完成，共浏览 {} 篇。", nodes.len());
    Ok(())
}

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_json<T: serde::Serialize>(obj: &T, path: &str) -> Result<()> {
    let tmp = format!("{}.tmp", path);
    let content = serde_json::to_string_pretty(obj)?;
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn print_summary(manifest: &[ManifestEntry]) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for m in manifest {
        *counts.entry(&m.judgment).or_insert(0) += 1;
    }
    println!("    汇总: {:?}", counts);
}

fn process_item(
    item: &FlatNode,
    space_id: &str,
    base_dir: &str,
    show: bool,
) -> Result<Option<ManifestEntry>> {
    let fp = file_path(base_dir, item);

    if !fp.exists() {
        println!("[{}] 文件不存在，跳过", item.path);
        return Ok(Some(ManifestEntry {
            title: item.title.clone(),
            obj_token: item.obj_token.clone(),
            node_token: item.node_token.clone(),
            has_child: item.has_child,
            path: item.path.clone(),
            space_id: space_id.to_string(),
            judgment: "skip".to_string(),
            reason: Some("not_found".to_string()),
        }));
    }

    let content = fs::read_to_string(&fp)?;
    if is_empty(&content) {
        println!("[{}] 空文档，跳过", item.path);
        return Ok(Some(ManifestEntry {
            title: item.title.clone(),
            obj_token: item.obj_token.clone(),
            node_token: item.node_token.clone(),
            has_child: item.has_child,
            path: item.path.clone(),
            space_id: space_id.to_string(),
            judgment: "skip".to_string(),
            reason: Some("empty".to_string()),
        }));
    }

    if show {
        println!("\n{}", "=".repeat(60));
        println!("{}", item.path);
        println!("    文件: {}", fp.display());
        println!("    原文: {}", make_doc_url(&item.node_token));
        println!("{}", "=".repeat(60));
        show_content(&content);
    }

    Ok(None)
}

fn interactive(
    nodes: &[FlatNode],
    base_dir: &str,
    space_id: &str,
    manifest_path: &str,
    resume: bool,
) -> Result<()> {
    let mut manifest: Vec<ManifestEntry> = if resume {
        load_json(manifest_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    let judged_paths: HashSet<String> = manifest.iter().map(|m| m.path.clone()).collect();

    for (_i, item) in nodes.iter().enumerate() {
        if judged_paths.contains(&item.path) {
            continue;
        }

        let result = process_item(item, space_id, base_dir, true)?;
        if let Some(entry) = result {
            manifest.push(entry);
            save_json(&manifest, manifest_path)?;
            continue;
        }

        loop {
            print!("? 如何处理？(keep/delete/skip) [keep]: ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();

            let (judgment, _ok) = match input.trim() {
                "" | "keep" => ("keep", true),
                "delete" => {
                    print!("  确认标记删除？(yes): ");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    let mut confirm = String::new();
                    std::io::stdin().read_line(&mut confirm).ok();
                    if confirm.trim() == "yes" {
                        ("delete", true)
                    } else {
                        println!("  取消");
                        continue;
                    }
                }
                "skip" => ("skip", true),
                _ => {
                    println!("! 选项: keep / delete / skip");
                    continue;
                }
            };

            manifest.push(ManifestEntry {
                title: item.title.clone(),
                obj_token: item.obj_token.clone(),
                node_token: item.node_token.clone(),
                has_child: item.has_child,
                path: item.path.clone(),
                space_id: space_id.to_string(),
                judgment: judgment.to_string(),
                reason: None,
            });
            break;
        }

        save_json(&manifest, manifest_path)?;
        println!("  ✓ 已记录（共 {} 条）", manifest.len());
    }

    println!("\n完成，共 {} 条判断 {}", manifest.len(), manifest_path);
    print_summary(&manifest);
    Ok(())
}

fn batch(
    nodes: &[FlatNode],
    base_dir: &str,
    space_id: &str,
    manifest_path: &str,
    answers_path: &str,
) -> Result<()> {
    let answers: Vec<BatchAnswer> = load_json(answers_path)?;
    let ans_by_path: HashMap<&str, &BatchAnswer> =
        answers.iter().map(|a| (a.path.as_str(), a)).collect();
    let mut manifest: Vec<ManifestEntry> = Vec::new();

    for (i, item) in nodes.iter().enumerate() {
        let (judgment, reason) = match ans_by_path.get(item.path.as_str()) {
            Some(ans) if ["keep", "delete", "skip"].contains(&ans.judgment.as_str()) => {
                (ans.judgment.clone(), ans.reason.clone())
            }
            Some(ans) => {
                println!(
                    "[{}/{}] [{}] 无效判断: {}，跳过",
                    i + 1,
                    nodes.len(),
                    item.path,
                    ans.judgment
                );
                ("skip".to_string(), None)
            }
            None => {
                println!(
                    "[{}/{}] [{}] 未在 batch 中指定，跳过",
                    i + 1,
                    nodes.len(),
                    item.path
                );
                ("skip".to_string(), None)
            }
        };

        let result = process_item(item, space_id, base_dir, false)?;
        if let Some(entry) = result {
            manifest.push(entry);
        } else {
            manifest.push(ManifestEntry {
                title: item.title.clone(),
                obj_token: item.obj_token.clone(),
                node_token: item.node_token.clone(),
                has_child: item.has_child,
                path: item.path.clone(),
                space_id: space_id.to_string(),
                judgment,
                reason,
            });
        }
        save_json(&manifest, manifest_path)?;
    }

    println!("\nBatch 完成，共 {} 条 {}", manifest.len(), manifest_path);
    print_summary(&manifest);
    Ok(())
}
