#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "windows")]
extern "system" {
    fn MessageBoxW(
        hWnd: *const std::ffi::c_void,
        lpText: *const u16,
        lpCaption: *const u16,
        uType: u32,
    ) -> i32;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    #[serde(rename = "base_token")]
    base_token: String,
    #[serde(rename = "table_id")]
    table_id: String,
    profile: String,
    #[serde(rename = "favorites_table_id")]
    favorites_table_id: Option<String>,
    #[serde(rename = "tags_table_id")]
    tags_table_id: Option<String>,
}

// Bundled defaults — applied when a field is missing or empty in the user's
// config.json. Bundling values into the binary (instead of relying on a
// separately-shipped config file) means fresh installs on Windows work
// out-of-the-box, and 1.0.13-era configs auto-upgrade on first launch.
//
// NOTE: profile, base_token, and table IDs all belong to the app author
// (Yang's own Feishu workspace / lark-cli setup). Hardcoding them is
// intentional — this is a single-tenant personal tool, not a multi-user
// product. Existing non-empty user values are never overwritten.
const DEFAULT_BASE_TOKEN: &str = "UB9MbMmHFaJRISs6jwqc5jLtnBz";
const DEFAULT_TABLE_ID: &str = "tblC5qyGBp6u3HcK";
const DEFAULT_PROFILE: &str = "cli_a976ca0e1c39dbda";
const DEFAULT_FAVORITES_TABLE_ID: &str = "tblMWc2mZ5kVLv4L";
const DEFAULT_TAGS_TABLE_ID: &str = "tblfXvTCLxXFGRlV";

impl Config {
    fn load() -> Result<Self, String> {
        let path = config_path();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("无法读取配置文件 {}: {}", path.display(), e))?;
        let mut config: Config =
            serde_json::from_str(&content).map_err(|e| format!("解析配置文件失败: {}", e))?;

        // Backfill any missing/empty field from bundled defaults, then persist
        // so the upgrade sticks across restarts.
        let mut changed = false;
        if config.base_token.trim().is_empty() {
            config.base_token = DEFAULT_BASE_TOKEN.to_string();
            changed = true;
        }
        if config.table_id.trim().is_empty() {
            config.table_id = DEFAULT_TABLE_ID.to_string();
            changed = true;
        }
        if config.profile.trim().is_empty() {
            config.profile = DEFAULT_PROFILE.to_string();
            changed = true;
        }
        if config
            .favorites_table_id
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            config.favorites_table_id = Some(DEFAULT_FAVORITES_TABLE_ID.to_string());
            changed = true;
        }
        if config
            .tags_table_id
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            config.tags_table_id = Some(DEFAULT_TAGS_TABLE_ID.to_string());
            changed = true;
        }
        if changed {
            let serialized = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("序列化配置失败: {}", e))?;
            std::fs::write(&path, serialized).map_err(|e| format!("写入配置文件失败: {}", e))?;
            log(&format!("已用默认值补全配置: {}", path.display()));
        }
        Ok(config)
    }

    fn favorites_table_id(&self) -> Option<&String> {
        self.favorites_table_id
            .as_ref()
            .filter(|s| !s.trim().is_empty())
    }

    fn tags_table_id(&self) -> Option<&String> {
        self.tags_table_id.as_ref().filter(|s| !s.trim().is_empty())
    }
}

fn config_dir() -> PathBuf {
    let home = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
    } else {
        std::env::var("HOME").map(PathBuf::from)
    };
    home.unwrap_or_else(|_| PathBuf::from("."))
        .join(".hermes")
        .join("scripts")
        .join("voice-todo-float")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn log(msg: &str) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("app.log");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{}] {}\n", now, msg);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

fn show_error_dialog(title: &str, message: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let title_w: Vec<u16> = std::ffi::OsStr::new(title)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let message_w: Vec<u16> = std::ffi::OsStr::new(message)
            .encode_wide()
            .chain(Some(0))
            .collect();
        unsafe {
            MessageBoxW(
                std::ptr::null(),
                message_w.as_ptr(),
                title_w.as_ptr(),
                0x00000010 | 0x00000000,
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" with title \"{}\" buttons {{\"OK\"}} default button \"OK\" with icon stop",
            message.replace("\"", "\\\""),
            title.replace("\"", "\\\"")
        );
        let _ = Command::new("osascript").arg("-e").arg(script).output();
    }
}

