#include "self_describing_device_portable.h"

#include <string.h>

#define SDP_FRAME_HEADER 0x73u
#define SDP_FRAME_TYPE_IDENTITY 0x01u
#define SDP_FRAME_TYPE_VARIABLE_CATALOG_PAGE 0x02u
#define SDP_FRAME_TYPE_COMMAND_CATALOG_PAGE 0x03u
#define SDP_FRAME_TYPE_HOST_ACK 0x04u
#define SDP_FRAME_TYPE_TELEMETRY_SAMPLE 0x05u

#define SDP_ACK_STAGE_IDENTITY 0x01u
#define SDP_ACK_STAGE_COMMAND 0x02u
#define SDP_ACK_STAGE_VARIABLE 0x03u

// FIXME: These page and payload limits are reference-friendly bounds; trim or
// split them if a target transport or flash budget needs smaller frames.
#define SDP_MAX_PAYLOAD_SIZE 255u
// FIXME: These per-page limits are reference-friendly bounds; adapt them to the
// device's catalog size and transport budget if the target requires different paging.
#define SDP_MAX_COMMANDS_PER_PAGE 16u
#define SDP_MAX_VARIABLES_PER_PAGE 32u

static int sdp_tx_frame(sdp_device_t *dev, const uint8_t *payload, size_t payload_len) {
    uint8_t frame[2 + SDP_MAX_PAYLOAD_SIZE];

    if (dev == NULL || payload == NULL || payload_len > SDP_MAX_PAYLOAD_SIZE) {
        return -1;
    }

    frame[0] = SDP_FRAME_HEADER;
    frame[1] = (uint8_t)payload_len;
    if (payload_len > 0) {
        memcpy(&frame[2], payload, payload_len);
    }

    if (dev->driver.lock) {
        dev->driver.lock(dev->driver.user);
    }

    if (dev->driver.tx_bytes) {
        int rc = dev->driver.tx_bytes(dev->driver.user, frame, payload_len + 2u);
        if (dev->driver.unlock) {
            dev->driver.unlock(dev->driver.user);
        }
        return rc;
    }

    if (dev->driver.unlock) {
        dev->driver.unlock(dev->driver.user);
    }
    return -1;
}

static size_t sdp_strlen(const char *s) {
    size_t n = 0;
    if (s == NULL) {
        return 0;
    }
    while (s[n] != '\0') {
        ++n;
    }
    return n;
}

static int sdp_put_u16(uint8_t *buf, size_t cap, size_t *cursor, uint16_t v) {
    if (*cursor + 2u > cap) {
        return -1;
    }
    buf[(*cursor)++] = (uint8_t)(v & 0xFFu);
    buf[(*cursor)++] = (uint8_t)(v >> 8);
    return 0;
}

static int sdp_put_u32(uint8_t *buf, size_t cap, size_t *cursor, uint32_t v) {
    if (*cursor + 4u > cap) {
        return -1;
    }
    buf[(*cursor)++] = (uint8_t)(v & 0xFFu);
    buf[(*cursor)++] = (uint8_t)((v >> 8) & 0xFFu);
    buf[(*cursor)++] = (uint8_t)((v >> 16) & 0xFFu);
    buf[(*cursor)++] = (uint8_t)((v >> 24) & 0xFFu);
    return 0;
}

static int sdp_put_string(uint8_t *buf, size_t cap, size_t *cursor, const char *s) {
    size_t len = sdp_strlen(s);
    if (len > 255u || *cursor + 1u + len > cap) {
        return -1;
    }
    buf[(*cursor)++] = (uint8_t)len;
    if (len > 0) {
        memcpy(&buf[*cursor], s, len);
        *cursor += len;
    }
    return 0;
}

static int sdp_build_identity(uint8_t *payload, size_t cap, const sdp_identity_t *identity) {
    size_t cursor = 0;
    if (cap < 1u) {
        return -1;
    }
    payload[cursor++] = SDP_FRAME_TYPE_IDENTITY;
    if (cursor + 1u > cap) {
        return -1;
    }
    payload[cursor++] = identity->protocol_version;
    if (sdp_put_string(payload, cap, &cursor, identity->device_name) != 0) {
        return -1;
    }
    if (sdp_put_string(payload, cap, &cursor, identity->firmware_version) != 0) {
        return -1;
    }
    if (sdp_put_u32(payload, cap, &cursor, identity->sample_rate_hz) != 0) {
        return -1;
    }
    if (sdp_put_u16(payload, cap, &cursor, identity->variable_count) != 0) {
        return -1;
    }
    if (sdp_put_u16(payload, cap, &cursor, identity->command_count) != 0) {
        return -1;
    }
    if (sdp_put_u16(payload, cap, &cursor, identity->sample_payload_len) != 0) {
        return -1;
    }
    return (int)cursor;
}

