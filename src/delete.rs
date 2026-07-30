use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Duration;

#[derive(clap::Args)]
pub struct Args {
    manifest: String,
    #[arg(long, required = true)]
    tree: String,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, default_value = "keep,delete")]
    filter: String,
    #[arg(long, default_value = "delete_result.json")]
    result: String,
}

#[derive(Deserialize)]
struct ManifestEntry {
    path: Option<String>,
    title: Option<String>,
    judgment: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TreeNode {
    title: String,
    #[serde(default)]
    obj_token: String,
    #[serde(default)]
    node_token: String,
    #[serde(default)]
    children: Vec<TreeNode>,
}

#[derive(Deserialize)]
struct TreeData {
    space_id: Option<String>,
    nodes: Vec<TreeNode>,
}

#[derive(Serialize)]
struct DeleteResult {
    total: usize,
    results: Vec<DeleteItem>,
}

#[derive(Serialize)]
struct DeleteItem {
    path: String,
    judgment: String,
    space_id: String,
    node_token: String,
    success: bool,
    error: String,
}

struct TokenInfo {
    space_id: String,
    node_token: String,
}

pub fn run(args: Args) -> Result<()> {
    let manifest: Vec<ManifestEntry> = {
        let content = fs::read_to_string(&args.manifest)?;
        serde_json::from_str(&content)?
    };

    let tree: TreeData = {
        let content = fs::read_to_string(&args.tree)?;
        serde_json::from_str(&content)?
    };

    let default_space_id = tree.space_id.as_deref().unwrap_or("");
    let lookup = build_lookup(&tree.nodes, default_space_id);

    let allowed: HashSet<String> = args
        .filter
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let mut to_delete: Vec<DeleteItem> = Vec::new();

    for item in &manifest {
        let judgment = item.judgment.as_deref().unwrap_or("");
        if !allowed.contains(judgment) {
            continue;
        }
        let path = item.path.as_deref().or(item.title.as_deref()).unwrap_or("");
        if path.is_empty() {
            continue;
        }
        match lookup.get(path) {
            Some(info) => {
                to_delete.push(DeleteItem {
                    path: path.to_string(),
                    judgment: judgment.to_string(),
                    space_id: info.space_id.clone(),
                    node_token: info.node_token.clone(),
                    success: false,
                    error: String::new(),
                });
            }
            None => {
                println!("无法查找到 [{}] 的 token 信息", path);
            }
        }
    }

    if to_delete.is_empty() {
        println!("没有匹配的文档。");
        return Ok(());
    }

    println!("共 {} 篇文档待删除:\n", to_delete.len());
    for item in &to_delete {
        println!("  [{}] {}", item.judgment, item.path);
        println!("        空间: {}  节点: {}", item.space_id, item.node_token);
    }

    if args.dry_run {
        println!("\n(dry-run，未实际删除)");
        return Ok(());
    }

    print!("\n确认删除 {} 篇？(输入 yes): ", to_delete.len());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm).ok();
    if confirm.trim() != "yes" {
        println!("取消。");
        return Ok(());
    }

    let total = to_delete.len();
    for (i, item) in to_delete.iter_mut().enumerate() {
        print!("[{}/{}] 删除 {} ...", i + 1, total, item.path);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut ok = false;
        let mut msg = String::new();
        for _ in 0..3 {
            match delete_node(&item.space_id, &item.node_token) {
                Ok(()) => {
                    ok = true;
                    break;
                }
                Err(e) => {
                    msg = format!("{}", e);
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }

        item.success = ok;
        item.error = if ok { String::new() } else { msg };
        println!(" {}", if ok { "OK" } else { "FAIL" });
    }

    let result = DeleteResult {
        total: to_delete.len(),
        results: to_delete,
    };

    let tmp = format!("{}.tmp", args.result);
    fs::write(&tmp, serde_json::to_string_pretty(&result)?)?;
    fs::rename(&tmp, &args.result)?;

    let ok_n = result.results.iter().filter(|r| r.success).count();
    println!(
        "\n{} 成功, {} 失败 -> {}",
        ok_n,
        result.results.len() - ok_n,
        args.result
    );
    Ok(())
}

fn build_lookup(nodes: &[TreeNode], space_id: &str) -> HashMap<String, TokenInfo> {
    let mut lookup = HashMap::new();

    fn walk(
        nodes: &[TreeNode],
        parent: &str,
        space_id: &str,
        lookup: &mut HashMap<String, TokenInfo>,
    ) {
        for n in nodes {
            let safe = n.title.replace('/', "／");
            let cur = if parent.is_empty() {
                safe.clone()
            } else {
                format!("{}/{}", parent, safe)
            };
            lookup.insert(
                cur.clone(),
                TokenInfo {
                    space_id: space_id.to_string(),
                    node_token: n.node_token.clone(),
                },
            );
            if !n.children.is_empty() {
                walk(&n.children, &cur, space_id, lookup);
            }
        }
    }

    walk(nodes, "", space_id, &mut lookup);
    lookup
}

fn delete_node(space_id: &str, node_token: &str) -> Result<()> {
    let _ = crate::lark::run(&[
        "api",
        "DELETE",
        &format!(
            "/open-apis/wiki/v2/spaces/{}/nodes/{}",
            space_id, node_token
        ),
    ])?;
    Ok(())
}
