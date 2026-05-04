# Portable self-describing device reference

This folder contains a small application-layer C reference implementation for the self-describing UART4 protocol.

## Contract

- Outer transport: `0x73 + len + payload`
- Identity first
- HostAck: exactly `0x04 + stage`
- Catalog pages: canonical host framing only
- Identity and catalog strings use canonical `u16 LE` length prefixes
- No `F3` wrapper

## Driver adapter

All platform details must be provided through `sdp_driver_vtable_t`.

Required hook:

- `tx_bytes(user, data, len)`

Optional hooks:

- `now_ms(user)`
- `lock(user)` / `unlock(user)`
- `debug(user, msg)`