static int sdp_build_catalog_page(uint8_t *payload,
                                  size_t cap,
                                  uint8_t frame_type,
                                  uint16_t page,
                                  uint16_t total_pages,
                                  uint8_t count) {
    size_t cursor = 0;
    if (cap < 5u) {
        return -1;
    }
    payload[cursor++] = frame_type;
    if (sdp_put_u16(payload, cap, &cursor, page) != 0) {
        return -1;
    }
    if (sdp_put_u16(payload, cap, &cursor, total_pages) != 0) {
        return -1;
    }
    payload[cursor++] = count;
    return (int)cursor;
}

static int sdp_append_descriptor_bytes(uint8_t *payload,
                                       size_t cap,
                                       size_t *cursor,
                                       const char *a,
                                       const char *b,
                                       const char *c) {
    if (sdp_put_string(payload, cap, cursor, a) != 0) {
        return -1;
    }
    if (sdp_put_string(payload, cap, cursor, b) != 0) {
        return -1;
    }
    if (sdp_put_string(payload, cap, cursor, c) != 0) {
        return -1;
    }
    return 0;
}

void sdp_init(sdp_device_t *dev,
              const sdp_driver_vtable_t *driver,
              const sdp_identity_t *identity,
              const sdp_command_descriptor_t *commands,
              size_t command_count,
              const sdp_variable_descriptor_t *variables,
              size_t variable_count) {
    if (dev == NULL) {
        return;
    }
    memset(dev, 0, sizeof(*dev));
    if (driver != NULL) {
        dev->driver = *driver;
    }
    if (identity != NULL) {
        dev->identity = *identity;
    }
    dev->commands = commands;
    dev->command_count = command_count;
    dev->variables = variables;
    dev->variable_count = variable_count;
    dev->state = SDP_STATE_BOOT;
}

int sdp_feed_byte(sdp_device_t *dev, uint8_t byte) {
    if (dev == NULL) {
        return -1;
    }
    dev->ack_rx[dev->ack_rx_len++] = byte;
    if (dev->ack_rx_len < 2u) {
        return 0;
    }
    dev->ack_rx_len = 0;

    if (dev->ack_rx[0] != SDP_FRAME_TYPE_HOST_ACK) {
        return -1;
    }

    if (dev->ack_rx[1] == SDP_ACK_STAGE_IDENTITY && dev->state == SDP_STATE_IDENTITY_SENT) {
        dev->state = SDP_STATE_COMMAND_CATALOG_SENT;
        return 0;
    }
    if (dev->ack_rx[1] == SDP_ACK_STAGE_COMMAND && dev->state == SDP_STATE_COMMAND_CATALOG_SENT) {
        dev->state = SDP_STATE_VARIABLE_CATALOG_SENT;
        return 0;
    }
    if (dev->ack_rx[1] == SDP_ACK_STAGE_VARIABLE && dev->state == SDP_STATE_VARIABLE_CATALOG_SENT) {
        dev->state = SDP_STATE_STREAMING;
        return 0;
    }
    return -1;
}

int sdp_send_identity(sdp_device_t *dev) {
    uint8_t payload[64];
    if (dev == NULL) {
        return -1;
    }
    int len = sdp_build_identity(payload, sizeof(payload), &dev->identity);
    if (len < 0) {
        return len;
    }
    if (sdp_tx_frame(dev, payload, (size_t)len) != 0) {
        return -1;
    }
    dev->state = SDP_STATE_IDENTITY_SENT;
    return 0;
}

int sdp_send_command_catalog_page(sdp_device_t *dev, uint16_t page, uint16_t total_pages) {
    uint8_t payload[255];
    size_t cursor;
    size_t start;
    size_t page_start = 0;
    size_t page_end;
    uint8_t count;

    if (dev == NULL || (dev->command_count > 0u && dev->commands == NULL)) {
        return -1;
    }
    if (page > 0xFFFFu || total_pages == 0u) {
        return -1;
    }

    count = (uint8_t)((dev->command_count > SDP_MAX_COMMANDS_PER_PAGE) ? SDP_MAX_COMMANDS_PER_PAGE : dev->command_count);
    start = (size_t)page * SDP_MAX_COMMANDS_PER_PAGE;
    if (start >= dev->command_count) {
        count = 0;
    } else if (start + count > dev->command_count) {
        count = (uint8_t)(dev->command_count - start);
    }

    cursor = (size_t)sdp_build_catalog_page(payload, sizeof(payload), 0x03u, page, total_pages, count);
    if ((int)cursor < 0) {
        return -1;
    }

    page_end = start + count;
    for (page_start = start; page_start < page_end; ++page_start) {
        const sdp_command_descriptor_t *cmd = &dev->commands[page_start];
        if (sdp_append_descriptor_bytes(payload, sizeof(payload), &cursor, cmd->id, cmd->params, cmd->docs) != 0) {
            return -1;
        }
    }

    if (sdp_tx_frame(dev, payload, cursor) != 0) {
        return -1;
    }
    dev->state = SDP_STATE_COMMAND_CATALOG_SENT;
    if (dev->driver.debug) {
        dev->driver.debug(dev->driver.user, "sdp: command catalog sent");
    }
    return 0;
}

