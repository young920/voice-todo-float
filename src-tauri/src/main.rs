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
    #[serde(rename = "anniversary_table_id")]
    anniversary_table_id: Option<String>,
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
const DEFAULT_ANNIVERSARY_TABLE_ID: &str = "tblAt0NJ5gaMYiFB";

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
        if config
            .anniversary_table_id
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            config.anniversary_table_id = Some(DEFAULT_ANNIVERSARY_TABLE_ID.to_string());
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

    fn anniversary_table_id(&self) -> Option<&String> {
        self.anniversary_table_id
            .as_ref()
            .filter(|s| !s.trim().is_empty())
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
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    if let Err(_e) = result {
        // silently ignore - sandbox or perm denied, app still works
    }
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
        "tags_table_id": DEFAULT_TAGS_TABLE_ID,
        "anniversary_table_id": DEFAULT_ANNIVERSARY_TABLE_ID
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Anniversary {
    id: String,
    name: String,
    #[serde(rename = "date_type")]
    date_type: String,
    month: Option<i64>,
    day: Option<i64>,
    #[serde(rename = "type")]
    ann_type: String,
    note: String,
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

fn fields_map_to_anniversary(
    record_id: String,
    fields_map: HashMap<String, serde_json::Value>,
) -> Anniversary {
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
    let get_num = |key: &str| -> Option<i64> {
        fields_map.get(key).and_then(|v| {
            if v.is_number() {
                v.as_i64()
            } else if v.is_array() && v.as_array().map(|a| a.len()) == Some(1) {
                v.as_array()
                    .and_then(|a| a.first())
                    .and_then(|f| f.as_i64())
            } else {
                None
            }
        })
    };

    Anniversary {
        id: record_id,
        name: get_str("名称"),
        date_type: get_str("日期类型"),
        month: get_num("月份"),
        day: get_num("日期"),
        ann_type: get_str("类型"),
        note: get_str("备注"),
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
    let mut cmd = if cfg!(target_os = "windows") && (ext == "cmd" || ext == "bat") {
        let mut c = Command::new("cmd.exe");
        c.arg("/c").arg(path);
        c
    } else {
        Command::new(path)
    };
    // lark-cli auto-detects Agent context from HERMES_*/OPENCLAW_HOME/LARK_CHANNEL
    // env vars and refuses to run with "not bound" if set. Strip these so the
    // spawned lark-cli uses the bot token directly without Agent context.
    for key in [
        "HERMES_HOME", "HERMES_AGENT_TIMEOUT", "HERMES_AGENT_NOTIFY_INTERVAL",
        "HERMES_REDACT_SECRETS", "HERMES_SESSION_KEY", "HERMES_EXEC_ASK",
        "HERMES_AGENT_TIMEOUT_WARNING", "HERMES_RESTART_DRAIN_TIMEOUT",
        "HERMES_QUIET", "HERMES_GATEWAY_TOKEN",
        "OPENCLAW_HOME", "LARK_CHANNEL",
    ] {
        cmd.env_remove(key);
    }
    cmd
}

fn find_lark_cli() -> Result<PathBuf, String> {
    // Prefer the hermes wrapper if installed. It strips HERMES_* env vars
    // (so lark-cli doesn't force the hermes context bind path) and chowns
    // keychain files back to the current user if they were touched by
    // a root session.
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        if let Ok(home) = std::env::var("HOME") {
            let wrapper = PathBuf::from(home)
                .join(".hermes")
                .join("scripts")
                .join("voice-todo-float")
                .join("lark-cli-wrapper.sh");
            if wrapper.exists() {
                if let Ok(meta) = std::fs::metadata(&wrapper) {
                    if meta.len() > 0 {
                        return Ok(wrapper);
                    }
                }
            }
        }
    }

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
fn get_anniversaries(state: tauri::State<AppState>) -> ApiResponse<Vec<Anniversary>> {
    let config = &state.config;
    let table_id = match config.anniversary_table_id() {
        Some(t) => t.clone(),
        None => return ApiResponse::err("未配置纪念日表"),
    };
    let args = vec![
        "--format".to_string(),
        "json".to_string(),
        "base".to_string(),
        "+record-list".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id,
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
                .unwrap_or_else(|| "获取纪念日失败".to_string()),
        );
    }

    let raw = match resp.data {
        Some(d) => d,
        None => return ApiResponse::ok(vec![]),
    };

    let mut anniversaries = Vec::new();
    for (idx, row) in raw.data.iter().enumerate() {
        let record_id = raw.record_id_list.get(idx).cloned().unwrap_or_default();
        let mut fields_map: HashMap<String, serde_json::Value> = HashMap::new();
        for (field_idx, field_name) in raw.fields.iter().enumerate() {
            if let Some(val) = row.get(field_idx) {
                fields_map.insert(field_name.clone(), val.clone());
            }
        }
        anniversaries.push(fields_map_to_anniversary(record_id, fields_map));
    }

    ApiResponse::ok(anniversaries)
}

#[tauri::command]
fn create_anniversary(
    state: tauri::State<AppState>,
    name: String,
    date_type: String,
    month: Option<i64>,
    day: Option<i64>,
    ann_type: String,
    note: Option<String>,
) -> ApiResponse<serde_json::Value> {
    if name.trim().is_empty() {
        return ApiResponse::err("纪念日名称不能为空");
    }
    let config = &state.config;
    let table_id = match config.anniversary_table_id() {
        Some(t) => t.clone(),
        None => return ApiResponse::err("未配置纪念日表"),
    };

    let mut fields = vec!["名称", "日期类型", "类型"];
    let mut row: Vec<serde_json::Value> = vec![
        name.trim().into(),
        date_type.into(),
        ann_type.into(),
    ];
    if let Some(m) = month {
        fields.push("月份");
        row.push(m.into());
    }
    if let Some(d) = day {
        fields.push("日期");
        row.push(d.into());
    }
    if let Some(n) = note {
        if !n.trim().is_empty() {
            fields.push("备注");
            row.push(n.trim().into());
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
        table_id,
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
                .unwrap_or_else(|| "创建纪念日失败".to_string()),
        );
    }

    ApiResponse::ok(serde_json::json!({ "created": true }))
}

#[tauri::command]
fn delete_anniversary(state: tauri::State<AppState>, id: String) -> ApiResponse<serde_json::Value> {
    let config = &state.config;
    let table_id = match config.anniversary_table_id() {
        Some(t) => t.clone(),
        None => return ApiResponse::err("未配置纪念日表"),
    };
    let args = vec![
        "base".to_string(),
        "+record-delete".to_string(),
        "--base-token".to_string(),
        config.base_token.clone(),
        "--table-id".to_string(),
        table_id,
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
                .unwrap_or_else(|| "删除纪念日失败".to_string()),
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

// ─────────────────────────────────────────────────────────────
// 模型测试模块
// ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Gateway {
    name: String,
    #[serde(rename = "openai_base")]
    openai_base: String,
    #[serde(rename = "anthropic_base")]
    anthropic_base: String,
    #[serde(rename = "api_key")]
    api_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ModelTestConfig {
    gateways: Vec<Gateway>,
    active: String,
}

impl Default for ModelTestConfig {
    fn default() -> Self {
        Self {
            gateways: vec![Gateway {
                name: "清晓云".to_string(),
                openai_base: "https://ai.qingxiaoyun.net/v1".to_string(),
                anthropic_base: "https://ai.qingxiaoyun.net".to_string(),
                api_key: "sk-YaP0IMQvlU5Wu3kfZUgTxA9YnoQO0IwbQixMNcYzKYle2sET".to_string(),
            }],
            active: "清晓云".to_string(),
        }
    }
}

fn model_test_config_path() -> PathBuf {
    config_dir().join("model_test.json")
}

fn load_model_test_config() -> ModelTestConfig {
    let path = model_test_config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        // 优先解析新结构（gateways 列表）
        if let Ok(cfg) = serde_json::from_str::<ModelTestConfig>(&content) {
            if !cfg.gateways.is_empty() {
                return cfg;
            }
        }
        // 兼容旧结构（单组 openai_base/anthropic_base/api_key）→ 迁移成默认网关
        #[derive(Deserialize)]
        struct Legacy {
            openai_base: Option<String>,
            anthropic_base: Option<String>,
            api_key: Option<String>,
        }
        if let Ok(old) = serde_json::from_str::<Legacy>(&content) {
            let mut cfg = ModelTestConfig::default();
            if let Some(g) = cfg.gateways.first_mut() {
                if let Some(v) = old.openai_base { g.openai_base = v; }
                if let Some(v) = old.anthropic_base { g.anthropic_base = v; }
                if let Some(v) = old.api_key { g.api_key = v; }
            }
            let _ = save_model_test_config(&cfg);
            return cfg;
        }
    }
    ModelTestConfig::default()
}

fn save_model_test_config(cfg: &ModelTestConfig) -> Result<(), String> {
    let path = model_test_config_path();
    let content = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_model_test_config() -> ModelTestConfig {
    load_model_test_config()
}

// 保存当前选中网关的地址/key（更新配置）
#[tauri::command]
fn save_model_test_config_cmd(
    openai_base: String,
    anthropic_base: String,
    api_key: String,
) -> Result<(), String> {
    let mut cfg = load_model_test_config();
    let active = cfg.active.clone();
    if let Some(g) = cfg.gateways.iter_mut().find(|g| g.name == active) {
        g.openai_base = openai_base;
        g.anthropic_base = anthropic_base;
        g.api_key = api_key;
    } else if let Some(g) = cfg.gateways.first_mut() {
        g.openai_base = openai_base;
        g.anthropic_base = anthropic_base;
        g.api_key = api_key;
    }
    save_model_test_config(&cfg)
}

// 添加网关（name 已存在则覆盖）
#[tauri::command]
fn add_gateway_cmd(
    name: String,
    openai_base: String,
    anthropic_base: String,
    api_key: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("网关名称不能为空".to_string());
    }
    let mut cfg = load_model_test_config();
    if let Some(g) = cfg.gateways.iter_mut().find(|g| g.name == name) {
        g.openai_base = openai_base;
        g.anthropic_base = anthropic_base;
        g.api_key = api_key;
    } else {
        cfg.gateways.push(Gateway {
            name,
            openai_base,
            anthropic_base,
            api_key,
        });
    }
    save_model_test_config(&cfg)
}

// 删除网关（不能删最后一个）
#[tauri::command]
fn delete_gateway_cmd(name: String) -> Result<(), String> {
    let mut cfg = load_model_test_config();
    if cfg.gateways.len() <= 1 {
        return Err("至少保留一个网关".to_string());
    }
    cfg.gateways.retain(|g| g.name != name);
    // 如果删的是当前选中，切到第一个
    if !cfg.gateways.iter().any(|g| g.name == cfg.active) {
        if let Some(g) = cfg.gateways.first() {
            cfg.active = g.name.clone();
        }
    }
    save_model_test_config(&cfg)
}

// 重命名网关
#[tauri::command]
fn rename_gateway_cmd(old_name: String, new_name: String) -> Result<(), String> {
    let new_name = new_name.trim().to_string();
    let old_name = old_name.trim().to_string();
    if new_name.is_empty() {
        return Err("网关名称不能为空".to_string());
    }
    if new_name == old_name {
        return Ok(());
    }
    let mut cfg = load_model_test_config();
    if !cfg.gateways.iter().any(|g| g.name == old_name) {
        return Err(format!("网关不存在: {}", old_name));
    }
    if cfg.gateways.iter().any(|g| g.name == new_name) {
        return Err(format!("网关名称已存在: {}", new_name));
    }
    if let Some(g) = cfg.gateways.iter_mut().find(|g| g.name == old_name) {
        g.name = new_name.clone();
    }
    // 如果改的是当前选中，active 跟着改
    if cfg.active == old_name {
        cfg.active = new_name;
    }
    save_model_test_config(&cfg)
}

// 切换当前选中网关
#[tauri::command]
fn select_gateway_cmd(name: String) -> Result<(), String> {
    let mut cfg = load_model_test_config();
    if !cfg.gateways.iter().any(|g| g.name == name) {
        return Err(format!("网关不存在: {}", name));
    }
    cfg.active = name;
    save_model_test_config(&cfg)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ModelInfo {
    id: String,
    #[serde(rename = "owned_by")]
    owned_by: String,
    #[serde(rename = "supported_endpoint_types")]
    supported_endpoint_types: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Serialize, Debug)]
struct ModelTestResult {
    model: String,
    status: String, // OK / SLOW / FAIL / TIMEOUT
    time_ms: u64,
    capability: i32,   // 能力分 0-100
    speed: i32,        // 速度分 0-100
    score: i32,        // 综合分 0-100
    error: String,
    protocol: String,  // openai / anthropic
    context: i64,      // 上下文长度（硬编码）
}

// 硬编码能力评级表：模型 id 前缀 → (能力分, 上下文长度, 说明)
fn capability_rating(model: &str) -> (i32, i64) {
    let m = model.to_lowercase();
    // 顶级推理模型
    if m.contains("step-5") { return (95, 200000); }
    if m.contains("step-3.7") { return (88, 128000); }
    if m.contains("glm-5") { return (92, 128000); }
    if m.contains("minimax-m3.1") { return (90, 200000); }
    if m.contains("minimax-m3") { return (88, 128000); }
    if m.contains("minimax-m2.7") { return (82, 128000); }
    if m.contains("k3") { return (90, 200000); }
    if m.contains("kimi-k2") { return (90, 409600); }
    if m.contains("doubao-seed") { return (80, 128000); }
    if m.contains("deepseek-v4") { return (85, 128000); }
    if m.contains("qwen-3.6") { return (84, 128000); }
    if m.contains("hy3") { return (86, 128000); }
    if m.contains("agnes") { return (78, 128000); }
    // 默认
    (70, 128000)
}

// 拉取模型清单
fn fetch_models(base: &str, key: &str, protocol: &str) -> Result<Vec<ModelInfo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let url = if protocol == "anthropic" {
        format!("{}/v1/models", base)
    } else {
        format!("{}/models", base)
    };

    let mut req = client.get(&url).header("Authorization", format!("Bearer {}", key));
    if protocol == "anthropic" {
        req = req.header("anthropic-version", "2023-06-01");
    }

    let resp = req.send().map_err(|e| format!("请求模型列表失败: {}", e))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!("模型列表接口返回 {}: {}", status, &text[..text.len().min(200)]));
    }

    let parsed: ModelsResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析模型列表失败: {}", e))?;
    Ok(parsed.data)
}

