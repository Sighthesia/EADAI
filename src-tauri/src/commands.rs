use crate::model::{
    ConnectRequest, LogicAnalyzerCaptureRequest, LogicAnalyzerStatus, McpServerStatus, SendRequest,
    SessionSnapshot,
};
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
pub fn get_logic_analyzer_status(state: State<'_, DesktopState>) -> LogicAnalyzerStatus {
    state.logic_analyzer_status()
}

#[tauri::command]
pub fn refresh_logic_analyzer_devices(
    state: State<'_, DesktopState>,
) -> Result<LogicAnalyzerStatus, String> {
    state.refresh_logic_analyzer_devices()
}

#[tauri::command]
pub fn start_logic_analyzer_capture(
    state: State<'_, DesktopState>,
    request: LogicAnalyzerCaptureRequest,
) -> Result<LogicAnalyzerStatus, String> {
    state.start_logic_analyzer_capture(request)
}

#[tauri::command]
pub fn stop_logic_analyzer_capture(
    state: State<'_, DesktopState>,
) -> Result<LogicAnalyzerStatus, String> {
    state.stop_logic_analyzer_capture()
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