fn ensure_config() -> Result<Config, String> {
    let path = config_path();
    if path.exists() {
        let config = Config::load()?;
        if config.base_token.trim().is_empty() {
            return Err(format!(
                "base_token 不能为空，请填写配置文件：{}",
                path.display()
            ));
        }
        if config.table_id.trim().is_empty() {
            return Err(format!(
                "table_id 不能为空，请填写配置文件：{}",
                path.display()
            ));
        }
        if config.profile.trim().is_empty() {
            return Err(format!(
                "profile 不能为空，请填写配置文件：{}",
                path.display()
            ));
        }
        return Ok(config);
    }

    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {}", e))?;
    // Write a fully-populated config with bundled defaults so the app works
    // out-of-the-box on first install (no manual file editing required).
    let template = serde_json::json!({
        "base_token": DEFAULT_BASE_TOKEN,
        "table_id": DEFAULT_TABLE_ID,
        "profile": DEFAULT_PROFILE,
        "favorites_table_id": DEFAULT_FAVORITES_TABLE_ID,
        "tags_table_id": DEFAULT_TAGS_TABLE_ID
    });
    let content =
        serde_json::to_string_pretty(&template).map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, &content).map_err(|e| format!("写入配置文件失败: {}", e))?;
    log(&format!("首次启动，已创建默认配置: {}", path.display()));

    serde_json::from_str(&content).map_err(|e| format!("解析默认配置失败: {}", e))
}

fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn tmp_json_dir() -> PathBuf {
    let dir = if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(".voice-todo-float-tmp")
    } else {
        config_dir().join("tmp")
    };
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn tmp_json_path(prefix: &str) -> (PathBuf, String) {
    let dir = tmp_json_dir();
    std::fs::create_dir_all(&dir).ok();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file = dir.join(format!("{}_{}.json", prefix, ts));
    let rel = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}_{}.json", prefix, ts));
    (file, rel)
}

struct TmpGuard {
    path: PathBuf,
}

