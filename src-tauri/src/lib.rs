use std::{collections::HashMap, env, fs, io::ErrorKind, path::PathBuf, sync::Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::State;

#[derive(Clone)]
struct Connection {
    base_url: String,
    management_key: String,
    delete_saved_credential_on_disconnect: bool,
}

struct CpaState {
    client: reqwest::Client,
    connection: Mutex<Option<Connection>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum BootstrapSource {
    InstallEnv,
    ProcessEnv,
    CredentialManager,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum BootstrapResult {
    Connected {
        source: BootstrapSource,
        #[serde(rename = "baseUrl")]
        base_url: String,
        config: Value,
    },
    NotConfigured,
    Failed {
        source: BootstrapSource,
        #[serde(rename = "baseUrl")]
        base_url: Option<String>,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartupCredentials {
    source: BootstrapSource,
    base_url: String,
    management_key: String,
}

#[derive(Debug)]
struct StartupConfigError {
    source: BootstrapSource,
    base_url: Option<String>,
    message: String,
}

const KEYRING_SERVICE: &str = "CPA Route Control";
const MANAGEMENT_URL_ENV: &str = "CPA_MANAGEMENT_URL";
const MANAGEMENT_KEY_ENV: &str = "CPA_MANAGEMENT_KEY";

fn credential_entry(base_url: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, base_url)
        .map_err(|error| format!("Windows Credential Manager error: {error}"))
}

fn normalize_base_url(value: &str) -> Result<String, String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err(format!("{MANAGEMENT_URL_ENV} 不能为空"));
    }

    let url = reqwest::Url::parse(value)
        .map_err(|_| format!("{MANAGEMENT_URL_ENV} 必须是合法的 HTTP 或 HTTPS 地址"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(format!("{MANAGEMENT_URL_ENV} 必须是合法的 HTTP 或 HTTPS 地址"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(format!("{MANAGEMENT_URL_ENV} 不能包含查询参数或片段"));
    }

    Ok(value.to_string())
}

fn credentials_from_values(
    source: BootstrapSource,
    base_url: Option<String>,
    management_key: Option<String>,
) -> Result<Option<StartupCredentials>, StartupConfigError> {
    if base_url.is_none() && management_key.is_none() {
        return Ok(None);
    }

    let raw_base_url = match base_url {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return Err(StartupConfigError {
                source,
                base_url: None,
                message: format!(
                    "{MANAGEMENT_URL_ENV} 和 {MANAGEMENT_KEY_ENV} 必须同时配置且不能为空"
                ),
            })
        }
    };
    let normalized_base_url = normalize_base_url(&raw_base_url).map_err(|message| {
        StartupConfigError {
            source,
            base_url: None,
            message,
        }
    })?;
    let management_key = match management_key {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => {
            return Err(StartupConfigError {
                source,
                base_url: Some(normalized_base_url),
                message: format!(
                    "{MANAGEMENT_URL_ENV} 和 {MANAGEMENT_KEY_ENV} 必须同时配置且不能为空"
                ),
            })
        }
    };

    Ok(Some(StartupCredentials {
        source,
        base_url: normalized_base_url,
        management_key,
    }))
}

fn install_env_path() -> Result<PathBuf, StartupConfigError> {
    let executable = env::current_exe().map_err(|_| StartupConfigError {
        source: BootstrapSource::InstallEnv,
        base_url: None,
        message: "无法确定程序安装目录，不能读取 .env".into(),
    })?;
    let directory = executable.parent().ok_or_else(|| StartupConfigError {
        source: BootstrapSource::InstallEnv,
        base_url: None,
        message: "无法确定程序安装目录，不能读取 .env".into(),
    })?;
    Ok(directory.join(".env"))
}

fn read_install_env() -> Result<Option<StartupCredentials>, StartupConfigError> {
    let path = install_env_path()?;
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(StartupConfigError {
                source: BootstrapSource::InstallEnv,
                base_url: None,
                message: "读取安装目录 .env 失败".into(),
            })
        }
    };

    let mut values = HashMap::new();
    for item in dotenvy::from_read_iter(content.as_bytes()) {
        let (key, value) = item.map_err(|_| StartupConfigError {
            source: BootstrapSource::InstallEnv,
            base_url: None,
            message: "安装目录 .env 格式无效".into(),
        })?;
        values.insert(key, value);
    }

    credentials_from_values(
        BootstrapSource::InstallEnv,
        values.remove(MANAGEMENT_URL_ENV),
        values.remove(MANAGEMENT_KEY_ENV),
    )
}

fn read_process_env_value(name: &str) -> Result<Option<String>, StartupConfigError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(StartupConfigError {
            source: BootstrapSource::ProcessEnv,
            base_url: None,
            message: format!("系统环境变量 {name} 不是有效文本"),
        }),
    }
}

