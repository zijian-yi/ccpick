# ccpick

按项目管理 Claude Code 扩展。

## 问题

Claude Code 的命令、技能和插件要么放在 `~/.claude/` 全局加载到每个会话，要么手动拷贝到各项目的 `.claude/` 目录。全局加载意味着一个只在前端项目有用的 `frontend-design` 技能会出现在所有项目中——而每个加载的扩展都会占用上下文窗口（即使技能采用渐进式加载，我们仍然希望排除无关上下文）。手动拷贝既繁琐又会产生多份副本，无法保持同步。

## 安装

前置条件：

- Git
- Rust、Cargo (https://www.rust-lang.org/tools/install)

```bash
git clone https://github.com/zijian-yi/ccpick
cd ccpick
cargo install --path .
```

## ccpick 做了什么

ccpick 在 `~/.claude/ccpick/` 维护一个集中的扩展库，保持全局目录干净。对每个项目，你可以选择启用哪些命令和技能——ccpick 通过符号链接将它们放入项目的 `.claude/` 目录，保证单一来源、自动同步。ccpick 同时管理 Claude Code 的插件机制，将启用/禁用状态写入项目的 `.claude/settings.local.json`。所有选择记录到清单文件中，方便复用。

## 快速开始

```bash
# 1. 将现有全局扩展选择性移入库中（仅需一次）
ccpick tidy

# 2. 从 GitHub 安装扩展
ccpick install owner/repo

# 3. 为当前项目选择扩展
ccpick init

# 4. 之后更新选择
ccpick edit

# 5. 从清单重新应用（例如 git clone 后）
ccpick sync
```

### 模板

保存配置并在项目间复用：

```bash
ccpick template save backend        # 将当前项目配置保存为模板
ccpick template apply backend       # 应用到其他项目
```

### 指南

用可复用的模板和预设生成 `CLAUDE.md` 和 `AGENTS.md`：

```bash
ccpick guide template create base       # 创建包含 {{ slot }} 占位符的模板
ccpick guide preset create language rust # 创建插槽预设
ccpick guide compose base               # 交互式填充插槽，写入两个文件
ccpick guide apply rust-backend          # 直接应用顶层预设
```

完整命令参考请查看 [USAGE.md](USAGE.md)。

## 工作原理

1. `ccpick tidy` 将 `~/.claude/commands/` 和 `~/.claude/skills/` 中的文件移入 `~/.claude/ccpick/{commands,skills}/`（扩展库）。
2. `ccpick init` 扫描扩展库，展示交互式多选界面，然后在项目的 `.claude/commands/` 和 `.claude/skills/` 中创建指向扩展库的绝对符号链接。
3. 对于插件，ccpick 读取已安装插件注册表（`~/.claude/plugins/installed_plugins.json`），让你选择对当前项目启用或禁用，并将结果写入 `.claude/settings.local.json`。
4. 所有选择保存到 `.claude/ccpick.json`（清单文件）。运行 `ccpick sync` 可从清单无交互地重新应用。
5. `.claude/.gitignore` 中的托管区块自动排除符号链接条目，同时保留用户自定义的忽略规则。

每个项目的 `.claude/` 目录相互独立。运行时不修改全局状态，多项目并发使用是安全的。

## 致谢

- [trailofbits/claude-code-config](https://github.com/trailofbits/claude-code-config)
