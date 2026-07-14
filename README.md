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

1. 安装 Rust、Node.js 和 `@tauri-apps/cli`：
   ```bash
   npm install
   ```

2. 安装并登录飞书 CLI：
   ```bash
   npm install -g @larksuite/cli
   lark-cli auth login
   ```
   默认使用的 profile 名称建议与 `lark-cli auth login` 时使用的名称保持一致；如不同，请在配置文件中修改。

3. 准备一个飞书多维表格，包含以下字段：
   - 任务名称（文本）
   - 状态（单选：待办 / 进行中 / 已完成）
   - 优先级（单选：高 / 中 / 低）
   - 截止时间（日期时间）
   - 备注（文本）
   - 链接（文本）
   - 创建时间（日期时间）
   - 完成时间（日期时间）

4. 将 Base 的 `base_token`、`table_id`、lark-cli `profile` 写入本地配置文件：
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
   > 注意：请替换为你自己的 `base_token` 和 `table_id`。该文件位于用户目录下，不会提交到仓库。

   macOS 注意：因为 Tauri 应用沙箱限制，`lark-cli` 默认使用 keychain 存储凭证，应用内无法读取。需要执行一次：
   ```bash
   lark-cli config keychain-downgrade
   ```
   并确保 `~/Library/Application Support/lark-cli` 下的文件当前用户可读。

## 开发

```bash
npm run dev
```

## 构建

```bash
# macOS Apple Silicon
npm run build-mac

# Windows (NSIS 安装包)
npm run build-win
```

构建产物输出在 `src-tauri/target/<target-triple>/release/bundle/` 目录。

## 项目结构

```
.
├── src/              # 前端 UI
│   ├── index.html
│   └── app.js
├── src-tauri/        # Rust 后端
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── assets/           # 图标源文件
└── .github/workflows/
    └── build.yml     # GitHub Actions 跨平台构建
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

- 点击标题栏"+"或语音按钮新建任务
- 语音识别需以 **"TODOList"** 开头，例如："TODOList 明天下午三点开会，高优先级"
- 点击任务左侧圆形按钮切换完成状态
- 任务卡片右侧"改"/"删"按钮用于编辑和删除

## 下载

- **macOS Apple Silicon**: [一纸待办_1.0.0_aarch64.dmg](https://github.com/young920/voice-todo-float/releases/download/v1.0.0/一纸待办_1.0.0_aarch64.dmg)

## 打包与发布

仓库已配置 GitHub Actions，推送 `main` 分支后自动构建 macOS 与 Windows 安装包。手动发布：

```bash
npm run build-mac
```

然后将 `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/` 下的 `.dmg` 上传到 GitHub Release。

## 致谢

- 设计灵感：宣纸质 + 印章红的传统中式文档风格
- 框架：[Tauri](https://tauri.app)
- 数据同步：飞书 [lark-cli](https://open.larkoffice.com/document/uAjLw4CM/okzOTsz4ycsLzkjM/jsu6iR)

## License

MIT
