use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

static GOOSE_AGENT_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static GOOSE_SESSION_ID: Mutex<Option<String>> = Mutex::new(None);

const DEFAULT_GOOSE_AGENT_HOST: &str = "127.0.0.1";
const DEFAULT_GOOSE_AGENT_PORT: &str = "32123";
const GOOSE_AGENT_SECRET: &str = "nextchat-local-goose-agent";
const GOOSE_CONFIG_FILE: &str = "goose.config.json";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GooseConfig {
    provider: Option<String>,
    provider_type: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    tls: Option<bool>,
    secret_key: Option<String>,
    env: Option<HashMap<String, String>>,
}

#[derive(Clone, serde::Serialize)]
pub struct GooseStatus {
    available: bool,
    path: Option<String>,
    version: Option<String>,
    error: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct GooseResponse {
    content: String,
    path: String,
}

fn base_dirs(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            dirs.push(exe_dir.to_path_buf());
        }
    }

    if let Some(resource_dir) = app.path_resolver().resource_dir() {
        dirs.push(resource_dir);
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest_dir));
    }

    dirs
}

fn goose_agent_file_name() -> &'static str {
    if cfg!(windows) {
        "goosed.exe"
    } else {
        "goosed"
    }
}

fn goose_agent_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for dir in base_dirs(app) {
        candidates.push(dir.join(goose_agent_file_name()));
        candidates.push(dir.join("bin").join(goose_agent_file_name()));
        candidates.push(
            dir.join("bin")
                .join("Goose-win32-x64")
                .join("resources")
                .join("bin")
                .join(goose_agent_file_name()),
        );
    }

    candidates
}

fn goose_config_candidates(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var("NEXTCHAT_GOOSE_CONFIG") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }

    for dir in base_dirs(app) {
        let mut current = Some(dir.as_path());
        let mut depth = 0;

        while let Some(candidate_dir) = current {
            candidates.push(candidate_dir.join(GOOSE_CONFIG_FILE));
            candidates.push(candidate_dir.join("resources").join(GOOSE_CONFIG_FILE));
            candidates.push(candidate_dir.join("_up_").join(GOOSE_CONFIG_FILE));
            candidates.push(
                candidate_dir
                    .join("resources")
                    .join("_up_")
                    .join(GOOSE_CONFIG_FILE),
            );

            depth += 1;
            if depth >= 6 {
                break;
            }

            current = candidate_dir.parent();
        }
    }

    candidates
}

fn load_goose_config(app: &tauri::AppHandle) -> GooseConfig {
    for path in goose_config_candidates(app) {
        if !path.exists() {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<GooseConfig>(&content) {
                Ok(config) => {
                    println!("Loaded Goose config from {}", path.display());
                    return config;
                }
                Err(err) => {
                    println!("Failed to parse Goose config {}: {}", path.display(), err);
                }
            },
            Err(err) => {
                println!("Failed to read Goose config {}: {}", path.display(), err);
            }
        }
    }

    GooseConfig::default()
}

