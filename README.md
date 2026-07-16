# 一纸待办

一个桌面悬浮待办小组件，语音输入新建任务，自动同步到飞书多维表格。设计为常显于桌面一角，随时查看、随时标记完成。

## 技术栈

- **Tauri v2** — Rust 后端 + 原生 Webview 前端
- **lark-cli** — 直接调用官方 CLI 读写飞书多维表格，无需自己维护 OpenAPI token
- **Web Speech API** — 浏览器内置语音识别（"TODOList xxx" 触发）

## 功能

- 悬浮窗，支持置顶/折叠/最小化/关闭
- 任务分组：全部 / 今日 / 计划 / 随时 / 已批
- 语音识别建任务，自动解析"今天/明天/后天/下周/下月"
- 任务可编辑、删除、状态切换
- 链接点击外部浏览器打开
- 飞书 Base 自动同步

## 前置条件

### 1. 通用开发环境

- **Rust**（推荐最新 stable，通过 [rustup](https://rustup.rs/) 安装）
- **Node.js** 20 或更高版本
- 克隆仓库后安装 npm 依赖：
  ```bash
  npm install
  ```

### 2. 飞书 CLI 与数据表

安装并登录飞书官方 CLI：

```bash
npm install -g @larksuite/cli
lark-cli auth login
```

> 登录时使用的 `profile` 名称需与配置文件中的 `profile` 保持一致。不同 profile 对应不同身份，应用运行时会读取该 profile 的凭证。

准备飞书多维表格，字段要求如下：

| 字段 | 类型 | 可选值 / 说明 |
|------|------|---------------|
| 任务名称 | 文本 | 必填 |
| 状态 | 单选 | 待办 / 进行中 / 已完成 |
| 优先级 | 单选 | 高 / 中 / 低 |
| 截止时间 | 日期时间 | 用于区分“计划”与“随时” |
| 备注 | 文本 | 可空 |
| 链接 | 文本 | 可空，点击用外部浏览器打开 |
| 创建时间 | 日期时间 | 创建时自动写入 |
| 完成时间 | 日期时间 | 状态变为“已完成”时自动写入 |

将 `base_token`、`table_id`、`profile` 写入本地配置文件（不提交到仓库）：

```bash
mkdir -p ~/.hermes/scripts/voice-todo-float
cat > ~/.hermes/scripts/voice-todo-float/config.json << 'EOF'
{
  "base_token": "YOUR_BASE_TOKEN",
  "table_id": "YOUR_TABLE_ID",
  "profile": "YOUR_LARK_CLI_PROFILE"
}
EOF
```

macOS 额外注意：因为 Tauri 应用沙箱限制，`lark-cli` 默认使用 keychain 存储凭证，应用内无法读取。需要执行一次：

```bash
lark-cli config keychain-downgrade
```

并确保 `~/Library/Application Support/lark-cli` 下的文件当前用户可读。

### 3. 本地构建额外依赖

#### macOS 本机构建

```bash
rustup target add aarch64-apple-darwin
npm run build-mac
```

#### macOS 交叉编译 Windows 安装包

`npm run build-win` 会调用 `cargo-xwin` 交叉编译到 `x86_64-pc-windows-msvc`。需要提前安装：

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-msvc

# 交叉编译工具链
cargo install cargo-xwin

# LLVM（tauri-winres 需要 llvm-rc、llvm-ar 等）
# 推荐通过 Homebrew 安装：brew install llvm
# 并确保 LLVM bin 目录在 PATH 中
```

#### Windows 本机构建

```bash
# 安装 NSIS（Tauri 用它来打安装包）
# 可通过 Chocolatey：choco install nsis

# 安装 Windows 目标并构建
rustup target add x86_64-pc-windows-msvc
npm run build-win
```

Windows 本机还需要 Visual Studio Build Tools 或 Visual Studio（提供 MSVC 链接器）。

## 开发

```bash
npm run dev
```

## 构建

```bash
# macOS Apple Silicon 安装包（.dmg）
npm run build-mac

# Windows 安装包（.exe，NSIS）
npm run build-win
```

构建产物输出在 `src-tauri/target/<target-triple>/release/bundle/` 目录。

## 项目结构

```
.
├── src/              # 前端 UI
│   └── index.html    # 已内联 JS 与样式
├── src-tauri/        # Rust 后端
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── assets/           # 图标源文件
└── .github/workflows/
    ├── build.yml          # 交叉平台构建（tauri-action）
    └── build-tauri.yml    # 原生 Tauri CLI 构建
```

## 配置说明

应用启动时从以下路径读取配置：

```
~/.hermes/scripts/voice-todo-float/config.json
```

格式：

```json
{
  "base_token": "YOUR_BASE_TOKEN",
  "table_id": "YOUR_TABLE_ID",
  "profile": "lark-cli-profile-name"
}
```

## 快捷键 / 语音

- 点击标题栏"​+"​或语音按钮新建任务
- 语音识别需以 **"TODOList"** 开头，例如："TODOList 明天下午三点开会，高优先级"
- 点击任务左侧圆形按钮切换完成状态
- 任务卡片右侧"改"/​"删"​按钮用于编辑和删除

## 下载

推送 `main` 分支后由 GitHub Actions 自动构建：

- [Build Tauri](https://github.com/young920/voice-todo-float/actions/workflows/build-tauri.yml) — 使用原生 Tauri CLI，产物为 `macos-dmg` 和 `windows-nsis`
- [Cross-platform Build](https://github.com/young920/voice-todo-float/actions/workflows/build.yml) — 使用 `tauri-apps/tauri-action`，产物为 `tauri-bundle-macos` 和 `tauri-bundle-windows`

手动构建后从 `src-tauri/target/<target-triple>/release/bundle/` 获取安装包。

## 打包与发布

仓库已配置两个 GitHub Actions 工作流，推送 `main` 分支后自动构建 macOS 与 Windows 安装包。手动发布步骤：

```bash
npm run build-mac
```

然后将 `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/` 下的 `.dmg` 和
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/` 下的 `.exe` 上传到 GitHub Release。

## 致谢

- 设计灵感：宣纸质 + 印章红的传统中式文档风格
- 框架：[Tauri](https://tauri.app)
- 数据同步：飞书 [lark-cli](https://open.larkoffice.com/document/uAjLw4CM/okzOTsz4ycsLzkjM/jsu6iR)

## License

MIT
