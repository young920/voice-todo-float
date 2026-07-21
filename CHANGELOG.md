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

- 1.1.0: bookmark import/export (Chrome/Firefox HTML bookmarks → favorites)
- 1.1.0: notification reminder X minutes before `截止时间`
- 1.2.0: per-category color themes

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

### Internal
- Version bumped in `package.json`, `src-tauri/tauri.conf.json`,
  `src-tauri/Cargo.lock` (1.0.13 → 1.0.14).
- Two atomic commits on top of 1.0.13:
  - `486dad9` backend (priority + signature + macOS tmp)
  - `3c2119a` frontend (priority UI + tag relaxation + select + title)

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
