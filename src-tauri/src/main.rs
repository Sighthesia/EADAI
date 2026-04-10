mod commands;
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
            commands::connect_serial,
            commands::disconnect_serial,
            commands::send_serial,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EADAI desktop app");
}
