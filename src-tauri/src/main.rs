mod commands;
mod fake_session;
mod logic_analyzer;
mod mcp;
mod model;
mod state;

use state::DesktopState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    tauri::Builder::default()
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_serial_ports,
            commands::get_session_snapshot,
            commands::get_mcp_server_status,
            commands::get_logic_analyzer_status,
            commands::refresh_logic_analyzer_devices,
            commands::start_logic_analyzer_capture,
            commands::stop_logic_analyzer_capture,
            commands::connect_serial,
            commands::disconnect_serial,
            commands::send_serial,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EADAI desktop app");
}