int sdp_send_variable_catalog_page(sdp_device_t *dev, uint16_t page, uint16_t total_pages) {
    uint8_t payload[255];
    size_t cursor;
    size_t start;
    size_t idx;
    uint8_t count;

    if (dev == NULL || (dev->variable_count > 0u && dev->variables == NULL)) {
        return -1;
    }
    if (page > 0xFFFFu || total_pages == 0u) {
        return -1;
    }

    count = (uint8_t)((dev->variable_count > SDP_MAX_VARIABLES_PER_PAGE) ? SDP_MAX_VARIABLES_PER_PAGE : dev->variable_count);
    start = (size_t)page * SDP_MAX_VARIABLES_PER_PAGE;
    if (start >= dev->variable_count) {
        count = 0;
    } else if (start + count > dev->variable_count) {
        count = (uint8_t)(dev->variable_count - start);
    }

    cursor = (size_t)sdp_build_catalog_page(payload, sizeof(payload), 0x02u, page, total_pages, count);
    if ((int)cursor < 0) {
        return -1;
    }

    for (idx = start; idx < start + count; ++idx) {
        const sdp_variable_descriptor_t *var = &dev->variables[idx];
        if (sdp_put_string(payload, sizeof(payload), &cursor, var->name) != 0) {
            return -1;
        }
        if (sdp_put_u16(payload, sizeof(payload), &cursor, var->order) != 0) {
            return -1;
        }
        if (sdp_put_string(payload, sizeof(payload), &cursor, var->unit) != 0) {
            return -1;
        }
        if (cursor + 2u > sizeof(payload)) {
            return -1;
        }
        payload[cursor++] = var->adjustable ? 1u : 0u;
        payload[cursor++] = var->value_type;
    }

    if (sdp_tx_frame(dev, payload, cursor) != 0) {
        return -1;
    }
    dev->state = SDP_STATE_VARIABLE_CATALOG_SENT;
    if (dev->driver.debug) {
        dev->driver.debug(dev->driver.user, "sdp: variable catalog sent");
    }
    return 0;
}

int sdp_enter_streaming(sdp_device_t *dev) {
    if (dev == NULL) {
        return -1;
    }
    if (dev->state != SDP_STATE_VARIABLE_CATALOG_SENT) {
        return -1;
    }
    dev->state = SDP_STATE_STREAMING;
    return 0;
}

int sdp_send_sample_frame(sdp_device_t *dev, const uint8_t *bitmap, size_t bitmap_len, const uint8_t *changed_values, size_t changed_len) {
    uint8_t payload[255];
    size_t cursor = 0;

    if (dev == NULL || (bitmap_len > 0u && bitmap == NULL) || (changed_len > 0u && changed_values == NULL)) {
        return -1;
    }
    if (dev->state != SDP_STATE_STREAMING) {
        return -1;
    }

    payload[cursor++] = SDP_FRAME_TYPE_TELEMETRY_SAMPLE;
    if (sdp_put_u32(payload, sizeof(payload), &cursor, dev->sample_seq++) != 0) {
        return -1;
    }
    if (bitmap_len > 0xFFFFu) {
        return -1;
    }
    if (sdp_put_u16(payload, sizeof(payload), &cursor, (uint16_t)bitmap_len) != 0) {
        return -1;
    }
    if (cursor + bitmap_len + changed_len > sizeof(payload)) {
        return -1;
    }
    if (bitmap_len > 0u) {
        memcpy(&payload[cursor], bitmap, bitmap_len);
        cursor += bitmap_len;
    }
    if (changed_len > 0u) {
        memcpy(&payload[cursor], changed_values, changed_len);
        cursor += changed_len;
    }

    return sdp_tx_frame(dev, payload, cursor);
}