fn resolve_goose_agent(app: &tauri::AppHandle) -> Option<String> {
    if let Ok(path) = std::env::var("NEXTCHAT_GOOSE_AGENT_BIN") {
        if !path.trim().is_empty() {
            return Some(path);
        }
    }

    for path in goose_agent_candidates(app) {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

fn hide_goose_command_window(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
}

fn command_error(command: &str, err: &std::io::Error) -> String {
    format!(
    "Unable to start Goose at '{}'. Put the Goose agent executable in src-tauri/bin for packaging, or set NEXTCHAT_GOOSE_AGENT_BIN. Original error: {}",
    command, err
  )
}

fn goose_agent_host(config: &GooseConfig) -> String {
    std::env::var("NEXTCHAT_GOOSE_AGENT_HOST").unwrap_or_else(|_| {
        config
            .host
            .clone()
            .unwrap_or_else(|| DEFAULT_GOOSE_AGENT_HOST.to_string())
    })
}

fn goose_agent_port(config: &GooseConfig) -> String {
    std::env::var("NEXTCHAT_GOOSE_AGENT_PORT").unwrap_or_else(|_| {
        config
            .port
            .map(|port| port.to_string())
            .unwrap_or_else(|| DEFAULT_GOOSE_AGENT_PORT.to_string())
    })
}

fn goose_agent_tls(config: &GooseConfig) -> bool {
    config.tls.unwrap_or(false)
}

fn goose_agent_secret(config: &GooseConfig) -> String {
    config
        .secret_key
        .clone()
        .filter(|secret| !secret.trim().is_empty())
        .unwrap_or_else(|| GOOSE_AGENT_SECRET.to_string())
}

fn goose_agent_base_url(config: &GooseConfig) -> String {
    let scheme = if goose_agent_tls(config) {
        "https"
    } else {
        "http"
    };
    format!(
        "{}://{}:{}",
        scheme,
        goose_agent_host(config),
        goose_agent_port(config)
    )
}

fn provider_api_key_env(provider: &str) -> Option<&'static str> {
    match provider.to_ascii_lowercase().as_str() {
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "azure" => Some("AZURE_OPENAI_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        "gemini" | "google" => Some("GOOGLE_API_KEY"),
        "groq" => Some("GROQ_API_KEY"),
        "mistral" => Some("MISTRAL_API_KEY"),
        "openai" => Some("OPENAI_API_KEY"),
        "openrouter" => Some("OPENROUTER_API_KEY"),
        "xai" => Some("XAI_API_KEY"),
        _ => None,
    }
}

fn known_provider_type(provider: &str) -> Option<&'static str> {
    match provider.to_ascii_lowercase().as_str() {
        "anthropic" => Some("anthropic"),
        "azure" => Some("azure"),
        "bedrock" | "amazon-bedrock" => Some("bedrock"),
        "databricks" => Some("databricks"),
        "deepseek" => Some("openai"),
        "gemini" | "google" => Some("google"),
        "groq" => Some("groq"),
        "litellm" => Some("litellm"),
        "mistral" => Some("mistral"),
        "ollama" => Some("ollama"),
        "openai" => Some("openai"),
        "openrouter" => Some("openrouter"),
        "xai" => Some("xai"),
        _ => None,
    }
}

fn goose_provider_name(config: &GooseConfig) -> Option<String> {
    let provider = config.provider.as_ref()?.trim();
    if provider.is_empty() {
        return None;
    }

    let provider_type = config
        .provider_type
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase());

    if matches!(provider_type.as_deref(), Some("openai")) {
        Some("openai".to_string())
    } else {
        Some(provider.to_ascii_lowercase())
    }
}

fn apply_goose_provider_config(command: &mut Command, config: &GooseConfig) {
    if let Some(provider) = goose_provider_name(config) {
        command.env("GOOSE_PROVIDER", &provider);

        let provider_type = config
            .provider_type
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.as_str())
            .or_else(|| known_provider_type(&provider))
            .or_else(|| {
                config
                    .base_url
                    .as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|_| "openai")
            });

        if let Some(provider_type) = provider_type {
            command.env("GOOSE_PROVIDER__TYPE", provider_type);
        }

        if let Some(api_key) = config
            .api_key
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            command.env("GOOSE_PROVIDER__API_KEY", api_key);
            if let Some(env_name) = provider_api_key_env(&provider) {
                command.env(env_name, api_key);
            }

            if provider == "openai" {
                command.env("OPENAI_API_KEY", api_key);
            }
        }
    }

    if let Some(model) = config
        .model
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        command.env("GOOSE_MODEL", model);
        command.env("GOOSE_PROVIDER__MODEL", model);
    }

    if let Some(base_url) = config
        .base_url
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        command.env("GOOSE_PROVIDER__HOST", base_url);
        if goose_provider_name(config).as_deref() == Some("openai") {
            command.env("OPENAI_BASE_URL", base_url);
        }
    }

    if let Some(env) = &config.env {
        for (key, value) in env {
            if !key.trim().is_empty() && !value.trim().is_empty() {
                command.env(key, value);
            }
        }
    }
}