// 探测单个模型（OpenAI 协议）
fn probe_openai(base: &str, key: &str, model: &str) -> Result<(u128, bool), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let url = format!("{}/chat/completions", base);
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "计算 17×23 等于多少？只回答数字。"}],
        "max_tokens": 200
    });

    let t0 = std::time::Instant::now();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;
    let dt = t0.elapsed().as_millis();

    if !status.is_success() {
        let err = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| format!("HTTP {}", status));
        return Err(err);
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("解析失败: {}", e))?;
    let content = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    Ok((dt, !content.trim().is_empty()))
}

// 探测单个模型（Anthropic 协议）
fn probe_anthropic(base: &str, key: &str, model: &str) -> Result<(u128, bool), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let url = format!("{}/v1/messages", base);
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 200,
        "messages": [{"role": "user", "content": "计算 17×23 等于多少？只回答数字。"}]
    });

    let t0 = std::time::Instant::now();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .map_err(|e| format!("请求失败: {}", e))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| format!("读取响应失败: {}", e))?;
    let dt = t0.elapsed().as_millis();

    if !status.is_success() {
        let err = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| format!("HTTP {}", status));
        return Err(err);
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("解析失败: {}", e))?;
    let content = parsed
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    Ok((dt, !content.trim().is_empty()))
}

