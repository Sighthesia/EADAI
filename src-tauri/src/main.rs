mod commands;
mod fake_session;
mod logic_analyzer;
mod mcp;
mod model;
mod state;

use state::DesktopState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .manage(DesktopState::default())
        .setup(|app| {
            app.state::<DesktopState>()
                .start_serial_device_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_serial_ports,
            commands::get_session_snapshot,
            commands::get_mcp_server_status,
            commands::get_mcp_tool_usage_snapshot,
            commands::get_logic_analyzer_status,
            commands::refresh_logic_analyzer_devices,
            commands::start_logic_analyzer_capture,
            commands::stop_logic_analyzer_capture,
            commands::connect_serial,
            commands::disconnect_serial,
            commands::send_serial,
            commands::send_bmi088_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EADAI desktop app");
}
