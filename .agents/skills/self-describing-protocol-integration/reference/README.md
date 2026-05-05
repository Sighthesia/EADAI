# Portable self-describing device reference

This folder contains a small application-layer C reference implementation for the self-describing UART4 protocol.

## Contract

- Outer transport: `0x73 + len + payload`
- Identity first
- HostAck: exactly `0x04 + stage`
- Catalog pages: canonical host framing only
- Identity and catalog strings use canonical `u16 LE` length prefixes
- No `F3` wrapper
- Telemetry sample envelope: raw transport payload starts with `0x05`, followed by `seq (u32 LE)`, `bitmap_len (u16 LE)`, `bitmap`, and changed values
- Portable examples may use `driver.debug(user, msg)` to confirm the emitted frame type, sequence, and payload length without inventing ad hoc diagnostics

## Driver adapter

All platform details must be provided through `sdp_driver_vtable_t`.

Required hook:

- `tx_bytes(user, data, len)`

Optional hooks:

- `now_ms(user)`
- `lock(user)` / `unlock(user)`
- `debug(user, msg)`
