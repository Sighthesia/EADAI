import type { UiMavlinkPacketPayload, UiCrtpPacketPayload } from '../types'

// ---------------------------------------------------------------------------
// MAVLink display helpers
// ---------------------------------------------------------------------------

export function buildMavlinkDisplayValue(packet: UiMavlinkPacketPayload): string {
  // Try to build a meaningful display value from semantic fields
  switch (packet.messageId) {
    case 0x0000: { // HEARTBEAT
      const type = packet.fields.type || 'unknown'
      const status = packet.fields.system_status || 'unknown'
      return `Heartbeat: ${type} (${status})`
    }
    case 0x0001: { // SYS_STATUS
      const voltage = packet.fields.voltage_battery || '?'
      const current = packet.fields.current_battery || '?'
      const remaining = packet.fields.battery_remaining || '?'
      return `SYS: ${voltage} ${current} (${remaining})`
    }
    case 0x0021: { // GPS_RAW_INT
      const fix = packet.fields.fix_type || 'unknown'
      const sats = packet.fields.satellites_visible || '?'
      return `GPS: ${fix} (${sats} sats)`
    }
    case 0x0039: { // GLOBAL_POSITION_INT
      const alt = packet.fields.alt || '?'
      const relAlt = packet.fields.relative_alt || '?'
      return `Pos: alt=${alt} rel=${relAlt}`
    }
    case 0x00BE: { // HIGHRES_IMU
      const xacc = packet.fields.xacc || '?'
      const yacc = packet.fields.yacc || '?'
      const zacc = packet.fields.zacc || '?'
      return `IMU: ${xacc} ${yacc} ${zacc}`
    }
    case 0x00CA: { // ATTITUDE
      const roll = packet.fields.roll || '?'
      const pitch = packet.fields.pitch || '?'
      const yaw = packet.fields.yaw || '?'
      return `Att: ${roll} ${pitch} ${yaw}`
    }
    case 0x00D0: { // BATTERY_STATUS
      const voltage = packet.fields.voltage || '?'
      const current = packet.fields.current || '?'
      const remaining = packet.fields.remaining || '?'
      return `Batt: ${voltage} ${current} (${remaining})`
    }
    case 0x0051: { // VFR_HUD
      const airspeed = packet.fields.airspeed || '?'
      const groundspeed = packet.fields.groundspeed || '?'
      const alt = packet.fields.alt || '?'
      return `HUD: as=${airspeed} gs=${groundspeed} alt=${alt}`
    }
    case 0x0002: { // SYSTEM_TIME
      const bootMs = packet.fields.time_boot_ms || '?'
      return `SysTime: boot=${bootMs}`
    }
    case 0x0033: { // ATTITUDE_QUATERNION
      const q1 = packet.fields.q1 || '?'
      const q2 = packet.fields.q2 || '?'
      const q3 = packet.fields.q3 || '?'
      const q4 = packet.fields.q4 || '?'
      return `AttQuat: q=${q1} ${q2} ${q3} ${q4}`
    }
    case 0x0035: { // LOCAL_POSITION_NED
      const x = packet.fields.x || '?'
      const y = packet.fields.y || '?'
      const z = packet.fields.z || '?'
      return `LocalPos: x=${x} y=${y} z=${z}`
    }
    case 0x0053: { // COMMAND_ACK
      const cmd = packet.fields.command || '?'
      const result = packet.fields.result || '?'
      return `CmdAck: ${cmd} ${result}`
    }
    case 0x00A0: { // RC_CHANNELS
      const rssi = packet.fields.rssi || '?'
      return `RC: rssi=${rssi}`
    }
    case 0x00C7: { // GPS_STATUS
      const count = packet.fields.satellite_count || '?'
      return `GPS Status: ${count} sats`
    }
    case 0x00C9: { // SCALED_PRESSURE
      const press = packet.fields.press_abs || '?'
      const temp = packet.fields.temperature || '?'
      return `Pressure: ${press} (${temp})`
    }
    case 0x00D1: { // AUTOPILOT_VERSION
      const fw = packet.fields.flight_sw_version || '?'
      return `Autopilot: fw=${fw}`
    }
    case 0x00FE: { // VIBRATION
      const vx = packet.fields.vibration_x || '?'
      const vy = packet.fields.vibration_y || '?'
      const vz = packet.fields.vibration_z || '?'
      return `Vib: ${vx} ${vy} ${vz}`
    }
    default: {
      // Fallback to generic display
      return `sys=${packet.systemId} comp=${packet.componentId}`
    }
  }
}