impl TmpGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct AppState {
    main_window: Mutex<Option<tauri::WebviewWindow>>,
    config: Config,
    expanded_width: AtomicU32,
    expanded_height: AtomicU32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Task {
    id: String,
    name: String,
    status: String,
    deadline: Option<String>,
    priority: String,
    note: String,
    link: String,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "completed_at")]
    completed_at: Option<String>,
    #[serde(rename = "type")]
    task_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Favorite {
    id: String,
    name: String,
    link: String,
    description: String,
    tags: Vec<String>,
    category: String,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
    #[serde(rename = "重要程度")]
    priority: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Tag {
    id: String,
    name: String,
    category: String,
    color: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LarkResponse<T> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<LarkError>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LarkError {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RecordListData {
    data: Vec<Vec<serde_json::Value>>,
    fields: Vec<String>,
    #[serde(rename = "record_id_list")]
    record_id_list: Vec<String>,
}

#[allow(dead_code)]
fn parse_single_record_response(id: &str, output: &str) -> Result<Task, String> {
    let value: serde_json::Value = serde_json::from_str(output)
        .map_err(|e| format!("解析 record-get 响应失败: {}\n{}", e, output))?;

    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if !ok {
        let msg = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("获取记录失败");
        return Err(msg.to_string());
    }

    let data = value.get("data").ok_or("record-get 返回空数据")?;

    // Try { record: { fields: {...}, record_id: ... } } first
    if let Some(record) = data.get("record") {
        let record_id = record
            .get("record_id")
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string();
        let fields_map: HashMap<String, serde_json::Value> = record
            .get("fields")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        return Ok(fields_map_to_task(record_id, fields_map));
    }

    // Fallback to { data: [[...]], fields: [...], record_id_list: [...] }
    let data_array = data
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("record-get 返回格式异常")?;
    let fields = data
        .get("fields")
        .and_then(|v| v.as_array())
        .ok_or("record-get 返回字段缺失")?;
    let record_id_list = data
        .get("record_id_list")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if data_array.is_empty() {
        return Err("record-get 未返回记录".to_string());
    }

    let record_id = record_id_list
        .get(0)
        .cloned()
        .unwrap_or_else(|| id.to_string());
    let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(row) = data_array.get(0) {
        if let Some(row_array) = row.as_array() {
            for (field_idx, field_name) in fields.iter().enumerate() {
                if let Some(field_name_str) = field_name.as_str() {
                    if let Some(val) = row_array.get(field_idx) {
                        fields_map.insert(field_name_str.to_string(), val.clone());
                    }
                }
            }
        }
    }

    Ok(fields_map_to_task(record_id, fields_map))
}

#[allow(dead_code)]
fn fetch_record(config: &Config, id: &str) -> Result<Task, String> {
    let args = vec![
        "--format".to_string(),
        "json".to_string(),
        "base".to_string(),
        "+record-get".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        config.table_id.clone(),
        "--record-id".to_string(),
        id.to_string(),
    ];

    let output = run_lark_cli(&config.profile, &args)?;
    parse_single_record_response(id, &output)
}

#[derive(Serialize, Debug)]
struct ApiResponse<T> {
    code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    msg: Option<String>,
}

impl<T> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            code: 0,
            data: Some(data),
            msg: None,
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self {
            code: -1,
            data: None,
            msg: Some(msg.into()),
        }
    }
}

fn fields_map_to_task(record_id: String, fields_map: HashMap<String, serde_json::Value>) -> Task {
    let get_str = |key: &str| -> String {
        fields_map
            .get(key)
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                    v.as_array()
                        .and_then(|a| a.first())
                        .and_then(|f| f.as_str().map(|s| s.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };

    let get_opt_str = |key: &str| -> Option<String> {
        fields_map.get(key).and_then(|v| {
            if v.is_string() && v.as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                v.as_str().map(|s| s.to_string())
            } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                v.as_array()
                    .and_then(|a| a.first())
                    .and_then(|f| f.as_str().map(|s| s.to_string()))
            } else {
                None
            }
        })
    };

    let _get_str_array = |key: &str| -> Vec<String> {
        fields_map
            .get(key)
            .map(|v| {
                if let Some(arr) = v.as_array() {
                    arr.iter()
                        .filter_map(|f| f.as_str().map(|s| s.to_string()))
                        .collect()
                } else if v.is_string() {
                    vec![v.as_str().unwrap_or("").to_string()]
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    };

    let deadline = get_opt_str("截止时间");
    let status = get_str("状态");
    let task_type = if deadline.is_some() {
        "scheduled"
    } else {
        "someday"
    };

    Task {
        id: record_id,
        name: get_str("任务名称"),
        status: if status.is_empty() {
            "待办".to_string()
        } else {
            status
        },
        deadline,
        priority: get_str("优先级"),
        note: get_str("备注"),
        link: get_str("链接"),
        created_at: get_opt_str("创建时间"),
        completed_at: get_opt_str("完成时间"),
        task_type: task_type.to_string(),
    }
}

fn fields_map_to_favorite(
    record_id: String,
    fields_map: HashMap<String, serde_json::Value>,
) -> Favorite {
    let get_str = |key: &str| -> String {
        fields_map
            .get(key)
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                    v.as_array()
                        .and_then(|a| a.first())
                        .and_then(|f| f.as_str().map(|s| s.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };

    let get_opt_str = |key: &str| -> Option<String> {
        fields_map.get(key).and_then(|v| {
            if v.is_string() && v.as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                v.as_str().map(|s| s.to_string())
            } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                v.as_array()
                    .and_then(|a| a.first())
                    .and_then(|f| f.as_str().map(|s| s.to_string()))
            } else {
                None
            }
        })
    };

    let get_str_array = |key: &str| -> Vec<String> {
        fields_map
            .get(key)
            .map(|v| {
                if let Some(arr) = v.as_array() {
                    arr.iter()
                        .filter_map(|f| f.as_str().map(|s| s.to_string()))
                        .collect()
                } else if let Some(s) = v.as_str() {
                    // Base 标签 列是 plain text; 新写入是逗号分隔字符串,
                    // 旧记录可能是单字符串 "AI" 或者被某些 lark-cli 版本
                    // 序列化成 "[\"AI\"]"。统一按 ",", "[", "]", 逗号, 全角逗号拆分。
                    s.trim_matches(|c| c == '[' || c == ']')
                        .split(|c: char| c == ',' || c == '，' || c == ' ' || c == '、')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\''))
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    };

    Favorite {
        id: record_id,
        name: get_str("名称"),
        link: get_str("链接"),
        description: get_str("描述"),
        tags: get_str_array("标签"),
        category: get_str("分类"),
        created_at: get_opt_str("创建时间"),
        priority: get_opt_str("重要程度"),
    }
}

fn fields_map_to_tag(record_id: String, fields_map: HashMap<String, serde_json::Value>) -> Tag {
    let get_str = |key: &str| -> String {
        fields_map
            .get(key)
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                    v.as_array()
                        .and_then(|a| a.first())
                        .and_then(|f| f.as_str().map(|s| s.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };

    Tag {
        id: record_id,
        name: get_str("标签名"),
        category: get_str("分类"),
        color: get_str("颜色"),
    }
}

fn lark_cli_name() -> &'static str {
    "lark-cli"
}

fn build_command_with_executable(path: &PathBuf) -> Command {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if cfg!(target_os = "windows") && (ext == "cmd" || ext == "bat") {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/c").arg(path);
        cmd
    } else {
        Command::new(path)
    }
}

fn find_lark_cli() -> Result<PathBuf, String> {
    if let Ok(path) = which::which(lark_cli_name()) {
        if is_lark_cli_compatible(&path) {
            return Ok(path);
        }
    }

    let home = if cfg!(target_os = "windows") {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("HOMEDRIVE").and_then(|d| {
                    std::env::var("HOMEPATH").map(|p| PathBuf::from(format!("{}{}", d, p)))
                })
            })
            .map_err(|_| "Cannot determine home directory")?
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| "Cannot determine home directory")?
    };

    let project_root = project_root();

    let mut all_paths: Vec<PathBuf> = Vec::new();

    let direct_candidates: Vec<PathBuf> = if cfg!(target_os = "windows") {
        vec![
            home.join("AppData")
                .join("Roaming")
                .join("npm")
                .join("lark-cli.cmd"),
            home.join("AppData")
                .join("Roaming")
                .join("npm")
                .join("lark-cli.exe"),
            PathBuf::from("C:\\Program Files\\nodejs\\lark-cli.cmd"),
        ]
    } else {
        vec![
            home.join(".local").join("bin").join("lark-cli"),
            PathBuf::from("/usr/local/bin/lark-cli"),
            PathBuf::from("/opt/homebrew/bin/lark-cli"),
            PathBuf::from("/tmp/npm-global/lib/node_modules/@larksuite/cli/bin/lark-cli"),
            project_root
                .join("lark-deps")
                .join("node_modules")
                .join("@larksuite")
                .join("cli")
                .join("bin")
                .join("lark-cli"),
        ]
    };
    all_paths.extend(direct_candidates);

    let glob_patterns: Vec<String> = if cfg!(target_os = "windows") {
        vec![]
    } else {
        vec![
            home.join(".nvm")
                .join("versions")
                .join("node")
                .join("*")
                .join("bin")
                .join("lark-cli")
                .to_string_lossy()
                .to_string(),
            home.join(".npm-cache")
                .join("_npx")
                .join("*")
                .join("node_modules")
                .join("@larksuite")
                .join("cli")
                .join("bin")
                .join("lark-cli")
                .to_string_lossy()
                .to_string(),
        ]
    };

    for pattern in glob_patterns {
        for entry in glob::glob(&pattern).map_err(|e| format!("glob error: {}", e))? {
            if let Ok(path) = entry {
                all_paths.push(path);
            }
        }
    }

    let mut best: Option<(PathBuf, Vec<u32>)> = None;
    for path in all_paths {
        if !path.exists() {
            continue;
        }
        if let Some(version) = get_lark_cli_version(&path) {
            let better = match &best {
                None => true,
                Some((_, v)) => version > *v,
            };
            if better {
                best = Some((path, version));
            }
        }
    }

    if let Some((path, _)) = best {
        return Ok(path);
    }

    Err(format!(
        "lark-cli 不可用。请先安装并登录：lark-cli auth login\n\n安装方法：npm install -g @larksuite/cli"
    ))
}

fn get_lark_cli_version(path: &PathBuf) -> Option<Vec<u32>> {
    use std::process::Stdio;
    use wait_timeout::ChildExt;

    let mut cmd = build_command_with_executable(path);
    cmd.arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().ok()?;
    match child.wait_timeout(Duration::from_secs(10)).ok()?? {
        status if !status.success() => None,
        _ => {
            let stdout = child
                .stdout
                .take()
                .and_then(|mut out| {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut out, &mut s)
                        .ok()
                        .map(|_| s)
                })
                .unwrap_or_default();
            parse_version(&stdout)
        }
    }
}

fn parse_version(text: &str) -> Option<Vec<u32>> {
    let digits_replaced = text
        .chars()
        .map(|c| {
            if c.is_ascii_digit() || c == '.' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();
    for token in digits_replaced.split_whitespace() {
        if token.matches('.').count() == 2 {
            let parts: Vec<u32> = token.split('.').map(|p| p.parse().unwrap_or(0)).collect();
            if parts.len() == 3 {
                return Some(parts);
            }
        }
    }
    None
}

fn is_lark_cli_compatible(path: &PathBuf) -> bool {
    match get_lark_cli_version(path) {
        Some(v) => v >= vec![1, 0, 68],
        None => false,
    }
}

fn run_lark_cli(profile: &str, args: &[String]) -> Result<String, String> {
    use std::process::Stdio;
    use wait_timeout::ChildExt;

    let cli = find_lark_cli()?;
    let mut full_args = vec!["--profile".to_string(), profile.to_string()];
    full_args.extend_from_slice(args);
    log(&format!(
        "运行 lark-cli: {} {}",
        cli.display(),
        full_args.join(" ")
    ));

    let mut cmd = build_command_with_executable(&cli);
    cmd.current_dir(tmp_json_dir())
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("运行 lark-cli 失败: {}", e))?;

    let status = match child
        .wait_timeout(Duration::from_secs(30))
        .map_err(|e| format!("等待 lark-cli 失败: {}", e))?
    {
        Some(s) => s,
        None => {
            let _ = child.kill();
            return Err(
                "运行 lark-cli 超时（30秒）。请检查 lark-cli 是否已登录：lark-cli auth login"
                    .to_string(),
            );
        }
    };

    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut out, &mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = std::io::Read::read_to_string(&mut err, &mut stderr);
    }

    log(&format!(
        "lark-cli 返回: success={}, stdout={:?}, stderr={:?}",
        status.success(),
        stdout,
        stderr
    ));

    if !status.success() {
        return Err(format!("lark-cli 错误: {}\n{}", stderr, stdout));
    }

    if stdout.trim().is_empty() {
        return Ok("{}".to_string());
    }

    Ok(stdout)
}

#[tauri::command]
fn get_tasks(state: tauri::State<AppState>) -> ApiResponse<Vec<Task>> {
    let config = &state.config;
    let args = vec![
        "--format".to_string(),
        "json".to_string(),
        "base".to_string(),
        "+record-list".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        config.table_id.clone(),
        "--limit".to_string(),
        "200".to_string(),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<RecordListData> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "获取任务失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(vec![]),
    };

    let mut tasks = Vec::new();
    for (idx, row) in raw.data.iter().enumerate() {
        let record_id = raw.record_id_list.get(idx).cloned().unwrap_or_default();
        let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
        tasks.push(fields_map_to_task(record_id, fields_map));
    }

    ApiResponse::ok(tasks)
}

#[tauri::command]
fn get_favorites(state: tauri::State<AppState>) -> ApiResponse<Vec<Favorite>> {
    let config = &state.config;
    let table_id = match config.favorites_table_id() {
        Some(id) => id,
        None => return ApiResponse::err("未配置锦囊表 ID"),
    };

    let args = vec![
        "--format".to_string(),
        "json".to_string(),
        "base".to_string(),
        "+record-list".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id.clone(),
        "--limit".to_string(),
        "200".to_string(),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<RecordListData> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "获取锦囊失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(vec![]),
    };

    let mut favorites = Vec::new();
    for (idx, row) in raw.data.iter().enumerate() {
        let record_id = raw.record_id_list.get(idx).cloned().unwrap_or_default();
        let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
        favorites.push(fields_map_to_favorite(record_id, fields_map));
    }

    ApiResponse::ok(favorites)
}

#[tauri::command]
fn get_tags(state: tauri::State<AppState>) -> ApiResponse<Vec<Tag>> {
    let config = &state.config;
    let table_id = match config.tags_table_id() {
        Some(id) => id,
        None => return ApiResponse::ok(vec![]),
    };

    let args = vec![
        "--format".to_string(),
        "json".to_string(),
        "base".to_string(),
        "+record-list".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id.clone(),
        "--limit".to_string(),
        "200".to_string(),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<RecordListData> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "获取标签失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(vec![]),
    };

    let mut tags = Vec::new();
    for (idx, row) in raw.data.iter().enumerate() {
        let record_id = raw.record_id_list.get(idx).cloned().unwrap_or_default();
        let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
        tags.push(fields_map_to_tag(record_id, fields_map));
    }

    ApiResponse::ok(tags)
}

#[tauri::command]
fn create_favorite(
    state: tauri::State<AppState>,
    name: String,
    link: String,
    description: String,
    category: String,
    tags: Vec<String>,
    priority: Option<String>,
) -> ApiResponse<serde_json::Value> {
    if name.trim().is_empty() {
        return ApiResponse::err("名称不能为空");
    }
    let config = &state.config;
    let table_id = match config.favorites_table_id() {
        Some(id) => id,
        None => return ApiResponse::err("未配置锦囊表 ID"),
    };

    let mut fields = vec!["名称", "链接", "描述"];
    let mut row: Vec<serde_json::Value> = vec![
        name.trim().into(),
        link.trim().into(),
        description.trim().into(),
    ];

    if !category.trim().is_empty() {
        fields.push("分类");
        row.push(category.trim().into());
    }

    if let Some(p) = priority {
        if !p.is_empty() {
            fields.push("重要程度");
            row.push(p.into());
        }
    }

    if !tags.is_empty() {
        fields.push("标签");
        row.push(tags.join(",").into());
    }

    let json_data = serde_json::json!({ "fields": fields, "rows": [row] });
    let (json_file, path_str) = tmp_json_path("fav_batch");

    if let Err(e) = std::fs::write(&json_file, json_data.to_string()) {
        return ApiResponse::err(format!("写入临时文件失败: {}", e));
    }
    let _guard = TmpGuard::new(json_file.clone());

    let args = vec![
        "base".to_string(),
        "+record-batch-create".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id.clone(),
        "--json".to_string(),
        format!("@{}", path_str),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<RecordListData> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "创建失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(serde_json::json!({ "created": true })),
    };

    let record_id = raw.record_id_list.get(0).cloned().unwrap_or_default();
    if record_id.is_empty() {
        return ApiResponse::ok(serde_json::json!({ "created": true }));
    }

    let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(first_row) = raw.data.get(0) {
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = first_row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
    }
    let favorite = fields_map_to_favorite(record_id, fields_map);
    match serde_json::to_value(favorite) {
        Ok(v) => ApiResponse::ok(v),
        Err(_) => ApiResponse::ok(serde_json::json!({ "created": true })),
    }
}

#[tauri::command]
fn update_favorite(
    state: tauri::State<AppState>,
    id: String,
    name: String,
    link: String,
    description: String,
    category: String,
    tags: Vec<String>,
    priority: Option<String>,
) -> ApiResponse<serde_json::Value> {
    let config = &state.config;
    let table_id = match config.favorites_table_id() {
        Some(id) => id,
        None => return ApiResponse::err("未配置锦囊表 ID"),
    };

    let mut payload_map: HashMap<String, serde_json::Value> = HashMap::new();
    payload_map.insert("名称".to_string(), name.trim().into());
    payload_map.insert("链接".to_string(), link.trim().into());
    payload_map.insert("描述".to_string(), description.trim().into());
    if !category.trim().is_empty() {
        payload_map.insert("分类".to_string(), category.trim().into());
    }
    if let Some(p) = priority {
        if !p.is_empty() {
            payload_map.insert("重要程度".to_string(), p.into());
        }
    }
    if !tags.is_empty() {
        payload_map.insert("标签".to_string(), tags.join(",").into());
    }

    log(&format!(
        "update_favorite 开始: record_id={}, payload={}",
        id,
        serde_json::to_string(&payload_map).unwrap_or_default()
    ));

    let (json_file, path_str) = tmp_json_path("fav_upsert");

    if let Err(e) = std::fs::write(
        &json_file,
        serde_json::to_string(&payload_map).unwrap_or_default(),
    ) {
        return ApiResponse::err(format!("写入临时文件失败: {}", e));
    }
    let _guard = TmpGuard::new(json_file.clone());

    let args = vec![
        "base".to_string(),
        "+record-upsert".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id.clone(),
        "--record-id".to_string(),
        id.clone(),
        "--json".to_string(),
        format!("@{}", path_str),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    log(&format!(
        "update_favorite upsert 响应: record_id={}, output={}",
        id, output
    ));

    let resp: LarkResponse<serde_json::Value> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "更新失败".to_string()),
        );
    }

    ApiResponse::ok(serde_json::json!({ "updated": true }))
}

