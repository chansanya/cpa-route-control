use std::sync::Mutex;

use serde_json::Value;
use tauri::State;

#[derive(Clone)]
struct Connection {
    base_url: String,
    management_key: String,
}

struct CpaState {
    client: reqwest::Client,
    connection: Mutex<Option<Connection>>,
}

const KEYRING_SERVICE: &str = "CPA Route Control";

fn credential_entry(base_url: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, base_url).map_err(|error| format!("Windows Credential Manager error: {error}"))
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
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| "invalid HTTP method")?;
    let url = format!("{}{}", connection.base_url.trim_end_matches('/'), path);
    let mut request = client.request(method, url).bearer_auth(&connection.management_key);
    if let Some(value) = body { request = request.json(&value); }
    let response = request.send().await.map_err(|error| format!("CPA connection failed: {error}"))?;
    let status = response.status();
    let text = response.text().await.map_err(|error| format!("CPA response failed: {error}"))?;
    let value = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| serde_json::json!({ "message": text }));
    if !status.is_success() {
        let message = value.get("message").or_else(|| value.get("error")).and_then(Value::as_str).unwrap_or("CPA request failed");
        return Err(format!("{message} (HTTP {})", status.as_u16()));
    }
    Ok(value)
}

#[tauri::command]
async fn connect_cpa(base_url: String, management_key: String, remember: bool, state: State<'_, CpaState>) -> Result<Value, String> {
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) || management_key.is_empty() {
        return Err("CPA Management URL 和明文管理密钥不能为空".into());
    }
    let connection = Connection { base_url: base_url.trim_end_matches('/').to_string(), management_key };
    let config = send(&state.client, &connection, "GET", "/config", None).await?;
    if remember {
        credential_entry(&connection.base_url)?.set_password(&connection.management_key).map_err(|error| format!("保存 CPA 密钥失败: {error}"))?;
    }
    *state.connection.lock().map_err(|_| "CPA connection lock failed")? = Some(connection);
    Ok(config)
}

#[tauri::command]
async fn restore_cpa(base_url: String, state: State<'_, CpaState>) -> Result<Option<Value>, String> {
    let base_url = base_url.trim_end_matches('/').to_string();
    let management_key = match credential_entry(&base_url)?.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => return Err(format!("读取 CPA 密钥失败: {error}")),
    };
    let connection = Connection { base_url, management_key };
    let config = send(&state.client, &connection, "GET", "/config", None).await?;
    *state.connection.lock().map_err(|_| "CPA connection lock failed")? = Some(connection);
    Ok(Some(config))
}

#[tauri::command]
async fn cpa_request(method: String, path: String, body: Option<Value>, state: State<'_, CpaState>) -> Result<Value, String> {
    let connection = state.connection.lock().map_err(|_| "CPA connection lock failed")?.clone().ok_or("请先连接 CPA")?;
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
        .manage(CpaState { client, connection: Mutex::new(None) })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![connect_cpa, restore_cpa, cpa_request])
        .run(tauri::generate_context!())
        .expect("error while running CPA Route Control");
}
