//! Crazyradio transport implementation.
//!
//! Uses the `crazyflie-link` crate to establish a radio link to a Crazyflie
//! quadcopter via the Crazyradio USB dongle. The link provides bidirectional
//! CRTP packet exchange over 2.4 GHz nRF24 radio.
//!
//! ## CRTP framing bridge
//!
//! `crazyflie-link` works with structured CRTP packets (port, channel, data).
//! The `ByteTransport` trait works with raw byte streams. This adapter bridges
//! the two worlds:
//!
//! - **Receive**: `crazyflie-link` gives us `(port, channel, data)`. We reconstruct
//!   a CRTP-over-serial frame (`[header][length][data...][crc8]`) so the framing
//!   layer can parse it. The header encodes port (bits 7-5) and channel (bits 2-0).
//! - **Send**: The framing layer gives us CRTP-over-serial bytes. We parse the
//!   header to extract port/channel, then forward the payload as a `Packet`.
//!
//! CRTP-over-serial format (NO SOF byte):
//!   `[header][length][data...][crc8]`
//!   - header: (port << 5) | (channel & 0x03)
//!   - length: payload length (0-63)
//!   - crc8: CRC over header+length+data (init=0, poly=0x1D)

use super::{ByteTransport, CrazyradioDatarate, TransportError, TransportKind, TransportResult};
use std::sync::mpsc;
use std::thread;

/// Parsed CRTP-over-serial frame components.
struct CrtpSerFrame {
    port: u8,
    channel: u8,
    payload: Vec<u8>,
}

/// Events received from the async link worker.
enum LinkEvent {
    Connected,
    /// CRTP packet received: (port, channel, data).
    Received(u8, u8, Vec<u8>),
    Disconnected(String),
}

/// Crazyradio transport wrapping `crazyflie-link`.
pub struct CrazyradioTransport {
    kind: TransportKind,
    command_tx: mpsc::Sender<CrtpSerFrame>,
    shutdown_tx: mpsc::Sender<()>,
    event_rx: mpsc::Receiver<LinkEvent>,
    _worker: thread::JoinHandle<()>,
    connected: bool,
}

impl CrazyradioTransport {
    /// Connects to a Crazyflie via Crazyradio.
    pub fn connect(uri: &str, datarate: CrazyradioDatarate) -> TransportResult<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<CrtpSerFrame>();
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        let (evt_tx, evt_rx) = mpsc::channel::<LinkEvent>();

        let uri_owned = uri.to_string();
        let worker = thread::Builder::new()
            .name("crazyradio-link".into())
            .spawn(move || {
                run_link_worker(&uri_owned, datarate, cmd_rx, shutdown_rx, evt_tx);
            })
            .map_err(|e| {
                TransportError::ConnectionFailed(format!("failed to spawn worker: {e}"))
            })?;

        match evt_rx.recv() {
            Ok(LinkEvent::Connected) => {}
            Ok(LinkEvent::Disconnected(reason)) => {
                return Err(TransportError::ConnectionFailed(reason));
            }
            Ok(LinkEvent::Received(_, _, _)) => {
                return Err(TransportError::ConnectionFailed(
                    "unexpected data before connection confirmation".into(),
                ));
            }
            Err(_) => {
                return Err(TransportError::ConnectionFailed(
                    "worker thread exited before connection".into(),
                ));
            }
        }

        Ok(Self {
            kind: TransportKind::Crazyradio {
                uri: uri.to_string(),
                datarate,
            },
            command_tx: cmd_tx,
            shutdown_tx,
            event_rx: evt_rx,
            _worker: worker,
            connected: true,
        })
    }

    /// Scans for nearby Crazyflie devices.
    /// FIXME: implement real scan via `LinkContext::scan()`.
    pub fn scan() -> Vec<String> {
        Vec::new()
    }
}

impl ByteTransport for CrazyradioTransport {
    fn kind(&self) -> &TransportKind {
        &self.kind
    }

    fn read_chunk(&mut self) -> TransportResult<Option<Vec<u8>>> {
        match self.event_rx.recv() {
            Ok(LinkEvent::Received(port, channel, data)) => {
                let framed = encode_crtp_over_serial_frame(port, channel, &data);
                Ok(Some(framed))
            }
            Ok(LinkEvent::Disconnected(reason)) => {
                self.connected = false;
                Err(TransportError::ReadFailed(reason))
            }
            Ok(LinkEvent::Connected) => self.read_chunk(),
            Err(_) => {
                self.connected = false;
                Err(TransportError::ReadFailed("worker disconnected".into()))
            }
        }
    }

