# Changelog

All notable changes to 一纸待办 / voice-todo-float are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/) style.
Versions follow [Semantic Versioning](https://semver.org/).

## Versioning policy

- **patch** (1.0.x): bug fixes, visual polish, no schema/code-contract changes
- **minor** (1.x.0): new user-facing features, new optional Base fields, additive command signatures
- **major** (x.0.0): breaking changes (Base schema migration, removed commands, incompatible Tauri upgrade)

A new minor or major release MUST update this file in the same commit that
bumps the version. Each user-visible change MUST be traceable back to a
single commit on `main`.

## Planned (backlog)

- 1.2.0: per-category color themes

---

## [1.3.1] - 2026-10-10

### Fixed
- **纪念日弹窗默认弹出。** 之前 `.ann-modal-overlay` 只靠内联 `style="display:none;"` 隐藏，CSS 里却是 `display:flex`，如果内联样式丢失会默认弹出。现在 CSS 默认 `display:none` 兜底，JS 用内联样式控制显示/隐藏，打开应用不再默认弹出。
- **纪念日表单无间距。** 名称框、日期选择栏、备注栏之间加了 8px 间距，保存按钮上方加了留白，布局不再拥挤。
- **保存重复创建 + 内容残留。** 保存按钮加了防重复提交（`isSavingAnn` 标志 + 按钮禁用），保存慢时连点两次只会创建一条；保存成功后清空名称和备注输入框。
- **纪念日强化显示。** 当日待办区域顶部新增红色横幅，强化显示当天是什么日子（如「结婚纪念日 · 领证」），横幅下方再列出具体待办事项。

---

## [1.3.0] - 2026-10-10

### Added
- **纪念日模块（独立于待办）。** 新增独立的飞书多维表格「纪念日」表（`tblAt0NJ5gaMYiFB`），与待办/锦囊/标签完全隔离。支持：
  - 在日历工具栏点击「纪念日」按钮打开管理弹窗。
  - 添加纪念日：名称、日期类型（农历/阳历）、月/日、类型（生日/纪念日/节日/其他）、备注。
  - 查看全部纪念日列表，可删除。
  - 日历格子上自动标注当天对应的纪念日（红色小字）。
- **农历纪念日自动换算。** 农历纪念日每年自动换算到对应的阳历日期并标注在日历上（如「腊月廿三」每年落在不同的阳历日）。阳历纪念日按固定月/日标注。
- **新增后端命令：** `get_anniversaries`、`create_anniversary`、`delete_anniversary`。

---

## [1.2.9] - 2026-10-10

### Fixed
- **日历出现两个空白面板。** 之前 `cal-picker`（月份/年份选择浮层）和 `cal-holiday-panel`（放假安排浮层）只靠内联 `style="display:none"` 隐藏，如果内联样式丢失或渲染时被意外触发，会以「空白面板」形式出现在日历顶部（一个在「第N天·第N周」右侧，一个在星期标题下方），点击日历任意处才消失。现在：
  - 两个浮层的 CSS 加了 `display: none` 兜底，即使内联样式丢失也不会显示。
  - `renderCalendar` 开头强制隐藏两个浮层，无论之前是否被意外显示，进入日历模块时一律隐藏。
- **启动时出现空白提示框。** 之前 `toast`（底部气泡）和 `loading-overlay`（全屏加载遮罩）初始只有 `opacity: 0` 隐藏，如果 CSS 加载异常或过渡动画卡住，会以「空白框」形式显示出来。现在给两者都加了 `visibility: hidden`，只有真正显示时才 `visibility: visible`，即使透明度失效也不会出现空白框。
- **空内容提示保护。** `showToast` 和 `showError` 现在会忽略空字符串/纯空白内容，不再弹出空白气泡或空白横条。
- **加载遮罩强制超时关闭。** `loadingOverlay` 最多显示 3 秒，即使某个异步加载卡住没调用 `hideLoading`，遮罩也会自动消失，不会一直盖住页面。

---

## [1.2.7] - 2026-10-10

### Fixed
- **日历格子宽度被节日名称撑开，导致表头不对齐。** 之前格子内容（节日/农历文字）用 `white-space: nowrap`，长名称会撑宽格子，导致 7 列宽度不一致、表头「周一到周日」与格子错位。现在：
  - 格子加 `min-width: 0`，保证 7 列宽度始终一致，不被内容撑开。
  - 节日/农历/节气文字改为允许折行（最多两行），长名称自动换行，不再撑宽格子。
  - 某一行格子内容多时，该行高度自动变高，但宽度保持不变。

---

## [1.2.6] - 2026-10-10

### Fixed
- **启动时重复提示气泡。** 之前后端连不上时，`invoke()` 失败就弹 toast（底部气泡），`loadTasks` 失败又弹 error-banner（顶部横条），多个 invoke 失败会叠加出多个气泡。现在 `invoke()` 不再直接弹 toast，由各调用方统一用 error-banner 提示一次；error-banner 加了自动消失（4 秒）和点击关闭。
- **完整黄历展开不再挤窄日历。** 之前点「完整黄历」展开后，详情面板变高会挤压日历格子，导致日历变矮要滚动。现在日历格子保持固定高度，整个日历模块向下扩展，由外层滚动。
- **待办数字被节日文字遮挡。** 待办数字原来在格子右下角，节日文字（如「记者节」）居中延伸到右下会盖住它。现在移到右上角，与节日文字错开。
- **非当月日期空白。** 之前非当月格子不显示日期数字，11 月第一排（1-6 号）是空的，看着很怪。现在非当月也显示日期数字和农历，用浅灰色淡化，不再空白。
- **日历区域自适应高度。** 之前日历格子是 `flex:1` + 内部滚动，高度被详情面板挤压只能滚动。现在按内容自然撑开，不再内部滚动。
- **周一到周日表头不对齐。** 之前表头「周一到周日」与下方格子错位歪斜，现已对齐。

---

## [1.2.5] - 2026-10-10

### Changed
- **节日数据源改为 lunar.js 全量内置，不再手写拼凑。** 之前节日是「lunar.js 农历节日 + holiday-cn 法定假日 + 手写 20 个」拼出来的，不全且容易漏。发现 lunar.js 的 `Solar`（公历）对象本身就带 `getFestivals()`（24 个公历节日：元旦/国庆/圣诞/平安夜/感恩节/母亲节/父亲节…）和 `getOtherFestivals()`（158 个补充节日：世界抗癌日/国家公祭日/香港回归纪念日/各种情人节…），加上农历节日，**全年共 223 个去重节日、覆盖 181 天**（之前约 60 个）。全部离线可用，无需网络请求。
- **格子只显示主要节日，详情显示全部。** 为避免格子被"世界艾滋病日"这类长尾补充节日挤满，新增 `mainFestivals`（格子用，只含主要节日 + 白名单里的重要补充节日）和 `festivals`（详情用，全部）。国家公祭日、香港回归纪念日、九一八事变纪念日等 23 个重要补充节日也显示在格子上。
- **修正清明节。** 之前手写硬编码 4 月 4 日为清明节，实际清明是浮动节气（4 月 4-6 日），现改由 lunar.js 节气自动处理。
- **元旦显示优化。** lunar.js 返回"元旦节"，现统一显示为"元旦"。

---

## [1.2.4] - 2026-10-10

### Added
- **补充西方/现代/纪念节日。** 之前日历只显示中国传统节日和法定节假日，缺平安夜、圣诞节、感恩节、国家公祭日、情人节、儿童节、万圣节、跨年夜等。现在补上 20 个节日（含浮动日期的感恩节——每年 11 月第 4 个周四），格子和详情面板都会显示。
- **完整黄历展开区。** 详情面板新增「完整黄历」折叠区（默认收起，点击展开），展示生肖、彭祖百忌、年月日五行、星座、星宿、季节、胎神、冲煞、六曜、值神、佛历年、儒略日等 12 项万年历信息。

### Fixed
- **`escapeHtml` 对非字符串（数字）报错。** 儒略日是数字，直接传入 `escapeHtml` 会抛 `text.replace is not a function`。修复：先转字符串再转义。

---

## [1.2.3] - 2026-10-10

### Changed
- **农历"初一"显示改为"月 · 初一"。** 之前初一那天只显示农历月份（如"九"），容易误看成"初九"。现在显示"九 · 初一"，既能看到是九月，又明确是初一。其他日子仍只显示日（如"廿九"）。

---

## [1.2.2] - 2026-10-10

### Fixed
- **日历格子内容被截断。** 某天标签多（如农历+节日）时，`.cal-grid` 固定行高 `grid-auto-rows:1fr` + `.cal-cell` 的 `overflow:hidden` 会把底部文字裁掉一半。修复：行高改为 `minmax(min-content,1fr)`（格子按内容自适应撑高，网格本身可滚动），并移除 `overflow:hidden`，保证任何格子的字都不被截断。

---

## [1.2.1] - 2026-10-09

### Fixed
- **放假安排加载卡死 + 需手动点击才加载。** 根因是 Tauri CSP `connect-src` 未允许 `raw.githubusercontent.com`，导致 `fetch` 节假日数据被拦截挂起。修复：CSP 放行该域名；`loadHolidays` 加 AbortController 8s 超时，防止 fetch 挂起卡死 UI。
- **放假面板闪"加载中"空框。** 数据已缓存时立即渲染结果，不再先闪"加载中…"再更新。
- **初次进入日历视图顶部出现空提示栏。** 放假面板只在用户主动点「放假」按钮时展开，不会自动弹出；数据预加载后点开立即显示完整列表。

### Added
- **预加载所有年份放假数据（2019–2026+）。** 进入日历即自动拉取并缓存各年份法定节假日/调休数据，翻到任何年份「休/补」标签立即显示，无需点按钮、无需等待。注：2027 及以后国务院尚未公布放假安排，数据源暂缺，仅显示周末休息。

---

## [1.2.0] - 2026-10-09

### Added
- **日历视图大幅增强（参照黄历 App 设计）。**
  - **农历**：每个日期格下方显示农历日（初一显示农历月份），数据源为 `lunar-javascript`（MIT 开源、纯前端离线计算、免费无 key、无网络依赖）。
  - **节气**：节气日以金色标注（如「寒露」「霜降」）。
  - **节日**：显示传统节日（重阳节、世界邮政日等），与法定节假日互补。
  - **黄历宜忌**：选中日期后，详情面板显示农历、干支（丙午年戊戌月丙辰日）、生肖、宜/忌事项。
  - **起始日切换**：工具栏新增「周一/周日」按钮，切换每周起始日，星期表头与格子同步重排。
  - **放假安排下拉**：工具栏新增「放假」按钮，下拉展示全年法定节假日与调休补班（休/补标签）。
  - **第X天·第X周**：工具栏显示当前月首日的年内天数与 ISO 周数。

### Fixed
- **Windows 任务栏找不到应用（最小化后消失）。** 根因是 `tauri.conf.json` 的 `skipTaskbar: true`（对 Mac 菜单栏 widget 合理，但 Windows 无此机制）。现改为在 `setup()` 中按平台处理：Windows 上 `set_skip_taskbar(false)` 让窗口显示在任务栏，Mac 保持 `true`。
- **日历底部格子被图例栏遮挡。** 调整 `.cal-grid` 为 `grid-auto-rows: 1fr` + 格子自适应高度，确保最后一行日期完整可见。

---

## [1.1.0] - 2026-10-08

### Added
- **日历视图模块。** 标题下拉菜单新增「日历」选项，点击切换到月历视图。
  - 标注法定节假日（元旦/春节/清明/劳动/端午/中秋/国庆），休息日红色、调休补班灰色+删除线并标注「·补」。
  - 数据源为 `NateScarlet/holiday-cn`（自动抓取国务院公告），免费无 key，按年缓存。
  - 每个日期格显示当天待办数量角标（按 `截止时间` 聚合）。
  - 支持上/下月切换、跨年、点击「今」回到当前月。
  - 设计遵循一纸编辑感：米白纸感 + 朱砂红，休息日红、补班灰、今天红框高亮。

---

## [1.0.16] - 2026-08-03

### Fixed
- **Windows widget sync failed with `同步锦囊失败: not_configured`.**
  When `HERMES_HOME` / `OPENCLAW_HOME` / `LARK_CHANNEL` (or any of the other
  hermes-agent runtime env vars) leaked into the widget's process
  environment, `lark-cli` auto-detected "Agent context", forced the bind
  path, and returned `{"ok": false, "error": {"subtype": "not_configured"}}`
  even though `lark-cli auth status` reported `ready`.
  `build_command_with_executable()` now strips all `HERMES_*` /
  `OPENCLAW_HOME` / `LARK_CHANNEL` keys from the spawned `Command`'s
  environment so the widget's `lark-cli` calls bypass Agent context
  detection and use the direct user-identity path. Verified on Windows
  1.0.15 (broken) → 1.0.16 (sync restored).

### Internal
- Version bumped in `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`.

## [1.0.15] - 2026-07-21

### Fixed
- **Fresh installs on Windows could not sync favorites/tags.** `未配置锦囊表ID`
  / `未配置标签表ID` errors appeared because the legacy `Config::load()`
  returned `None` when `favorites_table_id` / `tags_table_id` were missing
  from `config.json`. 1.0.13-era configs (which never wrote those fields)
  hit this path every time.
  `Config::load()` now backfills any missing/empty field from bundled
  defaults and persists the updated config back to disk, so the upgrade
  sticks across restarts. Fresh installs (no config file at all) now
  bootstrap with a fully populated `config.json` instead of an empty
  template that forces the user to fill it in by hand.

### Internal
- Version bumped in `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (1.0.14 → 1.0.15).
- One feature commit + one version-bump commit, kept separate so the
  fix can be cherry-picked if needed.

---

## [1.0.14] - 2026-07-21

### Added
- **Favorites: 重要程度 (priority) field.** Add and edit forms now expose a
  `高 / 中 / 低 / 无` dropdown. Cards display a colored badge next to the
  title. The list sorts by `created_at` desc, then priority desc.
  Field already exists in the Base as plain text — this version only
  surfaces it in the UI. Old records without the field render with no
  badge and sort to the bottom.

### Changed
- **Favorites: 标签 input is now free-form.** Any non-empty tag is
  accepted. The existing-tag datalist is still offered as suggestions.
  Previously the frontend rejected unknown tags, which was over-strict
  because the Base 标签 column is plain text, not a strict multi-select.
- **Favorites: 分类 is now optional on save.** Empty category no longer
  forces an empty 分类 cell; the field is omitted from the upsert
  payload when blank.
- **Forms: `<select>` styling unified** with `<input>` (32px height,
  custom SVG chevron, no native arrow). Closes the visual gap between
  dropdowns and text inputs in the favorites edit form.
- **Title bar:** long module/category names now ellipsize at 120px
  instead of pushing the right-side buttons off-screen.

### Fixed
- **macOS: tmp dir under `config_dir()/tmp` could not be written** when
  the app was launched from `/Applications/` or a DMG (sandbox returns
  `Permission denied (os error 13)`). Switched the macOS tmp dir to
  `$HOME/.voice-todo-float-tmp`; Windows/Linux behavior unchanged.
- **`update_favorite` Tauri command signature** switched from a
  `HashMap<String, Value>` payload to flat named parameters
  (`id, name, link, description, category, tags, priority`). The
  previous form expected the frontend to send a single object keyed by
  `payload`; the actual frontend call passes flat args, which would
  have caused `missing required key payload` errors on save.
- **Favorites 标签 (tags) save silently failed at the Base layer.**
  `create_favorite` / `update_favorite` were pushing the raw
  `Vec<String>` into a plain-text cell, which Base rejects with
  `800010407 "The cell value does not match the expected input shape"`.
  Tags are now joined with `,` before writing and split back into a
  `Vec<String>` on read. This bug existed in 1.0.13 (introduced in
  the favorites module rewrite `cf92c25f`); affected any save with
  one or more tags.
  Verified end-to-end against the live Base via `lark-cli`:
  create with `["qa","1.0.14","end2end"]` writes cell `"qa,1.0.14,end2end"`,
  update with `priority=中` + new tags reflects after the standard
  ~3-5s eventual-consistency window.

### Internal
- Version bumped in `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.lock` (1.0.13 → 1.0.14).
- Two atomic commits on top of 1.0.13:
  - `486dad9` backend (priority + signature + macOS tmp)
  - `3c2119a` frontend (priority UI + tag relaxation + select + title)
- One follow-up bug fix:
  - `62d1642` fix tags to write as comma-joined string (Base plain-text
    cell rejected the raw `Vec<String>`)

---

## [1.0.13] - 2026-07-18

### Fixed
- **Stale `lark-cli` writes overwriting optimistic UI state.** Removed
  the immediate `+record-get` after `update_task`; the backend now
  returns lightweight success and the frontend keeps the optimistic
  update, with a delayed silent refresh ~3s later. Applies to add /
  edit / toggle for both `task` and `favorite` flows.
- **macOS sandbox tmp write** — same family of fix as 1.0.14; the
  1.0.14 commit finalized the path (`$HOME/.voice-todo-float-tmp`).
- **Duplicate edit-form input IDs** when a task appears in multiple
  tabs (e.g. 全部 + Scheduled). Edit form now only renders in the
  active tab.
- **Tags filtered against an existing-tag dictionary** to avoid
  Base `not_found` rejections on favorites. Relaxed in 1.0.14 because
  the Base column is plain text.

### Verified
- GitHub Actions run `29630889308` (4m31s, success). Windows NSIS
  installer `一纸待办_1.0.13_x64-setup.exe` (3.45 MB).

---

## [1.0.12] - 2026-07-17

### Added
- **Favorites (一纸锦囊) module.** Bookmarks with 名称 / 链接 / 描述 /
  分类 / 标签 / 创建时间. Search across name/description/link/tags,
  category filter, add/edit/delete. Datalist suggests existing tags
  and categories from prior records.

### Fixed
- `update_task` response parsing; keep optimistic update on success.
- Use server-returned records for add/edit/toggle to avoid stale
  `loadTasks` overwriting fresh data.

---

## [1.0.0..1.0.11]

Initial release line. Inline HTML/CSS/JS frontend with a Rust backend
calling `lark-cli` to read/write Feishu Base. Things-3 inspired task UI
(white background, circular checkbox, SF Pro font). Modules: 待办
(All / Today / Scheduled / Anytime / Completed). Voice/text → AI →
Feishu Base → widget auto-sync.