#[tauri::command]
fn delete_favorite(state: tauri::State<AppState>, id: String) -> ApiResponse<serde_json::Value> {
    let config = &state.config;
    let table_id = match config.favorites_table_id() {
        Some(id) => id,
        None => return ApiResponse::err("未配置锦囊表 ID"),
    };

    let args = vec![
        "base".to_string(),
        "+record-delete".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id.clone(),
        "--record-id".to_string(),
        id,
        "--yes".to_string(),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<serde_json::Value> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "删除失败".to_string()),
        );
    }

    ApiResponse::ok(serde_json::json!({}))
}

#[tauri::command]
fn create_task(
    state: tauri::State<AppState>,
    name: String,
    deadline: Option<String>,
    priority: String,
) -> ApiResponse<serde_json::Value> {
    if name.trim().is_empty() {
        return ApiResponse::err("任务名称不能为空");
    }
    let config = &state.config;

    let mut fields = vec!["任务名称", "状态", "优先级"];
    let mut row: Vec<serde_json::Value> = vec![name.trim().into(), "待办".into(), priority.into()];

    if let Some(d) = deadline {
        if !d.is_empty() {
            fields.push("截止时间");
            row.push(d.into());
        }
    }

    let json_data = serde_json::json!({ "fields": fields, "rows": [row] });
    let (json_file, path_str) = tmp_json_path("batch");

    if let Err(e) = std::fs::write(&json_file, json_data.to_string()) {
        return ApiResponse::err(format!("写入临时文件失败: {}", e));
    }
    let _guard = TmpGuard::new(json_file.clone());

    let args = vec![
        "base".to_string(),
        "+record-batch-create".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        config.table_id.clone(),
        "--json".to_string(),
        format!("@{}", path_str),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<RecordListData> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "创建失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(serde_json::json!({ "created": true })),
    };

    let record_id = raw.record_id_list.get(0).cloned().unwrap_or_default();
    if record_id.is_empty() {
        return ApiResponse::ok(serde_json::json!({ "created": true }));
    }

    let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
    if let Some(first_row) = raw.data.get(0) {
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = first_row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
    }
    let task = fields_map_to_task(record_id, fields_map);
    match serde_json::to_value(task) {
        Ok(v) => ApiResponse::ok(v),
        Err(_) => ApiResponse::ok(serde_json::json!({ "created": true })),
    }
}