    fn write_all(&mut self, data: &[u8]) -> TransportResult<()> {
        let frame = parse_crtp_over_serial_frame(data).map_err(TransportError::WriteFailed)?;
        self.command_tx
            .send(frame)
            .map_err(|_| TransportError::WriteFailed("worker disconnected".into()))
    }

    fn flush(&mut self) -> TransportResult<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Drop for CrazyradioTransport {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
    }
}

// ---------------------------------------------------------------------------
// CRTP-over-serial framing helpers
// ---------------------------------------------------------------------------

/// Parses a CRTP-over-serial byte frame into its components.
///
/// Format: `[header][length][data...][crc8]` (NO SOF byte).
fn parse_crtp_over_serial_frame(data: &[u8]) -> Result<CrtpSerFrame, String> {
    if data.len() < 3 {
        return Err(format!(
            "data too short for CRTP-over-serial frame: {} bytes (min 3)",
            data.len()
        ));
    }

    let length = data[1] as usize;
    if length > 63 {
        return Err(format!("invalid CRTP length: {length} (max 63)"));
    }

    let frame_len = 1 + 1 + length + 1; // header + length + data + CRC
    if data.len() < frame_len {
        return Err(format!(
            "frame truncated: expected {frame_len} bytes, got {}",
            data.len()
        ));
    }

    let header = data[0];
    let port = (header >> 5) & 0x07;
    let channel = header & 0x03;
    let payload = data[2..2 + length].to_vec();

    Ok(CrtpSerFrame {
        port,
        channel,
        payload,
    })
}

/// Reconstructs a CRTP-over-serial frame from decoded CRTP packet metadata.
///
/// Format: `[header][length][data...][crc8]` (NO SOF byte).
/// CRC is computed over header+length+data (init=0, poly=0x1D).
fn encode_crtp_over_serial_frame(port: u8, channel: u8, data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![];
    }
    let length = data.len() as u8;
    let header = ((port & 0x07) << 5) | (channel & 0x03);
    let mut frame = Vec::with_capacity(2 + data.len() + 1);
    frame.push(header);
    frame.push(length);
    frame.extend_from_slice(data);
    // CRC over header + length + data (everything before the CRC byte)
    let crc = crtp_crc8(&frame);
    frame.push(crc);
    frame
}

/// CRC-8 for CRTP-over-serial (init=0, poly=0x1D).
fn crtp_crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x1D;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// ---------------------------------------------------------------------------
// Async link worker
// ---------------------------------------------------------------------------

