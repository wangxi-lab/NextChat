// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod goose;
mod stream;

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      stream::stream_fetch,
      goose::start_goose_agent,
      goose::goose_status,
      goose::goose_chat
    ])
    .setup(|app| {
      let handle = app.handle();
      tauri::async_runtime::spawn(async move {
        if let Err(err) = goose::start_goose_agent(handle) {
          println!("Failed to start Goose agent: {}", err);
        }
      });
      Ok(())
    })
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
