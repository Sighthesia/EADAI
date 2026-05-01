/// BMI088 protocol constants and default field definitions.

pub const BMI088_SOF: [u8; 2] = [0xA5, 0x5A];
pub const BMI088_VERSION: u8 = 0x01;
pub const BMI088_FRAME_TYPE_REQUEST: u8 = 0x01;
pub const BMI088_FRAME_TYPE_RESPONSE: u8 = 0x02;
pub const BMI088_FRAME_TYPE_EVENT: u8 = 0x03;

pub const BMI088_CMD_ACK: u8 = 0x10;
pub const BMI088_CMD_START: u8 = 0x11;
pub const BMI088_CMD_STOP: u8 = 0x12;
pub const BMI088_CMD_REQ_SCHEMA: u8 = 0x13;
pub const BMI088_CMD_REQ_IDENTITY: u8 = 0x14;
pub const BMI088_CMD_REQ_TUNING: u8 = 0x26;
pub const BMI088_CMD_SET_TUNING: u8 = 0x27;
pub const BMI088_CMD_SHELL_EXEC: u8 = 0x28;
pub const BMI088_CMD_SCHEMA: u8 = 0x80;
pub const BMI088_CMD_SAMPLE: u8 = 0x81;
pub const BMI088_CMD_IDENTITY: u8 = 0x82;
pub const BMI088_CMD_SHELL_OUTPUT: u8 = 0x83;
pub const BMI088_SCHEMA_VERSION: u8 = 0x01;
pub const BMI088_FIELD_TYPE_I16: u8 = 0x01;

pub const BMI088_SAMPLE_FIELD_NAMES: [&str; 30] = [
    "acc_x",
    "acc_y",
    "acc_z",
    "gyro_x",
    "gyro_y",
    "gyro_z",
    "roll",
    "pitch",
    "yaw",
    // FIXME: The pasted contract lists 30 fields but only names 28 of them.
    "reserved_0",
    "reserved_1",
    "motor_left_rear_wheel",
    "motor_left_front_wheel",
    "motor_right_front_wheel",
    "motor_right_rear_wheel",
    "roll_correction_output",
    "pitch_correction_output",
    "yaw_correction_output",
    "throttle_correction_output",
    "roll_proportional_gain_x100",
    "roll_integral_gain_x100",
    "roll_derivative_gain_x100",
    "pitch_proportional_gain_x100",
    "pitch_integral_gain_x100",
    "pitch_derivative_gain_x100",
    "yaw_proportional_gain_x100",
    "yaw_integral_gain_x100",
    "yaw_derivative_gain_x100",
    "output_limit",
    "bench_test_throttle",
];
pub const BMI088_SAMPLE_UNITS: [&str; 30] = [
    "raw", "raw", "raw", "raw", "raw", "raw", "deg", "deg", "deg", "raw", "raw", "raw", "raw",
    "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw",
    "raw", "raw", "raw", "raw",
];
pub const BMI088_SAMPLE_SCALE_Q: [i8; 30] = [
    0, 0, 0, 0, 0, 0, -2, -2, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -2, -2, -2, -2, -2, -2, -2, -2, -2,
    0, 0,
];

pub const BMI088_HEADER_LEN: usize = 7;
pub const BMI088_CRC_LEN: usize = 2;
pub const BMI088_MIN_FRAME_LEN: usize = BMI088_HEADER_LEN + BMI088_CRC_LEN;
