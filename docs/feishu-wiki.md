# feishu-wiki — 飞书知识库管理工具

纯 Rust 重写，通过 `lark-cli` 子进程调用飞书 API。

## 构建

```bash
cd apps/feishu-wiki
cargo build --release
```

## 全局用法

```
feishu-wiki <COMMAND> [OPTIONS]
```

## 命令

### 1. download — 下载知识库

递归下载知识库全部节点，按原始目录结构保存为 markdown。

```bash
# 下载默认知识库（量潮数据档案）
feishu-wiki download

# 指定知识库名称
feishu-wiki download "工作章程"

# 指定知识库名称和输出目录
feishu-wiki download "量潮数据档案" output/
```

输出：`<output>/nodes.json` + 目录树下的 `.md` 文件。

### 2. judge — 审阅文档

四种模式：

```bash
# interactive — 逐篇判断
feishu-wiki judge --tree tree.json --dir output/

# batch — 从 answers.json 批量判断
feishu-wiki judge --tree tree.json --dir output/ --batch answers.json

# preview — 只浏览不记录
feishu-wiki judge --tree tree.json --dir output/ --preview

# resume — 从已有 manifest 继续
feishu-wiki judge --tree tree.json --dir output/ --resume
```

参数：
- `--tree` — 知识库树 JSON
- `--dir` — 文档下载目录
- `--manifest` — 输出路径（默认: judge_manifest.json）
- `--batch` — answers.json 路径
- `--preview` — 预览模式
- `--resume` — 续传

Batch 格式 (`answers.json`):

```json
[
  {"path": "量潮科技章程", "judgment": "keep"},
  {"path": "量潮科技章程/公司代表大会制度", "judgment": "delete", "reason": "内容过时"}
]
```

### 3. delete — 删除飞书源文档

根据 manifest 删除已判断的文档。

```bash
# 预览（不实际删除）
feishu-wiki delete judge_manifest.json --tree tree.json --dry-run

# 执行删除
feishu-wiki delete judge_manifest.json --tree tree.json

# 只删除 keep 类型的（默认: keep,delete）
feishu-wiki delete judge_manifest.json --tree tree.json --filter keep
```

参数：
- `--tree` — 知识库树 JSON
- `--dry-run` — 预览模式
- `--filter` — 判断类型过滤（默认: keep,delete）
- `--result` — 结果输出（默认: delete_result.json）

### 4. validate — 验证资产

根据 `contract.yaml` 契约，对已下载的知识库执行声明式验证。

支持两种验证模式：

| 模式 | 说明 |
|------|------|
| `ATOMIC` | 空间必须包含指定的必需分类（如"议题/议程/决策/计划"） |
| `SCOPED` | 空间存在即可 |

```bash
# 使用默认契约文件
feishu-wiki validate --tree output/nodes.json

# 指定自定义契约
feishu-wiki validate --tree output/nodes.json --contract my-contract.yaml
```

输出：控制台报告 + `validate_report.json`

### 5. 契约文件 (contract.yaml)

验证策略定义在 YAML 文件中：

```yaml
spec_version: 0.0.4
validation:
  policies:
    - selector: "议事档案"
      mode: ATOMIC
      required_categories:
        - 议题
        - 议程
        - 决策
        - 计划
    - selector: "**"
      mode: SCOPED
```

## 完整工作流

```bash
# 1. 下载知识库
feishu-wiki download "知识库名称" output/

# 2. 验证资产结构
feishu-wiki validate --tree output/nodes.json

# 3. 审阅文档
feishu-wiki judge --tree output/nodes.json --dir output/

# 4. 删除飞书源文档
feishu-wiki delete judge_manifest.json --tree output/nodes.json
```

## 参考文档

实验资料已迁至归档站：[quanttide-archive-of-asset-management/quanttide-asset-lab](https://github.com/quanttide/quanttide-archive-of-asset-management/tree/main/quanttide-asset-lab)