#[tauri::command]
fn update_task(
    state: tauri::State<AppState>,
    mut payload: std::collections::HashMap<String, serde_json::Value>,
) -> ApiResponse<serde_json::Value> {
    let id = match payload.remove("id") {
        Some(serde_json::Value::String(s)) => s,
        _ => return ApiResponse::err("缺少记录 ID"),
    };
    let config = &state.config;

    log(&format!(
        "update_task 开始: record_id={}, payload={}",
        id,
        serde_json::to_string(&payload).unwrap_or_default()
    ));

    let (json_file, path_str) = tmp_json_path("upsert");

    if let Err(e) = std::fs::write(
        &json_file,
        serde_json::to_string(&payload).unwrap_or_default(),
    ) {
        return ApiResponse::err(format!("写入临时文件失败: {}", e));
    }
    let _guard = TmpGuard::new(json_file.clone());

    let args = vec![
        "base".to_string(),
        "+record-upsert".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        config.table_id.clone(),
        "--record-id".to_string(),
        id.clone(),
        "--json".to_string(),
        format!("@{}", path_str),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    log(&format!(
        "update_task upsert 响应: record_id={}, output={}",
        id, output
    ));

    let resp: LarkResponse<serde_json::Value> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "更新失败".to_string()),
        );
    }

    // 不立即拉取：飞书多维表缓存导致刚保存的值可能被立即获取时的旧值覆盖，
    // 前端已经优化更新并会延迟刷新，这里只返回成功即可
    ApiResponse::ok(serde_json::json!({ "updated": true }))
}

