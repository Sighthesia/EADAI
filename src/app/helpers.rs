use crate::message::LinePayload;
use std::cmp::min;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::command::StopSignal;
use super::RETRY_SLEEP_SLICE_MS;

pub(super) fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) fn outbound_payload(payload: &[u8]) -> LinePayload {
    let mut raw = payload.to_vec();

    while matches!(raw.last(), Some(b'\n' | b'\r')) {
        raw.pop();
    }

    let text = String::from_utf8_lossy(&raw).into_owned();
    LinePayload { text, raw }
}

pub(super) fn hex_preview(bytes: &[u8], max_len: usize) -> String {
    let preview = bytes
        .iter()
        .take(max_len)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    if bytes.len() > max_len {
        format!("{preview} ...")
    } else {
        preview
    }
}

pub(super) fn sleep_with_stop(stop_signal: &StopSignal, total_delay: Duration) -> bool {
    let mut remaining_ms = total_delay.as_millis() as u64;

    while remaining_ms > 0 {
        if stop_signal.is_requested() {
            return false;
        }

        let sleep_ms = min(remaining_ms, RETRY_SLEEP_SLICE_MS);
        thread::sleep(Duration::from_millis(sleep_ms));
        remaining_ms -= sleep_ms;
    }

    true
}
