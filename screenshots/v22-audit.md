# 一纸手记 Landing Page 审查 — 无效元素 + 过度宣传

> 审查时间：2026-07-28 · 站点版本：v21（commit c151a42）

---

## A. 严重无效（事实错误 / 功能不存在）

### A1. 产品名错（全站）
页面用「**一纸手记**」 → tauri.conf.json `productName` 是「**一纸待办**」
影响：nav logo、meta title、meta description、整个 footer 都要改

### A2. 版本号错（全站）
- Hero eyebrow：「v1.0.6 · 公開測試」
- CLI mock：「8.6 MB · v1.0.6」
- Feature Updates 标题：「從 v1.0.0 到 v1.0.6」
- Changelog pill：「一路迭代，從 v1.0.0 到 v1.0.6」

实际当前版本：**1.0.15**（CHANGELOG + Cargo.toml + package.json + tauri.conf.json）

### A3. 不存在的平台 / 入口
Hero CTA trust 行 + footer：
- 「移動端」→ 没有移动端 app
- 「瀏覽器插件」→ 没有浏览器插件
- 「Web 版」→ 没有 web 版本

实际 tauri targets = `["dmg", "nsis"]`（macOS + Windows）。README 写了 Win 但页面只字未提。

### A4. 不存在的功能 — 「MCP 通用接口」整套
- agents3 卡：「任何支援 MCP 的 agent，皆可調用 TODOList 工具」
- agents3 卡：「Hermes、OpenClaw、Claude Code——誰先建誰先寫，互不覆蓋」
- changelog：「多源同步 · Hermes · OpenClaw · Claude Code · OpenCode」
- demos 统计行：「OpenCode → 飛書 Base 0.3s」

→ **实际 product 代码里没有 MCP server 实现**
→ src-tauri/ 依赖只有 tauri / serde / which / glob / wait-timeout
→ Claude Code / OpenClaw / OpenCode 是用户自己的其他工具，voice-todo-float 没集成它们
→ 唯一真实的能力：Feishu Base 通过 lark-cli 读写（用户自己跑 lark-cli）

### A5. 不存在的功能 — 「個人/企業版自動切換」
changelog 任务 sub：「個人/企業版自動切換」
→ 没有这个功能（飞书本身有个人/企业版账号差异，但 voice-todo-float 没做自动切换）

### A6. 不存在的功能 — 「無彈窗」
Hero hint：「無彈窗、無訂閱」
→ CHANGELOG 路线图明确写了：**「1.1.0: notification reminder X minutes before 截止时间」** → 即将有弹窗

### A7. 不存在的功能 — 「NOW 標識當前焦點」
- changelog：「並行任務棧上線 · 多任務同屏推進 · NOW 標識當前焦點」
- widget mock：「跟進 N 君反饋」右边贴 NOW 标签

→ 实际 widget 是 **5 列**（全部/今日/计划/随时/已批），没有「NOW」概念
→ "NOW" 是演示花活

### A8. nav 的「註冊 / 登入」是假的
产品是本地 + 飞书 Base，**没有账号系统**
这两个按钮应该删

### A9. 编造的数据
- 「1,284 條任務已同步至飛書 Base」— 没有真实统计（产品 beta 阶段）
- 「0.3s / 0.4s / 0.5s」同步延迟 — 4 处出现，全是编的
- 「8.6 MB」下载大小 — 没测过
- 「過去 7 日」统计 — 无数据来源

### A10. 编造的用户证言（Community Voices）
12 条 fake testimonials + N/L/D/Y/M/K 6 个 fake avatar
→ 没有真实用户，全是文案组的脑补

---

## B. 过度宣传（事实但夸大 / 误导）

### B1. 「語音即錄」feature 卡文案
「一句「TODOList 明天十點回電話」即成任務」
→ README 明确说用浏览器内置 Web Speech API
→ 中文日期解析不可靠，「明天 10 点」可能识别成「明天十点零」

### B2. 「三卡 Agent」— 「版本控制 Agent」
「所有任務寫入前都經本地 Git。誤刪、誤改，git revert 即回。」
→ 这是说 Feishu Base 的 schema，不是真 git
→ tauri 代码里完全没有 git wrapping

### B3. Feature Updates 左栏
标题「語音即錄，命令可審」
描述「每一條 TODO 寫入都是 git 可審的命令流。」
→ 仍然在吹 git，没有

### B4. 「桌面常顯」feature
「無彈窗、無 dock 遮擋」
→ 跟 A6「無彈窗」矛盾
→ tauri.conf 里 `alwaysOnTop: false` — widget 本身并没 always-on-top