#[tauri::command]
fn delete_task(state: tauri::State<AppState>, id: String) -> ApiResponse<serde_json::Value> {
    let config = &state.config;
    let args = vec![
        "base".to_string(),
        "+record-delete".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        config.table_id.clone(),
        "--record-id".to_string(),
        id,
        "--yes".to_string(),
    ];

    let output = match run_lark_cli(&config.profile, &args) {
        Ok(o) => o,
        Err(e) => return ApiResponse::err(e),
    };

    let resp: LarkResponse<serde_json::Value> = match serde_json::from_str(&output) {
        Ok(r) => r,
        Err(e) => return ApiResponse::err(format!("解析响应失败: {}\n{}", e, output)),
    };

    if !resp.ok {
        return ApiResponse::err(
            resp.error
                .map(|e| e.message)
                .unwrap_or_else(|| "删除失败".to_string()),
        );
    }

    ApiResponse::ok(serde_json::json!({}))
}

#[tauri::command]
fn toggle_collapse(state: tauri::State<AppState>, collapsed: bool) -> Result<(), String> {
    let window = state.main_window.lock().map_err(|e| e.to_string())?;
    if let Some(w) = window.as_ref() {
        if collapsed {
            // Save current logical size before collapsing
            if let Ok(size) = w.inner_size() {
                let size: tauri::LogicalSize<f64> =
                    size.to_logical(w.scale_factor().unwrap_or(1.0));
                state
                    .expanded_width
                    .store(size.width as u32, Ordering::Relaxed);
                state
                    .expanded_height
                    .store(size.height as u32, Ordering::Relaxed);
            }
            w.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: 360.0,
                height: 52.0,
            }))
            .map_err(|e| e.to_string())?;
        } else {
            let width = state.expanded_width.load(Ordering::Relaxed).max(360) as f64;
            let height = state.expanded_height.load(Ordering::Relaxed).max(500) as f64;
            w.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn set_always_on_top(state: tauri::State<AppState>, value: bool) -> Result<(), String> {
    let window = state.main_window.lock().map_err(|e| e.to_string())?;
    if let Some(w) = window.as_ref() {
        w.set_always_on_top(value).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn minimize_window(state: tauri::State<AppState>) -> Result<(), String> {
    let window = state.main_window.lock().map_err(|e| e.to_string())?;
    if let Some(w) = window.as_ref() {
        w.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn close_window(state: tauri::State<AppState>) -> Result<(), String> {
    let window = state.main_window.lock().map_err(|e| e.to_string())?;
    if let Some(w) = window.as_ref() {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn start_dragging(state: tauri::State<AppState>) -> Result<(), String> {
    let window = state.main_window.lock().map_err(|e| e.to_string())?;
    if let Some(w) = window.as_ref() {
        w.start_dragging().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let normalized = normalize_url(url);
    app.opener()
        .open_url(&normalized, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn normalize_url(url: String) -> String {
    let url = url.trim();
    if url.is_empty() {
        return url.to_string();
    }

    // Already has a protocol (file://, http://, https://, etc.)
    if url.contains("://") {
        return url.to_string();
    }

    // Windows absolute path: D:\... or D:/...
    if let Some(c) = url.chars().next() {
        if c.is_ascii_alphabetic() && url.len() >= 2 && url.as_bytes()[1] == b':' {
            let path = url.replace('\\', "/");
            // Don't URL-encode: Windows shell can't decode file:// URLs with %20/%XX
            return format!("file:///{}", path);
        }
    }

    // Unix absolute path
    if url.starts_with('/') {
        return format!("file://{}", url);
    }

    // Default: treat as web URL with https
    format!("https://{}", url)
}

fn main() {
    log("应用启动");
    let config = match ensure_config() {
        Ok(c) => c,
        Err(e) => {
            log(&format!("配置加载失败: {}", e));
            eprintln!("配置加载失败: {}", e);
            show_error_dialog("一纸待办 - 配置错误", &e);
            std::process::exit(1);
        }
    };

    let app_config = config.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let window = app
                .get_webview_window("main")
                .expect("main window not found");

            #[cfg(target_os = "macos")]
            {
                use objc2::msg_send;
                use objc2::runtime::AnyObject;
                if let Ok(ns_window) = window.ns_window() {
                    let ns_window = ns_window as *mut AnyObject;
                    unsafe {
                        let _: () = msg_send![ns_window, setMovable: true];
                        let _: () = msg_send![ns_window, setMovableByWindowBackground: false];
                    }
                }
            }

            let _ = window.set_focus();
            app.manage(AppState {
                main_window: Mutex::new(Some(window)),
                config: app_config.clone(),
                expanded_width: AtomicU32::new(360),
                expanded_height: AtomicU32::new(500),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            create_task,
            update_task,
            delete_task,
            get_favorites,
            create_favorite,
            update_favorite,
            delete_favorite,
            get_tags,
            toggle_collapse,
            set_always_on_top,
            minimize_window,
            close_window,
            start_dragging,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