export function buildMavlinkConsoleText(packet: UiMavlinkPacketPayload): string {
  const messageIdHex = `0x${packet.messageId.toString(16).padStart(4, '0')}`

  // Build console text with semantic fields
  switch (packet.messageId) {
    case 0x0000: { // HEARTBEAT
      const type = packet.fields.type || 'unknown'
      const autopilot = packet.fields.autopilot || 'unknown'
      const status = packet.fields.system_status || 'unknown'
      const baseMode = packet.fields.base_mode || '?'
      const customMode = packet.fields.custom_mode || '?'
      return `HEARTBEAT ${messageIdHex} type=${type} autopilot=${autopilot} mode=${status} base=${baseMode} custom=${customMode} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0001: { // SYS_STATUS
      const voltage = packet.fields.voltage_battery || '?'
      const current = packet.fields.current_battery || '?'
      const remaining = packet.fields.battery_remaining || '?'
      const dropRate = packet.fields.drop_rate_comm || '?'
      return `SYS_STATUS ${messageIdHex} voltage=${voltage} current=${current} remaining=${remaining} drop_rate=${dropRate} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0021: { // GPS_RAW_INT
      const fix = packet.fields.fix_type || 'unknown'
      const lat = packet.fields.lat || '?'
      const lon = packet.fields.lon || '?'
      const alt = packet.fields.alt || '?'
      const sats = packet.fields.satellites_visible || '?'
      return `GPS_RAW_INT ${messageIdHex} fix=${fix} lat=${lat} lon=${lon} alt=${alt} sats=${sats} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0039: { // GLOBAL_POSITION_INT
      const lat = packet.fields.lat || '?'
      const lon = packet.fields.lon || '?'
      const alt = packet.fields.alt || '?'
      const relAlt = packet.fields.relative_alt || '?'
      const vx = packet.fields.vx || '?'
      const vy = packet.fields.vy || '?'
      const vz = packet.fields.vz || '?'
      const hdg = packet.fields.hdg || '?'
      return `GLOBAL_POSITION_INT ${messageIdHex} lat=${lat} lon=${lon} alt=${alt} rel_alt=${relAlt} vx=${vx} vy=${vy} vz=${vz} hdg=${hdg} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00BE: { // HIGHRES_IMU
      const xacc = packet.fields.xacc || '?'
      const yacc = packet.fields.yacc || '?'
      const zacc = packet.fields.zacc || '?'
      const xgyro = packet.fields.xgyro || '?'
      const ygyro = packet.fields.ygyro || '?'
      const zgyro = packet.fields.zgyro || '?'
      const xmag = packet.fields.xmag || '?'
      const ymag = packet.fields.ymag || '?'
      const zmag = packet.fields.zmag || '?'
      return `HIGHRES_IMU ${messageIdHex} acc=${xacc} ${yacc} ${zacc} gyro=${xgyro} ${ygyro} ${zgyro} mag=${xmag} ${ymag} ${zmag} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00CA: { // ATTITUDE
      const roll = packet.fields.roll || '?'
      const pitch = packet.fields.pitch || '?'
      const yaw = packet.fields.yaw || '?'
      const rollSpeed = packet.fields.rollspeed || '?'
      const pitchSpeed = packet.fields.pitchspeed || '?'
      const yawSpeed = packet.fields.yawspeed || '?'
      return `ATTITUDE ${messageIdHex} roll=${roll} pitch=${pitch} yaw=${yaw} rollspeed=${rollSpeed} pitchspeed=${pitchSpeed} yawspeed=${yawSpeed} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00D0: { // BATTERY_STATUS
      const function_ = packet.fields.battery_function || 'unknown'
      const type = packet.fields.battery_type || 'unknown'
      const temp = packet.fields.temperature || '?'
      const voltage = packet.fields.voltage || '?'
      const current = packet.fields.current || '?'
      const remaining = packet.fields.remaining || '?'
      return `BATTERY_STATUS ${messageIdHex} function=${function_} type=${type} temp=${temp} voltage=${voltage} current=${current} remaining=${remaining} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0051: { // VFR_HUD
      const airspeed = packet.fields.airspeed || '?'
      const groundspeed = packet.fields.groundspeed || '?'
      const heading = packet.fields.heading || '?'
      const throttle = packet.fields.throttle || '?'
      const alt = packet.fields.alt || '?'
      const climb = packet.fields.climb || '?'
      return `VFR_HUD ${messageIdHex} airspeed=${airspeed} groundspeed=${groundspeed} heading=${heading} throttle=${throttle} alt=${alt} climb=${climb} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0002: { // SYSTEM_TIME
      const timeUnix = packet.fields.time_unix_usec || '?'
      const bootMs = packet.fields.time_boot_ms || '?'
      return `SYSTEM_TIME ${messageIdHex} time_unix=${timeUnix} boot=${bootMs} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0033: { // ATTITUDE_QUATERNION
      const q1 = packet.fields.q1 || '?'
      const q2 = packet.fields.q2 || '?'
      const q3 = packet.fields.q3 || '?'
      const q4 = packet.fields.q4 || '?'
      const rollSpeed = packet.fields.rollspeed || '?'
      const pitchSpeed = packet.fields.pitchspeed || '?'
      const yawSpeed = packet.fields.yawspeed || '?'
      return `ATTITUDE_QUATERNION ${messageIdHex} q=${q1} ${q2} ${q3} ${q4} rollspeed=${rollSpeed} pitchspeed=${pitchSpeed} yawspeed=${yawSpeed} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0035: { // LOCAL_POSITION_NED
      const x = packet.fields.x || '?'
      const y = packet.fields.y || '?'
      const z = packet.fields.z || '?'
      const vx = packet.fields.vx || '?'
      const vy = packet.fields.vy || '?'
      const vz = packet.fields.vz || '?'
      return `LOCAL_POSITION_NED ${messageIdHex} x=${x} y=${y} z=${z} vx=${vx} vy=${vy} vz=${vz} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x0053: { // COMMAND_ACK
      const cmd = packet.fields.command || '?'
      const result = packet.fields.result || '?'
      return `COMMAND_ACK ${messageIdHex} command=${cmd} result=${result} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00A0: { // RC_CHANNELS
      const rssi = packet.fields.rssi || '?'
      const ch1 = packet.fields.ch1 || '?'
      const ch2 = packet.fields.ch2 || '?'
      const ch3 = packet.fields.ch3 || '?'
      const ch4 = packet.fields.ch4 || '?'
      return `RC_CHANNELS ${messageIdHex} ch1=${ch1} ch2=${ch2} ch3=${ch3} ch4=${ch4} rssi=${rssi} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00C7: { // GPS_STATUS
      const sats = packet.fields.satellites_visible || '?'
      const count = packet.fields.satellite_count || '?'
      return `GPS_STATUS ${messageIdHex} satellites=[${sats}] count=${count} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00C9: { // SCALED_PRESSURE
      const pressAbs = packet.fields.press_abs || '?'
      const pressDiff = packet.fields.press_diff || '?'
      const temp = packet.fields.temperature || '?'
      return `SCALED_PRESSURE ${messageIdHex} press_abs=${pressAbs} press_diff=${pressDiff} temp=${temp} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00D1: { // AUTOPILOT_VERSION
      const fw = packet.fields.flight_sw_version || '?'
      const mw = packet.fields.middleware_sw_version || '?'
      const os = packet.fields.os_sw_version || '?'
      return `AUTOPILOT_VERSION ${messageIdHex} flight_sw=${fw} middleware_sw=${mw} os_sw=${os} sys=${packet.systemId} comp=${packet.componentId}`
    }
    case 0x00FE: { // VIBRATION
      const vx = packet.fields.vibration_x || '?'
      const vy = packet.fields.vibration_y || '?'
      const vz = packet.fields.vibration_z || '?'
      const c0 = packet.fields.clipping_0 || '?'
      const c1 = packet.fields.clipping_1 || '?'
      const c2 = packet.fields.clipping_2 || '?'
      return `VIBRATION ${messageIdHex} vib=${vx} ${vy} ${vz} clip=${c0} ${c1} ${c2} sys=${packet.systemId} comp=${packet.componentId}`
    }
    default: {
      // Fallback to generic display
      return `MAVLink ${messageIdHex} sys=${packet.systemId} comp=${packet.componentId}`
    }
  }
}

// ---------------------------------------------------------------------------
// CRTP display helpers
// ---------------------------------------------------------------------------

export function buildCrtpDisplayValue(packet: UiCrtpPacketPayload): string {
  const port = packet.port

  // Try to build a meaningful display value from semantic fields
  switch (port) {
    case 'console': {
      const text = packet.fields.text || ''
      // Truncate long text for display
      const truncated = text.length > 50 ? text.substring(0, 50) + '...' : text
      return `Console: ${truncated}`
    }
    case 'parameter': {
      const operation = packet.fields.operation || 'unknown'
      const paramId = packet.fields.param_id || '?'
      const paramValue = packet.fields.param_value || ''
      if (operation === 'read') {
        return `Param read: id=${paramId}`
      } else if (operation === 'write') {
        return `Param write: id=${paramId} value=${paramValue}`
      }
      return `Param: ${operation}`
    }
    case 'commander': {
      const controlMode = packet.fields.control_mode || 'unknown'
      if (controlMode === 'rpyt') {
        const roll = packet.fields.roll || '?'
        const pitch = packet.fields.pitch || '?'
        const yaw = packet.fields.yaw || '?'
        const thrust = packet.fields.thrust || '?'
        return `Cmd RPYT: r=${roll} p=${pitch} y=${yaw} t=${thrust}`
      } else if (controlMode === 'alt_hold') {
        const height = packet.fields.height || '?'
        return `Cmd AltHold: h=${height}`
      } else if (controlMode === 'velocity') {
        const vx = packet.fields.vx || '?'
        const vy = packet.fields.vy || '?'
        return `Cmd Vel: vx=${vx} vy=${vy}`
      }
      return `Cmd: ${controlMode}`
    }
    case 'logging': {
      const logType = packet.fields.log_type || 'unknown'
      if (logType === 'data') {
        const logChannel = packet.fields.log_channel || '?'
        const logId = packet.fields.log_id || '?'
        return `Log data: ch=${logChannel} id=${logId}`
      } else if (logType === 'control') {
        const command = packet.fields.command || '?'
        return `Log control: ${command}`
      }
      return `Log: ${logType}`
    }
    case 'high_level_commander': {
      const commandType = packet.fields.command_type || 'unknown'
      const command = packet.fields.command || '?'
      if (commandType === 'trajectory') {
        const x = packet.fields.x || '?'
        const y = packet.fields.y || '?'
        const z = packet.fields.z || '?'
        return `HL Cmd: ${command} x=${x} y=${y} z=${z}`
      }
      return `HL Cmd: ${command}`
    }
    case 'memory': {
      const operation = packet.fields.operation || 'unknown'
      const cmd = packet.fields.memory_cmd || '?'
      return `Mem: ${operation} ${cmd}`
    }
    case 'setting': {
      const operation = packet.fields.operation || 'unknown'
      const settingId = packet.fields.setting_id || '?'
      return `Setting: ${operation} id=${settingId}`
    }
    case 'debug': {
      const text = packet.fields.text || ''
      const truncated = text.length > 50 ? text.substring(0, 50) + '...' : text
      return `Debug: ${truncated}`
    }
    default: {
      // Fallback to generic display
      return `ch=${packet.channel}`
    }
  }
}

export function buildCrtpConsoleText(packet: UiCrtpPacketPayload): string {
  const port = packet.port

  // Build console text with semantic fields
  switch (port) {
    case 'console': {
      const text = packet.fields.text || ''
      return `CRTP Console: ${text}`
    }
    case 'parameter': {
      const operation = packet.fields.operation || 'unknown'
      const paramId = packet.fields.param_id || '?'
      const paramValue = packet.fields.param_value || ''
      const tocCmd = packet.fields.toc_cmd || ''
      if (operation === 'read') {
        return `CRTP Parameter read: id=${paramId}`
      } else if (operation === 'write') {
        return `CRTP Parameter write: id=${paramId} value=${paramValue}`
      } else if (operation === 'toc_info') {
        return `CRTP Parameter TOC: cmd=${tocCmd}`
      }
      return `CRTP Parameter: ${operation}`
    }
    case 'commander': {
      const controlMode = packet.fields.control_mode || 'unknown'
      if (controlMode === 'rpyt') {
        const roll = packet.fields.roll || '?'
        const pitch = packet.fields.pitch || '?'
        const yaw = packet.fields.yaw || '?'
        const thrust = packet.fields.thrust || '?'
        return `CRTP Commander RPYT: roll=${roll} pitch=${pitch} yaw=${yaw} thrust=${thrust}`
      } else if (controlMode === 'alt_hold') {
        const height = packet.fields.height || '?'
        return `CRTP Commander AltHold: height=${height}`
      } else if (controlMode === 'velocity') {
        const vx = packet.fields.vx || '?'
        const vy = packet.fields.vy || '?'
        const yawRate = packet.fields.yaw_rate || '?'
        return `CRTP Commander Velocity: vx=${vx} vy=${vy} yaw_rate=${yawRate}`
      } else if (controlMode === 'high_level') {
        const command = packet.fields.command || '?'
        return `CRTP Commander HighLevel: ${command}`
      }
      return `CRTP Commander: ${controlMode}`
    }
    case 'logging': {
      const logType = packet.fields.log_type || 'unknown'
      if (logType === 'data') {
        const logChannel = packet.fields.log_channel || '?'
        const logId = packet.fields.log_id || '?'
        return `CRTP Logging data: channel=${logChannel} id=${logId}`
      } else if (logType === 'control') {
        const command = packet.fields.command || '?'
        return `CRTP Logging control: ${command}`
      }
      return `CRTP Logging: ${logType}`
    }
    case 'high_level_commander': {
      const commandType = packet.fields.command_type || 'unknown'
      const command = packet.fields.command || '?'
      if (commandType === 'trajectory') {
        const x = packet.fields.x || '?'
        const y = packet.fields.y || '?'
        const z = packet.fields.z || '?'
        return `CRTP HighLevelCmd trajectory: ${command} x=${x} y=${y} z=${z}`
      }
      return `CRTP HighLevelCmd: ${command}`
    }
    case 'memory': {
      const operation = packet.fields.operation || 'unknown'
      const cmd = packet.fields.memory_cmd || '?'
      const status = packet.fields.status || '?'
      return `CRTP Memory ${operation}: cmd=${cmd} status=${status}`
    }
    case 'setting': {
      const operation = packet.fields.operation || 'unknown'
      const settingId = packet.fields.setting_id || '?'
      const value = packet.fields.value || '?'
      return `CRTP Setting ${operation}: id=${settingId} value=${value}`
    }
    case 'debug': {
      const text = packet.fields.text || ''
      return `CRTP Debug: ${text}`
    }
    default: {
      // Fallback to generic display
      return `CRTP port=${packet.port} ch=${packet.channel}`
    }
  }
}
