// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod stream;

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tauri::{
    api::process::{Command, CommandChild, CommandEvent},
    RunEvent,
};
use tauri_plugin_window_state::StateFlags;

type AgentProcess = Arc<Mutex<Option<CommandChild>>>;

fn main() {
    let agent_process: AgentProcess = Arc::new(Mutex::new(None));
    let setup_agent_process = Arc::clone(&agent_process);
    let exit_agent_process = Arc::clone(&agent_process);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![stream::stream_fetch])
        .setup(move |app| {
            let app_config_dir = app.path_resolver().app_config_dir();
            let log_dir = app_config_dir.as_ref().map(|dir| dir.join("logs"));
            write_app_log(&log_dir, "NextChat setup starting");

            if is_generic_agent_running() {
                log_message(
                    &log_dir,
                    "GenericAgent is already listening on 127.0.0.1:8765",
                );
                return Ok(());
            }

            match start_generic_agent(app_config_dir, log_dir.clone()) {
                Ok(child) => {
                    log_message(
                        &log_dir,
                        &format!("GenericAgent sidecar started, pid={}", child.pid()),
                    );
                    *setup_agent_process.lock().unwrap() = Some(child);
                }
                Err(err) => {
                    log_message(
                        &log_dir,
                        &format!("Failed to start GenericAgent sidecar: {err}"),
                    );
                }
            }
            Ok(())
        })
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::SIZE)
                .build(),
        )
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let RunEvent::Exit = event {
                stop_generic_agent(&exit_agent_process);
            }
        });
}

fn is_generic_agent_running() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8765));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

fn start_generic_agent(
    app_config_dir: Option<PathBuf>,
    log_dir: Option<PathBuf>,
) -> Result<CommandChild, Box<dyn std::error::Error>> {
    let config_dir = app_config_dir
        .unwrap_or_else(|| std::env::temp_dir().join("NextChat"))
        .join("generic-agent");
    fs::create_dir_all(&config_dir)?;
    if let Some(dir) = &log_dir {
        fs::create_dir_all(dir)?;
    }

    let mut env = HashMap::new();
    env.insert("NEXTCHAT_AGENT_HOST".to_string(), "127.0.0.1".to_string());
    env.insert("NEXTCHAT_AGENT_PORT".to_string(), "8765".to_string());
    env.insert(
        "GENERIC_AGENT_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().to_string(),
    );
    if let Some(dir) = &log_dir {
        env.insert(
            "NEXTCHAT_LOG_DIR".to_string(),
            dir.to_string_lossy().to_string(),
        );
    }
    env.insert("PYTHONUTF8".to_string(), "1".to_string());

    let (mut rx, child) = Command::new_sidecar("generic-agent-nextchat")?
        .args(["--host", "127.0.0.1", "--port", "8765"])
        .envs(env)
        .current_dir(config_dir)
        .spawn()?;

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    log_message(&log_dir, &format!("[GenericAgent] {line}"))
                }
                CommandEvent::Stderr(line) => {
                    log_message(&log_dir, &format!("[GenericAgent] {line}"))
                }
                CommandEvent::Error(err) => {
                    log_message(&log_dir, &format!("[GenericAgent error] {err}"))
                }
                CommandEvent::Terminated(payload) => {
                    log_message(
                        &log_dir,
                        &format!("[GenericAgent] terminated: {:?}", payload),
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(child)
}

fn log_message(log_dir: &Option<PathBuf>, message: &str) {
    eprintln!("{message}");
    write_app_log(log_dir, message);
}

fn write_app_log(log_dir: &Option<PathBuf>, message: &str) {
    if let Some(dir) = log_dir {
        let _ = append_log(dir, message);
    }
}

fn append_log(log_dir: &Path, message: &str) -> std::io::Result<()> {
    fs::create_dir_all(log_dir)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("nextchat.log"))?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    writeln!(file, "[{ts}] {message}")?;
    Ok(())
}

fn stop_generic_agent(agent_process: &AgentProcess) {
    if let Some(child) = agent_process.lock().unwrap().take() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &child.pid().to_string()])
                .status();
        }
        let _ = child.kill();
    }
}