fn read_process_env() -> Result<Option<StartupCredentials>, StartupConfigError> {
    credentials_from_values(
        BootstrapSource::ProcessEnv,
        read_process_env_value(MANAGEMENT_URL_ENV)?,
        read_process_env_value(MANAGEMENT_KEY_ENV)?,
    )
}

fn failed_bootstrap(error: StartupConfigError) -> BootstrapResult {
    BootstrapResult::Failed {
        source: error.source,
        base_url: error.base_url,
        message: error.message,
    }
}

fn redact_secret(message: String, secret: &str) -> String {
    if secret.is_empty() {
        message
    } else {
        message.replace(secret, "[REDACTED]")
    }
}

async fn send(
    client: &reqwest::Client,
    connection: &Connection,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("invalid CPA API path".into());
    }
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| "invalid HTTP method")?;
    let url = format!("{}{}", connection.base_url.trim_end_matches('/'), path);
    let mut request = client
        .request(method, url)
        .bearer_auth(&connection.management_key);
    if let Some(value) = body {
        request = request.json(&value);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("CPA connection failed: {error}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("CPA response failed: {error}"))?;
    let value = serde_json::from_str::<Value>(&text)
        .unwrap_or_else(|_| serde_json::json!({ "message": text }));
    if !status.is_success() {
        let message = value
            .get("message")
            .or_else(|| value.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("CPA request failed");
        return Err(format!("{message} (HTTP {})", status.as_u16()));
    }
    Ok(value)
}

async fn connect_startup_credentials(
    credentials: StartupCredentials,
    state: &State<'_, CpaState>,
) -> Result<BootstrapResult, String> {
    let connection = Connection {
        base_url: credentials.base_url.clone(),
        management_key: credentials.management_key.clone(),
        delete_saved_credential_on_disconnect: false,
    };
    let config = match send(&state.client, &connection, "GET", "/config", None).await {
        Ok(config) => config,
        Err(message) => {
            return Ok(BootstrapResult::Failed {
                source: credentials.source,
                base_url: Some(credentials.base_url),
                message: redact_secret(message, &credentials.management_key),
            })
        }
    };
    *state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")? = Some(connection);
    Ok(BootstrapResult::Connected {
        source: credentials.source,
        base_url: credentials.base_url,
        config,
    })
}

#[tauri::command]
async fn bootstrap_cpa(
    fallback_base_url: String,
    state: State<'_, CpaState>,
) -> Result<BootstrapResult, String> {
    match read_install_env() {
        Ok(Some(credentials)) => return connect_startup_credentials(credentials, &state).await,
        Err(error) => return Ok(failed_bootstrap(error)),
        Ok(None) => {}
    }

    match read_process_env() {
        Ok(Some(credentials)) => return connect_startup_credentials(credentials, &state).await,
        Err(error) => return Ok(failed_bootstrap(error)),
        Ok(None) => {}
    }

    let base_url = match normalize_base_url(&fallback_base_url) {
        Ok(value) => value,
        Err(message) => {
            return Ok(BootstrapResult::Failed {
                source: BootstrapSource::CredentialManager,
                base_url: None,
                message,
            })
        }
    };
    let entry = match credential_entry(&base_url) {
        Ok(entry) => entry,
        Err(message) => {
            return Ok(BootstrapResult::Failed {
                source: BootstrapSource::CredentialManager,
                base_url: Some(base_url),
                message,
            })
        }
    };
    let management_key = match entry.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => return Ok(BootstrapResult::NotConfigured),
        Err(error) => {
            return Ok(BootstrapResult::Failed {
                source: BootstrapSource::CredentialManager,
                base_url: Some(base_url),
                message: format!("读取 CPA 密钥失败: {error}"),
            })
        }
    };
    let connection = Connection {
        base_url: base_url.clone(),
        management_key: management_key.clone(),
        delete_saved_credential_on_disconnect: true,
    };
    let config = match send(&state.client, &connection, "GET", "/config", None).await {
        Ok(config) => config,
        Err(message) => {
            return Ok(BootstrapResult::Failed {
                source: BootstrapSource::CredentialManager,
                base_url: Some(base_url),
                message: redact_secret(message, &management_key),
            })
        }
    };
    *state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")? = Some(connection);
    Ok(BootstrapResult::Connected {
        source: BootstrapSource::CredentialManager,
        base_url,
        config,
    })
}

#[tauri::command]
async fn connect_cpa(
    base_url: String,
    management_key: String,
    remember: bool,
    state: State<'_, CpaState>,
) -> Result<Value, String> {
    let base_url = normalize_base_url(&base_url)?;
    let management_key = management_key.trim().to_string();
    if management_key.is_empty() {
        return Err("CPA Management URL 和明文管理密钥不能为空".into());
    }
    let connection = Connection {
        base_url,
        management_key,
        delete_saved_credential_on_disconnect: true,
    };
    let config = send(&state.client, &connection, "GET", "/config", None).await?;
    if remember {
        credential_entry(&connection.base_url)?
            .set_password(&connection.management_key)
            .map_err(|error| format!("保存 CPA 密钥失败: {error}"))?;
    }
    *state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")? = Some(connection);
    Ok(config)
}

#[tauri::command]
fn current_management_key(state: State<'_, CpaState>) -> Result<String, String> {
    state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")?
        .as_ref()
        .map(|connection| connection.management_key.clone())
        .ok_or_else(|| "请先连接 CPA".into())
}

#[tauri::command]
fn disconnect_cpa(state: State<'_, CpaState>) -> Result<(), String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")?
        .take();
    if let Some(connection) = connection {
        if connection.delete_saved_credential_on_disconnect {
            match credential_entry(&connection.base_url)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(error) => return Err(format!("删除 CPA 凭据失败: {error}")),
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn cpa_request(
    method: String,
    path: String,
    body: Option<Value>,
    state: State<'_, CpaState>,
) -> Result<Value, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "CPA connection lock failed")?
        .clone()
        .ok_or("请先连接 CPA")?;
    send(&state.client, &connection, &method, &path, body).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build CPA HTTP client");
    tauri::Builder::default()
        .manage(CpaState {
            client,
            connection: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            bootstrap_cpa,
            connect_cpa,
            current_management_key,
            disconnect_cpa,
            cpa_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running CPA Route Control");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_complete_credentials_and_normalizes_url() {
        let result = credentials_from_values(
            BootstrapSource::InstallEnv,
            Some(" https://cpa.example.com/v0/management/ ".into()),
            Some(" secret ".into()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(result.base_url, "https://cpa.example.com/v0/management");
        assert_eq!(result.management_key, "secret");
    }

    #[test]
    fn ignores_source_without_relevant_values() {
        let result = credentials_from_values(BootstrapSource::ProcessEnv, None, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn rejects_partial_credentials_without_cross_source_fallback() {
        let error = credentials_from_values(
            BootstrapSource::InstallEnv,
            Some("http://127.0.0.1:8317/v0/management".into()),
            None,
        )
        .unwrap_err();

        assert_eq!(error.source, BootstrapSource::InstallEnv);
        assert_eq!(
            error.base_url.as_deref(),
            Some("http://127.0.0.1:8317/v0/management")
        );
    }

    #[test]
    fn rejects_non_http_management_url() {
        let error = credentials_from_values(
            BootstrapSource::ProcessEnv,
            Some("file:///tmp/config".into()),
            Some("secret".into()),
        )
        .unwrap_err();

        assert!(error.message.contains("HTTP 或 HTTPS"));
        assert!(error.base_url.is_none());
    }

    #[test]
    fn redacts_management_key_from_errors() {
        let message = redact_secret("request with top-secret failed".into(), "top-secret");
        assert_eq!(message, "request with [REDACTED] failed");
    }
}
