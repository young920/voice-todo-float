# 一纸待办

一个桌面悬浮待办小组件,语音输入新建任务,自动同步到飞书多维表格。设计为常显于桌面一角,随时查看、随时标记完成。

附带**一纸锦囊**模块,管理网址书签(名称 / 链接 / 描述 / 分类 / 标签 / 重要程度)。

## 技术栈

- **Tauri v2** — Rust 后端 + 原生 Webview 前端
- **lark-cli** — 调用飞书官方 CLI 读写多维表格,无需自己维护 OpenAPI token
- **Web Speech API** — 浏览器内置语音识别("TODOList xxx"触发)

## 下载安装

每个发布的版本都有现成的安装包,直接下下来装就能用:

👉 **[Releases 页面](https://github.com/young920/voice-todo-float/releases)** 下载最新版

| 平台 | 安装包 | 说明 |
|------|--------|------|
| Windows | `voice-todo-float-<版本>_x64-setup.exe` | NSIS 安装包,需 Windows 10/11 x64 |
| macOS | `voice-todo-float-<版本>_universal.dmg` | Universal,Apple Silicon + Intel |

也可以从 GitHub Actions 的 [Build Tauri](https://github.com/young920/voice-todo-float/actions/workflows/build-tauri.yml) workflow 里下最新构建产物(版本号是开发中的,不一定稳定)。

## 完整使用流程

### 步骤 1:安装 lark-cli 并登录

`lark-cli` 是飞书官方命令行工具,本应用通过它读写你的多维表格,所以必须先装好。

#### 1.1 安装

需要 Node.js 18+:

```bash
npm install -g @larksuite/cli
```

验证安装:

```bash
lark-cli --version
```

#### 1.2 登录

首次使用需要登录飞书账号:

```bash
lark-cli auth login
```

**这个命令会做什么:**

1. 提示你输入 **profile 名称**(本应用用来识别身份,例如 `personal` / `work` / `yang`)
2. 弹出一个**浏览器窗口**,你需要在浏览器里用飞书 App 扫码或账号密码登录
3. 登录成功后,凭证会保存到本地(默认用 keychain)

**profile 命名建议:**

- 一个 profile = 一个飞书账号 / 一种用途
- 常用格式:`<用户名>` 或 `<用户>-<用途>`,例如 `yang-personal` / `yang-work`
- ⚠️ 这个名字后面要填到配置文件的 `profile` 字段,记好

如果你有多个飞书账号(比如工作 + 个人),可以登录多个:

```bash
lark-cli auth login          # 第一次,起名 personal
lark-cli auth login          # 第二次,起名 work
```

切换/查看当前 profile:

```bash
lark-cli auth list           # 列出所有已登录 profile
lark-cli config use-profile personal   # 切换当前默认
```

#### 1.3 (macOS 必需)降级 keychain

> ⚠️ **macOS 用户必须执行这一步**,否则应用读不到凭证会报错。

macOS 应用有沙箱限制,默认的 keychain 存凭证方式沙箱外读不到:

```bash
lark-cli config keychain-downgrade
```

并确保 `~/Library/Application Support/lark-cli` 下的文件当前用户可读(默认就是)。

#### 1.4 验证登录成功

```bash
lark-cli base +table-list --help
```

不应该报错"未登录"。如果提示 token 过期或缺失,重新跑 `lark-cli auth login`。

### 步骤 2:准备飞书多维表格

需要两张表:一张存任务,一张存书签(锦囊)。

#### 2.1 创建多维表格

1. 打开 [飞书多维表格](https://feishu.cn/base),新建一个 Base
2. 打开 Base 后,浏览器地址栏的 URL 长这样:

   ```
   https://feishu.cn/base/<BASE_TOKEN>?table=<TABLE_ID>
   ```

   - `<BASE_TOKEN>` = 形如 `UB9MbMmHFaJRISs6jwqc5jLtnBz` 的字符串
   - `<TABLE_ID>` = 形如 `tblC5qyGBp6u3HcK` 的字符串

3. **记下这两个值**,步骤 4 要填

#### 2.2 创建任务表字段

在 Base 里新建一张表,字段如下(字段名必须完全一致,带空格也算):

| 字段 | 类型 | 可选值 / 说明 |
|------|------|---------------|
| 任务名称 | 文本 | **必填** |
| 状态 | 单选 | `待办` / `进行中` / `已完成` |
| 优先级 | 单选 | `高` / `中` / `低` |
| 截止时间 | 日期时间 | 用于区分"计划"与"随时",可空 |
| 备注 | 文本 | 可空 |
| 链接 | 文本 | 可空,点击用外部浏览器打开 |
| 创建时间 | 日期时间 | 创建时自动写入 |
| 完成时间 | 日期时间 | 状态变为"已完成"时自动写入 |

字段名创建后支持修改,但**类型一旦选错就很难改**(比如把"单选"建成"文本"),务必确认。

记下这张表的 `table_id`。

#### 2.3 创建书签(锦囊)表字段

再建一张表(或在同一个 Base 下新建),字段:

| 字段 | 类型 | 可选值 / 说明 |
|------|------|---------------|
| 名称 | 文本 | **必填** |
| 链接 | 链接 | **必填**,URL |
| 描述 | 文本 | 可空 |
| 分类 | 文本 | 可空,例如 `工具` / `阅读` / `设计` |
| 标签 | 文本 | 可空,多个标签用**英文逗号**分隔,例如 `AI,效率,工具` |
| 重要程度 | 文本 | 可空,`高` / `中` / `低` / `无`,留空表示"无" |
| 创建时间 | 日期时间 | 创建时自动写入 |

⚠️ **标签字段是普通文本**,不是飞书的多选标签 —— 用逗号分隔即可。重要程度也是文本,不是单选 —— 这样便于扩展自定义值。

记下这张表的 `table_id`。

#### 2.4 (可选)设置表格权限

确保你登录的飞书账号对这两张表有**读写权限**(默认创建者就有)。

如果你用的是企业自建应用(profile 不是个人飞书账号),需要给应用授予 Base 的编辑权限。

### 步骤 3:安装并启动应用

从 [Releases](https://github.com/young920/voice-todo-float/releases) 下载对应平台的安装包:

- **Windows**:双击 `.exe`,一路 Next,完成
- **macOS**:双击 `.dmg`,把 `一纸待办` 拖进 Applications,从启动台打开

⚠️ macOS 首次打开如果提示"无法验证开发者",右键 → 打开 → 确认。

### 步骤 4:填写配置

应用启动时会读 `~/.hermes/scripts/voice-todo-float/config.json`,缺失会自动建模板并退出(让你填)。

#### 配置文件路径

```
~/.hermes/scripts/voice-todo-float/config.json
```

- **Windows**: `C:\Users\<你>\.hermes\scripts\voice-todo-float\config.json`
- **macOS / Linux**: `/Users/<你>/.hermes/scripts/voice-todo-float/config.json`

> 路径里的 `.hermes/scripts/` 是为了和 Hermes Agent 的脚本统一管理,跟 Hermes 本身没关系,不是必须装。

#### 手动填写(如果你想绑自己的 Base)

```bash
mkdir -p ~/.hermes/scripts/voice-todo-float
```

**macOS / Linux:**

```bash
cat > ~/.hermes/scripts/voice-todo-float/config.json << 'EOF'
{
  "base_token": "UB9MbMmHFaJRISs6jwqc5jLtnBz",
  "table_id": "tblC5qyGBp6u3HcK",
  "profile": "personal",
  "favorites_table_id": "tblMWc2mZ5kVLv4L",
  "tags_table_id": "tblfXvTCLxXFGRlV"
}
EOF
```

**Windows (PowerShell):**

```powershell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.hermes\scripts\voice-todo-float"
@'
{
  "base_token": "UB9MbMmHFaJRISs6jwqc5jLtnBz",
  "table_id": "tblC5qyGBp6u3HcK",
  "profile": "personal",
  "favorites_table_id": "tblMWc2mZ5kVLv4L",
  "tags_table_id": "tblfXvTCLxXFGRlV"
}
'@ | Set-Content "$env:USERPROFILE\.hermes\scripts\voice-todo-float\config.json"
```

字段说明:

| 字段 | 含义 |
|------|------|
| `base_token` | 步骤 2.1 拿到的 Base 标识 |
| `table_id` | 步骤 2.2 拿到的任务表 ID |
| `favorites_table_id` | 步骤 2.3 拿到的书签表 ID(1.0.14+ 需要) |
| `tags_table_id` | 预留,目前未启用,填任意字符串占位即可 |
| `profile` | 步骤 1.2 你给 lark-cli 设的 profile 名 |

#### 自带默认值兜底(1.0.15+)

从 **1.0.15** 开始,如果 config 文件完全不存在 / 字段缺失,应用会用内置默认值自动补全(指向作者的私有 Base)。**这只是为了方便试用**,你要正经用请按上面手动填写自己的。

⚠️ 注意:默认值是作者的 Base,**只有作者本人登录的 lark-cli 才能读写**。如果你想用自己的数据,**必须手动填写** config.json。

### 步骤 5:验证

启动应用后:

1. 主界面应该能看到任务列表(初次是空的)
2. 点击 "+" 新建一条任务,填名字,保存
3. 打开飞书 Base,刷新 —— 应该能看到刚加的任务
4. 在飞书里改一条任务的"状态"为"已完成",3-5 秒后桌面应用里也应该看到勾选

如果任务同步失败:

- **macOS**:确认执行过 `lark-cli config keychain-downgrade`
- **Windows**:确认 config.json 里 `profile` 和你 `lark-cli auth list` 里看到的对得上
- 查看应用日志:`~/.hermes/scripts/voice-todo-float/app.log`

## 多账户(多 profile)

如果你有多个飞书账号,可以都登录到 lark-cli,应用一次只能用一个。

```bash
lark-cli auth login          # 登录第一个
lark-cli auth login          # 登录第二个
lark-cli auth list           # 列出所有
```

切换应用的 profile = 修改 `config.json` 的 `profile` 字段,重启应用即可:

```json
{
  ...
  "profile": "work",   // ← 改成另一个 profile 名
  ...
}
```

应用本身不需要重装。

## 功能

### 待办

- 悬浮窗,支持置顶 / 折叠 / 最小化 / 关闭
- 任务分组:全部 / 今日 / 计划 / 随时 / 已批
- 语音识别建任务,自动解析"今天/明天/后天/下周/下月"
- 任务可编辑、删除、状态切换
- 链接点击外部浏览器打开
- 飞书 Base 自动同步(3-5 秒)

### 一纸锦囊

- 书签管理:名称 / 链接 / 描述 / 分类 / 标签 / 重要程度
- 按分类筛选 + 关键词搜索(匹配名称 / 描述 / 链接 / 标签)
- 重要程度彩色徽章,排序按 创建时间↓ 然后 重要程度↓
- 自由格式标签,逗号分隔

### 语音

- 点击标题栏 🎙️ 或任务页语音按钮
- 必须以 **"TODOList"** 开头,例如:"TODOList 明天下午三点开会,高优先级"
- 浏览器需要授权麦克风权限

## 开发

### 本地构建

需要:
- **Rust**(stable,通过 [rustup](https://rustup.rs/))
- **Node.js** 20+
- **macOS**:Xcode Command Line Tools(`xcode-select --install`)
- **Windows**:Visual Studio Build Tools + NSIS(通过 `choco install nsis`)

```bash
git clone https://github.com/young920/voice-todo-float.git
cd voice-todo-float
npm install

# 开发模式(热重载)
npm run dev

# 构建
npm run build-mac   # macOS .dmg
npm run build-win   # Windows .exe
```

构建产物输出在 `src-tauri/target/<target-triple>/release/bundle/`。

### 项目结构

```
.
├── src/                          # 前端 UI(单文件)
│   └── index.html                # 内联 JS + 样式
├── src-tauri/                    # Rust 后端
│   ├── src/main.rs               # Tauri commands + lark-cli 调用
│   ├── Cargo.toml
│   └── tauri.conf.json
└── .github/workflows/
    └── build-tauri.yml           # GitHub Actions 构建
```

## 常见问题

<details>
<summary><b>Q: 应用启动后弹出"未配置锦囊表ID"</b></summary>

`config.json` 缺 `favorites_table_id` 字段。

- 升级到 **1.0.15+**:缺字段会自动用内置默认值补全
- 想用自己的锦囊表:按上面"步骤 4"添加 `favorites_table_id` 字段,重启应用
</details>

<details>
<summary><b>Q: macOS 提示"未授权访问 keychain"或应用打不开</b></summary>

执行 `lark-cli config keychain-downgrade` 并重启应用。
</details>

<details>
<summary><b>Q: 任务加进去之后飞书里看不到</b></summary>

1. 检查 `lark-cli auth list` 确认 profile 已登录
2. 检查 config.json 里 `base_token` / `table_id` 是不是对(URL 里复制)
3. 看 `~/.hermes/scripts/voice-todo-float/app.log` 日志
4. 飞书 Base 默认有 ~3 秒延迟,等等刷新
</details>

<details>
<summary><b>Q: 如何换 Base / 多账户</b></summary>

编辑 `config.json` 里的 `base_token` / `table_id` / `profile`,重启应用。
profile 多账户用 `lark-cli auth login` 加,然后切换 config 的 `profile` 字段。
</details>

<details>
<summary><b>Q: 语音识别不工作</b></summary>

浏览器内置的 Web Speech API 需要联网,而且 Chrome 内核才能用。
- macOS:用 Chrome / Edge / Brave,不要用 Safari
- Windows:Edge / Chrome / Brave
- 首次使用需要授权麦克风权限
</details>

## 路线图

见 [CHANGELOG.md](./CHANGELOG.md) 的 Planned 段落。1.1.0 计划:
- Chrome / Firefox HTML 书签导入 → 锦囊
- 截止时间前的本地提醒

## 致谢

- 设计灵感:宣纸质 + 印章红的传统中式文档风格
- 框架:[Tauri](https://tauri.app)
- 数据同步:飞书 [lark-cli](https://open.larkoffice.com/document/uAjLw4CM/okzOTsz4ycsLzkjM/jsu6iR)

## License

MIT