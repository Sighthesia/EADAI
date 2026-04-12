use crate::model::{ConnectRequest, McpServerStatus, SendRequest, SessionSnapshot};
use crate::state::DesktopState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_serial_ports(state: State<'_, DesktopState>) -> Result<Vec<String>, String> {
    state.list_ports()
}

#[tauri::command]
pub fn get_session_snapshot(state: State<'_, DesktopState>) -> SessionSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn get_mcp_server_status(state: State<'_, DesktopState>) -> McpServerStatus {
    state.mcp_status()
}

#[tauri::command]
pub fn connect_serial(
    app_handle: AppHandle,
    state: State<'_, DesktopState>,
    request: ConnectRequest,
) -> Result<SessionSnapshot, String> {
    state.connect(&app_handle, request)
}

#[tauri::command]
pub fn disconnect_serial(state: State<'_, DesktopState>) -> Result<SessionSnapshot, String> {
    state.disconnect()
}

#[tauri::command]
pub fn send_serial(state: State<'_, DesktopState>, request: SendRequest) -> Result<(), String> {
    state.send(request)
}