// 过滤掉非聊天模型
fn is_chat_model(m: &ModelInfo) -> bool {
    let id = m.id.to_lowercase();
    if id.contains("embedding") || id.contains("codex") { return false; }
    if id.contains("hailuo") || id.contains("image") || id.contains("tts") { return false; }
    if m.owned_by.contains("task plugin") { return false; }
    true
}

#[tauri::command]
fn test_models(
    openai_base: String,
    anthropic_base: String,
    api_key: String,
    protocol: String,
) -> Result<Vec<ModelTestResult>, String> {
    let protocol = if protocol == "anthropic" { "anthropic" } else { "openai" };
    let base = if protocol == "anthropic" { &anthropic_base } else { &openai_base };

    // 拉取模型清单
    let models = fetch_models(base, &api_key, protocol)?;

    // 过滤聊天模型
    let chat_models: Vec<&ModelInfo> = models.iter().filter(|m| is_chat_model(m)).collect();

    // 并发探测
    let mut results: Vec<ModelTestResult> = Vec::new();
    let handles: Vec<_> = chat_models
        .iter()
        .map(|m| {
            let base = base.clone();
            let key = api_key.clone();
            let model_id = m.id.clone();
            let protocol = protocol.to_string();
            std::thread::spawn(move || {
                let probe = if protocol == "anthropic" {
                    probe_anthropic(&base, &key, &model_id)
                } else {
                    probe_openai(&base, &key, &model_id)
                };

                let (capability, context) = capability_rating(&model_id);

                match probe {
                    Ok((dt, has_content)) => {
                        let time_ms = dt as u64;
                        // 速度分：<1s=100, 1-2s=90, 2-3s=80, 3-5s=60, >5s=40
                        let speed = if time_ms < 1000 { 100 }
                            else if time_ms < 2000 { 90 }
                            else if time_ms < 3000 { 80 }
                            else if time_ms < 5000 { 60 }
                            else { 40 };
                        // 综合分：能力 50% + 速度 30% + 可用性 20%
                        let score = (capability * 50 + speed * 30 + 100 * 20) / 100;
                        ModelTestResult {
                            model: model_id,
                            status: if has_content { "OK" } else { "EMPTY" }.to_string(),
                            time_ms,
                            capability,
                            speed,
                            score,
                            error: String::new(),
                            protocol: protocol.clone(),
                            context,
                        }
                    }
                    Err(e) => {
                        ModelTestResult {
                            model: model_id,
                            status: "FAIL".to_string(),
                            time_ms: 0,
                            capability,
                            speed: 0,
                            score: 0,
                            error: e,
                            protocol: protocol.clone(),
                            context,
                        }
                    }
                }
            })
        })
        .collect();

    for h in handles {
        results.push(h.join().unwrap_or_else(|_| ModelTestResult {
            model: "unknown".to_string(),
            status: "FAIL".to_string(),
            time_ms: 0,
            capability: 0,
            speed: 0,
            score: 0,
            error: "线程异常".to_string(),
            protocol: protocol.to_string(),
            context: 0,
        }));
    }

    // 按综合分排序
    results.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(results)
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

            // Windows 必须显示在任务栏，否则最小化后无法找回。
            // tauri.conf.json 里 skipTaskbar:true 是为 macOS 菜单栏 widget 设计的；
            // Windows 没有菜单栏图标机制，窗口必须出现在任务栏。
            #[cfg(target_os = "windows")]
            {
                let _ = window.set_skip_taskbar(false);
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
            get_anniversaries,
            create_anniversary,
            delete_anniversary,
            toggle_collapse,
            set_always_on_top,
            minimize_window,
            close_window,
            start_dragging,
            open_external,
            get_model_test_config,
            save_model_test_config_cmd,
            add_gateway_cmd,
            delete_gateway_cmd,
            rename_gateway_cmd,
            select_gateway_cmd,
            test_models
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