fn run_link_worker(
    uri: &str,
    datarate: CrazyradioDatarate,
    cmd_rx: mpsc::Receiver<CrtpSerFrame>,
    shutdown_rx: mpsc::Receiver<()>,
    evt_tx: mpsc::Sender<LinkEvent>,
) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            let _ = evt_tx.send(LinkEvent::Disconnected(format!(
                "failed to create runtime: {e}"
            )));
            return;
        }
    };

    rt.block_on(async move {
        let ctx = crazyflie_link::LinkContext::new();
        let datarate_str = match datarate {
            CrazyradioDatarate::Dr250K => "250K",
            CrazyradioDatarate::Dr1M => "1M",
            CrazyradioDatarate::Dr2M => "2M",
        };
        let connection_uri = if uri.starts_with("radio://") {
            uri.to_string()
        } else {
            format!("radio://0/60/{datarate_str}/E7E7E7E7E7")
        };
        let connection: crazyflie_link::Connection = match ctx.open_link(&connection_uri).await {
            Ok(conn) => conn,
            Err(e) => {
                let _ = evt_tx.send(LinkEvent::Disconnected(format!(
                    "failed to open connection: {e}"
                )));
                return;
            }
        };
        if evt_tx.send(LinkEvent::Connected).is_err() {
            return;
        }
        loop {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }
            while let Ok(frame) = cmd_rx.try_recv() {
                let pkt = crazyflie_link::Packet::new(frame.port, frame.channel, frame.payload);
                if let Err(e) = connection.send_packet(pkt).await {
                    let _ = evt_tx.send(LinkEvent::Disconnected(format!("send error: {e}")));
                    return;
                }
            }
            match tokio::time::timeout(
                std::time::Duration::from_millis(10),
                connection.recv_packet(),
            )
            .await
            {
                Ok(Ok(pkt)) => {
                    let port = pkt.get_port();
                    let channel = pkt.get_channel();
                    let data: Vec<u8> = pkt.get_data().clone();
                    if evt_tx
                        .send(LinkEvent::Received(port, channel, data))
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    let _ = evt_tx.send(LinkEvent::Disconnected(format!("recv error: {e}")));
                    break;
                }
                Err(_timeout) => {}
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_frame_preserves_port_and_channel() {
        let data = vec![0x01, 0x02, 0x03];
        let frame = encode_crtp_over_serial_frame(3, 2, &data);
        // header = (3 << 5) | 2 = 0x62
        assert_eq!(frame[0], 0x62);
        assert_eq!(frame[1], 3); // length
        assert_eq!(&frame[2..5], &[0x01, 0x02, 0x03]);
        assert_eq!(frame.len(), 6); // header + length + 3 data + CRC
    }

    #[test]
    fn encode_frame_port_zero_channel_zero() {
        let data = vec![0xAA];
        let frame = encode_crtp_over_serial_frame(0, 0, &data);
        assert_eq!(frame[0], 0x00); // header
        assert_eq!(frame[1], 1); // length
        assert_eq!(frame[2], 0xAA);
        assert_eq!(frame.len(), 4);
    }

    #[test]
    fn encode_frame_max_port_and_channel() {
        let data = vec![0xBB];
        let frame = encode_crtp_over_serial_frame(7, 3, &data);
        assert_eq!(frame[0], 0xE3); // (7 << 5) | 3
    }

    #[test]
    fn encode_frame_empty_data() {
        assert!(encode_crtp_over_serial_frame(1, 0, &[]).is_empty());
    }

    #[test]
    fn encode_frame_round_trips_through_decoder() {
        let data = vec![0x10, 0x20, 0x30];
        let frame = encode_crtp_over_serial_frame(4, 1, &data);

        let mut decoder = crate::protocols::CrtpDecoder::new(4096);
        let packets = decoder.push(&frame);

        assert_eq!(
            packets.len(),
            1,
            "decoder should parse the frame, got 0 packets"
        );
        assert_eq!(packets[0].port, crate::protocols::crtp::CrtpPort::Logging);
        assert_eq!(packets[0].channel, 1);
        assert_eq!(packets[0].payload, data);
    }

    #[test]
    fn encode_frame_rejects_corrupted_crc() {
        let data = vec![0x01, 0x02, 0x03];
        let mut frame = encode_crtp_over_serial_frame(0, 0, &data);
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;
        let mut decoder = crate::protocols::CrtpDecoder::new(4096);
        assert!(decoder.push(&frame).is_empty());
    }

    #[test]
    fn parse_frame_extracts_port_and_channel() {
        let frame = encode_crtp_over_serial_frame(5, 2, &[0xAA, 0xBB]);
        let parsed = parse_crtp_over_serial_frame(&frame).unwrap();
        assert_eq!(parsed.port, 5);
        assert_eq!(parsed.channel, 2);
        assert_eq!(parsed.payload, vec![0xAA, 0xBB]);
    }

    #[test]
    fn parse_frame_rejects_short_data() {
        assert!(parse_crtp_over_serial_frame(&[0x00, 0x01]).is_err());
    }

    #[test]
    fn parse_frame_rejects_invalid_length() {
        let mut frame = vec![0x00, 0x80]; // length=128 > 63
        frame.extend_from_slice(&[0xAA]);
        assert!(parse_crtp_over_serial_frame(&frame).is_err());
    }

    #[test]
    fn round_trip_preserves_all_port_channel_combinations() {
        for port in 0..=7u8 {
            for channel in 0..=3u8 {
                let data = vec![port * 16 + channel];
                let frame = encode_crtp_over_serial_frame(port, channel, &data);
                let parsed = parse_crtp_over_serial_frame(&frame).unwrap();
                assert_eq!(parsed.port, port, "port mismatch for ({port}, {channel})");
                assert_eq!(
                    parsed.channel, channel,
                    "channel mismatch for ({port}, {channel})"
                );
                assert_eq!(parsed.payload, data);
            }
        }
    }
}
