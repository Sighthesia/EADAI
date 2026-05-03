#ifndef SELF_DESCRIBING_DEVICE_PORTABLE_H
#define SELF_DESCRIBING_DEVICE_PORTABLE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SDP_STATE_BOOT = 0,
    SDP_STATE_IDENTITY_SENT,
    SDP_STATE_COMMAND_CATALOG_SENT,
    SDP_STATE_VARIABLE_CATALOG_SENT,
    SDP_STATE_STREAMING
} sdp_state_t;

typedef struct sdp_driver_vtable {
    void *user;
    int (*tx_bytes)(void *user, const uint8_t *data, size_t len);
    uint32_t (*now_ms)(void *user);
    void (*lock)(void *user);
    void (*unlock)(void *user);
    void (*debug)(void *user, const char *msg);
} sdp_driver_vtable_t;

typedef struct {
    const char *device_name;
    const char *firmware_version;
    uint8_t protocol_version;
    uint32_t sample_rate_hz;
    uint16_t variable_count;
    uint16_t command_count;
    uint16_t sample_payload_len;
} sdp_identity_t;

typedef struct {
    const char *id;
    const char *params;
    const char *docs;
} sdp_command_descriptor_t;

typedef struct {
    const char *name;
    uint16_t order;
    const char *unit;
    uint8_t adjustable;
    uint8_t value_type;
} sdp_variable_descriptor_t;

typedef struct {
    sdp_state_t state;
    sdp_driver_vtable_t driver;
    sdp_identity_t identity;
    const sdp_command_descriptor_t *commands;
    size_t command_count;
    const sdp_variable_descriptor_t *variables;
    size_t variable_count;
    uint32_t sample_seq;
    uint8_t ack_rx[2];
    size_t ack_rx_len;
} sdp_device_t;

void sdp_init(sdp_device_t *dev,
              const sdp_driver_vtable_t *driver,
              const sdp_identity_t *identity,
              const sdp_command_descriptor_t *commands,
              size_t command_count,
              const sdp_variable_descriptor_t *variables,
              size_t variable_count);

int sdp_feed_byte(sdp_device_t *dev, uint8_t byte);
int sdp_send_identity(sdp_device_t *dev);
int sdp_send_command_catalog_page(sdp_device_t *dev, uint16_t page, uint16_t total_pages);
int sdp_send_variable_catalog_page(sdp_device_t *dev, uint16_t page, uint16_t total_pages);
int sdp_enter_streaming(sdp_device_t *dev);
int sdp_send_sample_frame(sdp_device_t *dev, const uint8_t *bitmap, size_t bitmap_len, const uint8_t *changed_values, size_t changed_len);

#ifdef __cplusplus
}
#endif

#endif
