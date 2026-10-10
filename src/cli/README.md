# 第二大脑创建 CLI

一个面向量潮资产章程的 Git 工作流执行器，按 plan、approve、apply、verify 四步完成领域第二大脑的创建与维护。全部 Git 调用走 `subprocess` 参数数组，不求值 shell 字符串；没有人工确认不执行远程写操作。

本目录是一个独立的 Python 项目，按标准结构组织：可执行代码在 `src/`，给人读的文档在 `docs/`，程序读的规格在 `assets/`。

## 目录结构

```text
src/cli/
├── src/
│   ├── engine.py            执行器本体，1615 行，提供四个子命令
│   └── validate_outputs.py  校验项目完整性与执行结果
├── docs/
│   ├── index.md             使用入口，执行步骤与完成判断
│   ├── request-schema.md    请求 YAML 的字段定义
│   ├── scenarios.md         七个场景的判定规则
│   └── contract-rules.md    命名、目录、文档许可与 Git 安全规则
├── assets/
│   └── specification.yaml   规格文件，20 类资产与 7 个场景
├── README.md
└── ROADMAP.md
```

## 运行要求

Python 3.10 以上、PyYAML 6、Git 三者必需。操作公开 GitHub 时另需已认证的 GitHub CLI 与可用的 Git 账号权限。

```sh
python3 src/engine.py --help
```

## 四个子命令

`plan` 读取请求配置并做只读调查，生成待确认的计划。它绑定引擎与规格的指纹、锁定来源提交，产出 `execution-plan.md` 与 JSON 记录。

```sh
python3 src/engine.py plan --config request.yaml --workspace <工作区> --run-dir <记录目录>
```

`approve` 在人工审阅后确认具体计划，确认记录必须绑定完整计划编号，改动一个字都需要重新规划。测试用的 `--simulated` 只能用于 local 模式，且如实记为模拟反馈。

```sh
python3 src/engine.py approve --run-dir <记录目录> --reviewer <审阅者> --accept <完整计划编号>
```

`apply` 按检查点逐步执行已确认的动作，分层提交推送并验证结果。没有确认记录时程序拒绝执行；失败会暂停在可恢复状态，不扫描并提交无关文件。

```sh
python3 src/engine.py apply --run-dir <记录目录>
```

`verify` 逐项核对提交一致性、子模块指针、目录坐标，以及契约与 `.gitmodules` 的对应关系。

```sh
python3 src/engine.py verify --run-dir <记录目录>
```

local 是默认模式，创建本地 bare 仓库；需要操作公开 GitHub 时显式指定 `--provider github --organization <账号或组织>`。

## 规格文件

`assets/specification.yaml` 登记 20 类资产、17 个保留简称、20 个标准目录坐标与 7 个场景，并把来源锁定到具体提交：原始流程 `2b5666c`、资产章程 `0301294`。执行器的计划会记录该文件的 sha256，规格一变，旧计划即失效。

规则的判定方式分两类，程序校验与人工审阅分列，中间不留「AI 自行把握」的地带。领域边界是否准确、英文全名含义是否恰当属于人工审阅项，程序不代为判断。

## 当前状态

本次重组只移动了文件位置，尚未修改代码与文档内容，因此有三处已知残留：

- `validate_outputs.py` 的 `required` 仍按旧的 Skill 布局列举文件，当前运行报 5 项缺失并返回 1；
- `docs/index.md` 中 `scripts/engine.py` 与三个 `references/` 链接均已失效；
- `assets/specification.yaml` 的 `runtime.entrypoint` 仍写着 `scripts/engine.py`。

此外尚无 `pyproject.toml`、包入口与控制台命令，本目录是源码树而非可安装的 Python 项目。改造计划见 [ROADMAP.md](./ROADMAP.md)。