### B5. 「飛書直連」feature
「多端可見，手機、網頁、桌面——一處記，處處查」
→ A3 已经说没手机、没网页端

### B6. agent section 副标题
「從你的 AI 助手到飛書 Base——中間不過一句話的距離」
→ 前提是用户已经跑了一个 AI agent（Hermes 等），不是 voice-todo-float 本身的功能

---

## C. 设计 / 技术问题

### C1. glow-card SVG 滤镜
```
<feColorMatrix values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 8 -3"/>
```
alpha 放大 8 倍减 3，会出现「黑底+白色发光描边」硬亮边，在深色 hero 背景下可能很扎眼
→ 需要看实际渲染，可能要去掉或减弱

### C2. 「All updates →」anchor 死链
`<a href="#changelog">` 但 section 没有 `id="changelog"`，点不动

### C3. nav 内联 SVG 噪音
4 个 link 各带内联 SVG（X/盾牌/人形/默认），文件冗余
→ 可简化或去掉 icon

### C4. Feature Updates tabs 是装饰
「指令流 / 配置文件」两个 button 无切换逻辑
→ 装饰品，要么去掉要么实现

### C5. 4 个 stray 文件
`site/` 目录里有：
- `glow-card-v1.jpg`
- `glow-card-v2.jpg`
- `glow-final-2.jpg`
- `glow-final.jpg`

→ 临时调试截图，部署目录里不该有，应该删

---

## D. 自相矛盾 / 内伤

| 这边说 | 那边说 | 矛盾点 |
|--------|--------|--------|
| Hero hint「無彈窗」| CHANGELOG 1.1.0 reminder | 即将有弹窗 |
| 「無訂閱、無雲端賬號」| nav「註冊 / 登入」按钮 | 不该有账号 |
| 「個人/企業版自動切換」| 整个定位「极简本地」| 跟极简冲突 |
| 「多端可見，手機、網頁、桌面」| A3「移动端/浏览器插件/Web 版」不存在 | 互相拆台 |
| changelog「macOS 26 sandbox 修復」| 用户功能列表 | 内部修复当用户功能 |
| changelog「開源釋出於 GitHub」| v1.0.0 之前 | 时间线错位 |

---

## 处理优先级

### P0 — 必改（事实错 / 假功能）
1. 全站 `一纸手记` → `一纸待办`
2. 全站 `v1.0.6` → `v1.0.15`
3. 删 MCP / OpenCode / OpenClaw / 多 agent 整套叙事（agents3、changelog、demos stats）
4. 删 nav `註冊` / `登入`
5. 删 移动端 / 浏览器插件 / Web 版 trust 标签
6. 删 1,284 / 0.3s / 8.6MB 等编造数据
7. 删 macOS 26 sandbox 修复（changelog 内项）

### P1 — 必改（吹牛 / 自相矛盾）
8. Community Voices 全删，改成「TODOList 实操截图」或「Key Bindings 参考」
9. 「無彈窗、無訂閱」→ 「無訂閱、無追蹤、無雲端賬號」（去掉「無彈窗」）
10. 「個人/企業版自動切換」删
11. 「NOW 標識當前焦點」删
12. 「每一條 TODO 都是 git 可審」→ 「每條 TODO 直接寫入你的飛書 Base」
13. 「多端可見，手機、網頁、桌面」→ 「多端可見：iOS/Android 飛書 app 直接讀 Base」（如果有）
   或干脆改成「電腦、手機飛書 app，皆能讀同一張 Base」

### P2 — 设计清理
14. glow-card 滤镜测试看是否过亮，决定保留 / 减弱 / 删
15. changelog section 加 `id="changelog"`
16. nav 内联 SVG 简化或去掉
17. Feature Updates tabs 装饰去掉或实现
18. 删 4 个 stray glow-card-*.jpg
19. 「開源釋出於 GitHub」从 changelog 删（1.0.0 之前的事）
20. 「桌面常顯」描述：「無彈窗、無 dock 遮擋」→ 「置于屏幕一角，隨手即記」（去掉无弹窗）

---

## 接下来怎么改

我建议这样推：

**第一波（P0 + P1 全上）** — 一刀切，把所有假数据、假功能、假用户全清掉
预计改动量：~30 处文本 + 删除 Community Voices section + 删 4 张 stray JPG
保留视觉风格（深底 / 米金 / 朱红 / 桃色 dither）

**第二波（P2 设计清理）** — 看你视觉反馈

要不要直接动手？或者你想先定个原则再让我改？比如：
- Community Voices 删了之后用啥补？空白？TODOList 命令参考？还是 Key Bindings 表？
- 三个 Agent 卡（agents3 删到只剩 1-2 个）改成什么主题？