#[tauri::command]
pub fn start_goose_agent(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(goose_agent) = resolve_goose_agent(&app) else {
        return Ok(None);
    };

    let mut process = GOOSE_AGENT_PROCESS
        .lock()
        .map_err(|err| format!("Failed to lock Goose process: {}", err))?;

    if let Some(child) = process.as_mut() {
        if child.try_wait().map_err(|err| err.to_string())?.is_none() {
            return Ok(Some(goose_agent));
        }
    }

    let config = load_goose_config(&app);
    let mut command = Command::new(&goose_agent);
    hide_goose_command_window(&mut command);
    command
        .arg("agent")
        .env("GOOSE_HOST", goose_agent_host(&config))
        .env("GOOSE_PORT", goose_agent_port(&config))
        .env("GOOSE_TLS", goose_agent_tls(&config).to_string())
        .env("GOOSE_SERVER__SECRET_KEY", goose_agent_secret(&config))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    apply_goose_provider_config(&mut command, &config);

    let child = command
        .spawn()
        .map_err(|err| command_error(&goose_agent, &err))?;

    *process = Some(child);
    Ok(Some(goose_agent))
}

#[tauri::command]
pub fn goose_status(app: tauri::AppHandle) -> GooseStatus {
    let Some(goose_agent) = resolve_goose_agent(&app) else {
        return GooseStatus {
            available: false,
            path: None,
            version: None,
            error: Some("Goose agent executable was not found".to_string()),
        };
    };

    let mut command = Command::new(&goose_agent);
    hide_goose_command_window(&mut command);

    match command.arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            GooseStatus {
                available: true,
                path: Some(goose_agent),
                version: if version.is_empty() {
                    None
                } else {
                    Some(version)
                },
                error: None,
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            GooseStatus {
                available: false,
                path: Some(goose_agent),
                version: None,
                error: Some(stderr),
            }
        }
        Err(err) => GooseStatus {
            available: false,
            path: Some(goose_agent.clone()),
            version: None,
            error: Some(command_error(&goose_agent, &err)),
        },
    }
}

#[tauri::command]
pub async fn goose_chat(app: tauri::AppHandle, prompt: String) -> Result<GooseResponse, String> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Prompt is empty".to_string());
    }

    let config = load_goose_config(&app);
    let path = start_goose_agent(app)?
        .ok_or_else(|| "Goose agent executable was not found".to_string())?;
    let session_id = ensure_goose_session(&config).await?;

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs() as i64;

    let body = serde_json::json!({
        "user_message": {
            "id": null,
            "role": "user",
            "created": created,
            "content": [
                {
                    "type": "text",
                    "text": prompt
                }
            ],
            "metadata": {
                "userVisible": true,
                "agentVisible": true
            }
        },
        "session_id": session_id,
        "recipe_name": null,
        "recipe_version": null
    });

    let response = reqwest::Client::new()
        .post(format!("{}/reply", goose_agent_base_url(&config)))
        .header("X-Secret-Key", goose_agent_secret(&config))
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("Failed to call Goose agent: {}", err))?;

    let status = response.status();
    let event_text = response
        .text()
        .await
        .map_err(|err| format!("Failed to read Goose response: {}", err))?;

    if !status.is_success() {
        return Err(format!("Goose agent returned {}: {}", status, event_text));
    }

    let content = parse_goose_sse_response(&event_text)?;

    if content.trim().is_empty() {
        Err("Goose agent returned an empty response".to_string())
    } else {
        Ok(GooseResponse { content, path })
    }
}

