use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(clap::Args)]
pub struct Args {
    /// 知识库节点树 JSON（由 download 生成）
    #[arg(long, default_value = "nodes.json")]
    tree: String,
    /// 契约文件路径
    #[arg(long, default_value = "contract.yaml")]
    contract: String,
}

// ── 契约结构 ──

#[derive(Debug, Deserialize)]
struct Contract {
    #[serde(default)]
    spec_version: String,
    #[serde(default)]
    validation: ValidationConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ValidationConfig {
    policies: Vec<Policy>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self { policies: Vec::new() }
    }
}

#[derive(Debug, Deserialize)]
struct Policy {
    selector: String,
    mode: String,
    #[serde(default)]
    required_categories: Vec<String>,
}

// ── 节点树结构（与 download 输出一致） ──

#[derive(Debug, Deserialize, Serialize)]
struct TreeNode {
    title: String,
    #[serde(default)]
    obj_token: String,
    #[serde(default)]
    node_token: String,
    #[serde(default)]
    has_child: bool,
    #[serde(default)]
    parent_node_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TreeData {
    #[serde(default)]
    space_id: String,
    nodes: Vec<TreeNode>,
}

// ── 验证报告结构 ──

#[derive(Debug, Serialize)]
struct ValidationReport {
    space_id: String,
    validated_at: String,
    total_policies: usize,
    results: Vec<PolicyResult>,
    summary: Summary,
}

#[derive(Debug, Serialize)]
struct PolicyResult {
    selector: String,
    mode: String,
    status: String, // "pass" | "fail"
    details: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
}

// ── 验证逻辑 ──

fn match_selector(space_name: &str, selector: &str) -> bool {
    if selector == "**" {
        return true;
    }
    if selector.ends_with("/**") {
        let prefix = selector.trim_end_matches("/**");
        return space_name.starts_with(prefix);
    }
    space_name == selector
}

fn validate_policy(nodes: &[TreeNode], policy: &Policy) -> PolicyResult {
    let mut details = Vec::new();
    let node_titles: Vec<&str> = nodes.iter().map(|n| n.title.as_str()).collect();

    if policy.mode == "ATOMIC" {
        let mut all_pass = true;
        for cat in &policy.required_categories {
            if node_titles.contains(&cat.as_str()) {
                details.push(format!("  ✅ 必需分类存在: {}", cat));
            } else {
                details.push(format!("  ❌ 必需分类缺失: {}", cat));
                all_pass = false;
            }
        }
        let status = if all_pass { "pass" } else { "fail" };
        PolicyResult {
            selector: policy.selector.clone(),
            mode: policy.mode.clone(),
            status: status.to_string(),
            details,
        }
    } else if policy.mode == "SCOPED" {
        details.push("  ✅ 空间存在".to_string());
        PolicyResult {
            selector: policy.selector.clone(),
            mode: policy.mode.clone(),
            status: "pass".to_string(),
            details,
        }
    } else {
        details.push(format!("  ⚠️ 未知验证模式: {}", policy.mode));
        PolicyResult {
            selector: policy.selector.clone(),
            mode: policy.mode.clone(),
            status: "skip".to_string(),
            details,
        }
    }
}

// ── 入口 ──

pub fn run(args: Args) -> Result<()> {
    // 1. 读取契约
    let contract: Contract = {
        let content = fs::read_to_string(&args.contract)?;
        serde_yaml::from_str(&content)?
    };

    // 2. 读取节点树
    let tree: TreeData = {
        let content = fs::read_to_string(&args.tree)?;
        serde_json::from_str(&content)?
    };

    let policies = &contract.validation.policies;
    if policies.is_empty() {
        println!("⚠️  契约文件中未定义验证策略");
        return Ok(());
    }

    println!("=== 资产验证报告 ===\n");
    println!("契约版本: {}", contract.spec_version);
    println!("知识库: {} (space_id: {})", tree.nodes.first().map(|n| n.title.as_str()).unwrap_or("?"), tree.space_id);
    println!("节点数: {}", tree.nodes.len());
    println!("验证策略数: {}\n", policies.len());

    let mut all_results = Vec::new();
    let mut total_pass = 0usize;
    let mut total_fail = 0usize;

    for policy in policies {
        if !match_selector(&tree.space_id, &policy.selector) {
            continue;
        }
        let result = validate_policy(&tree.nodes, policy);
        if result.status == "pass" {
            total_pass += 1;
        } else if result.status == "fail" {
            total_fail += 1;
        }
        println!("策略: [{}] selector=\"{}\"", result.mode, result.selector);
        for d in &result.details {
            println!("{}", d);
        }
        println!();
        all_results.push(result);

        // selector 首位命中即停止（与旧版行为一致）
        break;
    }

    println!("{}", "-".repeat(50));
    println!("\n=== 统计 ===");
    println!("通过: {}", total_pass);
    println!("失败: {}", total_fail);
    println!("总计: {}", total_pass + total_fail);

    if total_fail == 0 {
        println!("\n✅ 所有验证通过！");
    } else {
        println!("\n⚠️  有 {} 项验证失败", total_fail);
    }

    // 输出机器可读报告
    let report_path = "validate_report.json";
    let report = ValidationReport {
        space_id: tree.space_id,
        validated_at: chrono_now(),
        total_policies: policies.len(),
        results: all_results,
        summary: Summary {
            total: total_pass + total_fail,
            passed: total_pass,
            failed: total_fail,
        },
    };
    let tmp = format!("{}.tmp", report_path);
    fs::write(&tmp, serde_json::to_string_pretty(&report)?)?;
    fs::rename(&tmp, report_path)?;
    println!("\n报告已输出到 {}", report_path);

    Ok(())
}

fn chrono_now() -> String {
    // 不使用 chrono crate，用简单时间戳
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    // 粗略计算日期字符串（只做展示用）
    let secs = d.as_secs();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    // 从 Unix epoch (1970-01-01) 推算年份
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1u32;
    let mut day_rem = remaining;
    for &md in &month_days {
        if day_rem < md {
            break;
        }
        day_rem -= md;
        mo += 1;
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        mo,
        day_rem + 1,
        h,
        m,
        s
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