async fn update_goose_agent_provider(config: &GooseConfig, session_id: &str) -> Result<(), String> {
    let provider = goose_provider_name(config).ok_or_else(|| {
        format!(
            "Goose provider is not configured. Please set provider in {}",
            GOOSE_CONFIG_FILE
        )
    })?;
    let model = config
        .model
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "Goose model is not configured. Please set model in {}",
                GOOSE_CONFIG_FILE
            )
        })?;

    let response = reqwest::Client::new()
        .post(format!(
            "{}/agent/update_provider",
            goose_agent_base_url(config)
        ))
        .header("X-Secret-Key", goose_agent_secret(config))
        .json(&serde_json::json!({
            "session_id": session_id,
            "provider": provider,
            "model": model,
            "context_limit": null,
            "request_params": null
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to update Goose provider: {}", err))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("Failed to read Goose provider response: {}", err))?;

    if status.is_success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to update Goose provider {}: {}",
            status, text
        ))
    }
}

async fn ensure_goose_session(config: &GooseConfig) -> Result<String, String> {
    if let Some(session_id) = GOOSE_SESSION_ID
        .lock()
        .map_err(|err| format!("Failed to lock Goose session: {}", err))?
        .clone()
    {
        return Ok(session_id);
    }

    let working_dir = std::env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let response = reqwest::Client::new()
        .post(format!("{}/agent/start", goose_agent_base_url(config)))
        .header("X-Secret-Key", goose_agent_secret(config))
        .json(&serde_json::json!({
            "working_dir": working_dir,
            "recipe": null,
            "recipe_id": null,
            "recipe_deeplink": null,
            "extension_overrides": null
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to start Goose session: {}", err))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("Failed to read Goose session response: {}", err))?;

    if !status.is_success() {
        return Err(format!(
            "Failed to start Goose session {}: {}",
            status, text
        ));
    }

    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| format!("Invalid Goose session response: {}", err))?;
    let session_id = value
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| "Goose session response did not include an id".to_string())?
        .to_string();

    update_goose_agent_provider(config, &session_id).await?;

    *GOOSE_SESSION_ID
        .lock()
        .map_err(|err| format!("Failed to lock Goose session: {}", err))? =
        Some(session_id.clone());

    Ok(session_id)
}

fn parse_goose_sse_response(event_text: &str) -> Result<String, String> {
    let mut chunks = Vec::new();
    let mut last_error = None;

    for line in event_text.lines() {
        let Some(data) = line.strip_prefix("data: ") else {
            continue;
        };

        let value: serde_json::Value = serde_json::from_str(data)
            .map_err(|err| format!("Failed to parse Goose event: {}", err))?;

        match value.get("type").and_then(|v| v.as_str()) {
            Some("Message") => {
                if value
                    .get("message")
                    .and_then(|message| message.get("role"))
                    .and_then(|role| role.as_str())
                    != Some("assistant")
                {
                    continue;
                }

                if let Some(content) = value
                    .get("message")
                    .and_then(|message| message.get("content"))
                    .and_then(|content| content.as_array())
                {
                    for item in content {
                        if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                            if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                chunks.push(text.to_string());
                            }
                        }
                    }
                }
            }
            Some("Error") => {
                last_error = value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(|error| error.to_string());
            }
            _ => {}
        }
    }

    if !chunks.is_empty() {
        Ok(strip_think_blocks(&chunks.join("\n")))
    } else if let Some(error) = last_error {
        Err(error)
    } else {
        Ok(String::new())
    }
}

fn strip_think_blocks(text: &str) -> String {
    let mut output = String::new();
    let mut remaining = text;

    loop {
        let Some(start) = remaining.find("<think>") else {
            output.push_str(remaining);
            break;
        };

        output.push_str(&remaining[..start]);
        remaining = &remaining[start + "<think>".len()..];

        let Some(end) = remaining.find("</think>") else {
            break;
        };

        remaining = &remaining[end + "</think>".len()..];
    }

    output.trim().to_string()
}
