import type {
  ImuAttitudeMap,
  ImuAttitudeRole,
  ImuChannelMap,
  ImuChannelRole,
  ImuQuaternionMap,
  ImuQuaternionRole,
  VariableEntry,
} from '../types'

export const IMU_CHANNEL_ROLES: ImuChannelRole[] = ['accelX', 'accelY', 'accelZ', 'gyroX', 'gyroY', 'gyroZ']

export const IMU_ROLE_LABELS: Record<ImuChannelRole, string> = {
  accelX: 'Accel X',
  accelY: 'Accel Y',
  accelZ: 'Accel Z',
  gyroX: 'Gyro X',
  gyroY: 'Gyro Y',
  gyroZ: 'Gyro Z',
}

export const IMU_ATTITUDE_ROLES: ImuAttitudeRole[] = ['roll', 'pitch', 'yaw']

export const IMU_ATTITUDE_LABELS: Record<ImuAttitudeRole, string> = {
  roll: 'Roll',
  pitch: 'Pitch',
  yaw: 'Yaw',
}

export const IMU_QUATERNION_ROLES: ImuQuaternionRole[] = ['quatW', 'quatX', 'quatY', 'quatZ']

export const IMU_QUATERNION_LABELS: Record<ImuQuaternionRole, string> = {
  quatW: 'Quat W',
  quatX: 'Quat X',
  quatY: 'Quat Y',
  quatZ: 'Quat Z',
}

const ROLE_ALIASES: Record<ImuChannelRole, string[]> = {
  accelX: ['ax', 'accx', 'accelx', 'accelerometerx', 'imuax', 'imuaccx', 'imuaccelx'],
  accelY: ['ay', 'accy', 'accely', 'accelerometery', 'imuay', 'imuaccy', 'imuaccely'],
  accelZ: ['az', 'accz', 'accelz', 'accelerometerz', 'imuaz', 'imuaccz', 'imuaccelz'],
  gyroX: ['gx', 'gyrox', 'angularvelocityx', 'imugx', 'imugyrox'],
  gyroY: ['gy', 'gyroy', 'angularvelocityy', 'imugy', 'imugyroy'],
  gyroZ: ['gz', 'gyroz', 'angularvelocityz', 'imugz', 'imugyroz'],
}

const ATTITUDE_ALIASES: Record<ImuAttitudeRole, string[]> = {
  roll: ['roll', 'rollangle', 'eulerroll', 'imur', 'imuroll', 'fusedroll', 'imufusedroll'],
  pitch: ['pitch', 'pitchangle', 'eulerpitch', 'imupitch', 'fusedpitch', 'imufusedpitch'],
  yaw: ['yaw', 'heading', 'yawangle', 'euleryaw', 'imuyaw', 'fusedyaw', 'imufusedyaw'],
}

const QUATERNION_ALIASES: Record<ImuQuaternionRole, string[]> = {
  quatW: ['qw', 'quatw', 'quaternionw', 'imuqw', 'imuquatw', 'fusedqw', 'fusedquatw', 'imufusedqw'],
  quatX: ['qx', 'quatx', 'quaternionx', 'imuqx', 'imuquatx', 'fusedqx', 'fusedquatx', 'imufusedqx'],
  quatY: ['qy', 'quaty', 'quaterniony', 'imuqy', 'imuquaty', 'fusedqy', 'fusedquaty', 'imufusedqy'],
  quatZ: ['qz', 'quatz', 'quaternionz', 'imuqz', 'imuquatz', 'fusedqz', 'fusedquatz', 'imufusedqz'],
}

export const createEmptyImuChannelMap = (): ImuChannelMap => ({
  accelX: null,
  accelY: null,
  accelZ: null,
  gyroX: null,
  gyroY: null,
  gyroZ: null,
})

export const createEmptyImuAttitudeMap = (): ImuAttitudeMap => ({
  roll: null,
  pitch: null,
  yaw: null,
})

export const createEmptyImuQuaternionMap = (): ImuQuaternionMap => ({
  quatW: null,
  quatX: null,
  quatY: null,
  quatZ: null,
})

export const autoDetectImuChannelMap = (
  variables: Record<string, VariableEntry>,
  currentMap: ImuChannelMap,
): ImuChannelMap => {
  const channels = Object.keys(variables)
  const normalizedChannels = channels.map((channel) => ({ channel, normalized: normalizeChannelName(channel) }))
  const usedChannels = new Set<string>()
  const nextMap = createEmptyImuChannelMap()

  for (const role of IMU_CHANNEL_ROLES) {
    const currentChannel = currentMap[role]
    if (currentChannel && variables[currentChannel]) {
      nextMap[role] = currentChannel
      usedChannels.add(currentChannel)
    }
  }

  for (const role of IMU_CHANNEL_ROLES) {
    if (nextMap[role]) {
      continue
    }

    let bestChannel: string | null = null
    let bestScore = -1
    for (const entry of normalizedChannels) {
      if (usedChannels.has(entry.channel)) {
        continue
      }

      const score = scoreChannelForRole(entry.normalized, role)
      if (score > bestScore) {
        bestScore = score
        bestChannel = entry.channel
      }
    }

    if (bestChannel && bestScore > 0) {
      nextMap[role] = bestChannel
      usedChannels.add(bestChannel)
    }
  }

  return nextMap
}

export const autoDetectImuAttitudeMap = (
  variables: Record<string, VariableEntry>,
  currentMap: ImuAttitudeMap,
): ImuAttitudeMap => {
  const channels = Object.keys(variables)
  const normalizedChannels = channels.map((channel) => ({ channel, normalized: normalizeChannelName(channel) }))
  const usedChannels = new Set<string>()
  const nextMap = createEmptyImuAttitudeMap()

  for (const role of IMU_ATTITUDE_ROLES) {
    const currentChannel = currentMap[role]
    if (currentChannel && variables[currentChannel]) {
      nextMap[role] = currentChannel
      usedChannels.add(currentChannel)
    }
  }

  for (const role of IMU_ATTITUDE_ROLES) {
    if (nextMap[role]) {
      continue
    }

    let bestChannel: string | null = null
    let bestScore = -1
    for (const entry of normalizedChannels) {
      if (usedChannels.has(entry.channel)) {
        continue
      }

      const score = scoreAttitudeChannel(entry.normalized, role)
      if (score > bestScore) {
        bestScore = score
        bestChannel = entry.channel
      }
    }

    if (bestChannel && bestScore > 0) {
      nextMap[role] = bestChannel
      usedChannels.add(bestChannel)
    }
  }

  return nextMap
}

export const autoDetectImuQuaternionMap = (
  variables: Record<string, VariableEntry>,
  currentMap: ImuQuaternionMap,
): ImuQuaternionMap => {
  const channels = Object.keys(variables)
  const normalizedChannels = channels.map((channel) => ({ channel, normalized: normalizeChannelName(channel) }))
  const usedChannels = new Set<string>()
  const nextMap = createEmptyImuQuaternionMap()

  for (const role of IMU_QUATERNION_ROLES) {
    const currentChannel = currentMap[role]
    if (currentChannel && variables[currentChannel]) {
      nextMap[role] = currentChannel
      usedChannels.add(currentChannel)
    }
  }

  for (const role of IMU_QUATERNION_ROLES) {
    if (nextMap[role]) {
      continue
    }

    let bestChannel: string | null = null
    let bestScore = -1
    for (const entry of normalizedChannels) {
      if (usedChannels.has(entry.channel)) {
        continue
      }

      const score = scoreQuaternionChannel(entry.normalized, role)
      if (score > bestScore) {
        bestScore = score
        bestChannel = entry.channel
      }
    }

    if (bestChannel && bestScore > 0) {
      nextMap[role] = bestChannel
      usedChannels.add(bestChannel)
    }
  }

  return nextMap
}

export const hasMappedImuChannel = (map: ImuChannelMap) => IMU_CHANNEL_ROLES.some((role) => Boolean(map[role]))

export const countMappedImuChannels = (map: ImuChannelMap) =>
  IMU_CHANNEL_ROLES.reduce((count, role) => count + (map[role] ? 1 : 0), 0)

export const countMappedImuAttitudeChannels = (map: ImuAttitudeMap) =>
  IMU_ATTITUDE_ROLES.reduce((count, role) => count + (map[role] ? 1 : 0), 0)

export const countMappedImuQuaternionChannels = (map: ImuQuaternionMap) =>
  IMU_QUATERNION_ROLES.reduce((count, role) => count + (map[role] ? 1 : 0), 0)

export const inferFusedQuaternionMapFromRawMap = (map: ImuChannelMap): ImuQuaternionMap => {
  const sensorLabel = inferImuSensorLabelFromRawMap(map)
  if (!sensorLabel) {
    return createEmptyImuQuaternionMap()
  }

  return {
    quatW: `${sensorLabel}_fused_qw`,
    quatX: `${sensorLabel}_fused_qx`,
    quatY: `${sensorLabel}_fused_qy`,
    quatZ: `${sensorLabel}_fused_qz`,
  }
}

const normalizeChannelName = (value: string) => value.toLowerCase().replace(/[^a-z0-9]/g, '')

const inferImuSensorLabelFromRawMap = (map: ImuChannelMap) => {
  const candidates = Object.values(map)
    .map((channel) => (channel ? inferSensorLabelFromChannel(channel) : null))
    .filter((value): value is string => Boolean(value))

  if (candidates.length === 0) {
    return null
  }

  const counts = new Map<string, number>()
  for (const candidate of candidates) {
    counts.set(candidate, (counts.get(candidate) ?? 0) + 1)
  }

  return [...counts.entries()].sort((left, right) => right[1] - left[1])[0]?.[0] ?? null
}

const inferSensorLabelFromChannel = (channel: string) => {
  const normalized = normalizeChannelName(channel)

  for (const aliases of Object.values(ROLE_ALIASES)) {
    for (const alias of aliases) {
      if (normalized === alias || normalized.endsWith(alias)) {
        const prefix = normalized.slice(0, normalized.length - alias.length)
        if (!prefix) {
          return 'imu'
        }
        return prefix.endsWith('imu') ? prefix : `${prefix}_imu`
      }
    }
  }

  return null
}

const scoreChannelForRole = (normalizedChannel: string, role: ImuChannelRole) => {
  let bestScore = 0

  for (const alias of ROLE_ALIASES[role]) {
    if (normalizedChannel === alias) {
      return 120
    }
    if (normalizedChannel.endsWith(alias)) {
      bestScore = Math.max(bestScore, 80)
    }
    if (normalizedChannel.includes(alias)) {
      bestScore = Math.max(bestScore, 40)
    }
  }

  if (bestScore > 0 && normalizedChannel.includes('imu')) {
    return bestScore + 10
  }

  return bestScore
}

const scoreAttitudeChannel = (normalizedChannel: string, role: ImuAttitudeRole) => {
  let bestScore = 0

  for (const alias of ATTITUDE_ALIASES[role]) {
    if (normalizedChannel === alias) {
      return 120
    }
    if (normalizedChannel.endsWith(alias)) {
      bestScore = Math.max(bestScore, 80)
    }
    if (normalizedChannel.includes(alias)) {
      bestScore = Math.max(bestScore, 40)
    }
  }

  if (bestScore > 0 && normalizedChannel.includes('imu')) {
    return bestScore + 10
  }

  return bestScore
}

const scoreQuaternionChannel = (normalizedChannel: string, role: ImuQuaternionRole) => {
  let bestScore = 0

  for (const alias of QUATERNION_ALIASES[role]) {
    if (normalizedChannel === alias) {
      return 120
    }
    if (normalizedChannel.endsWith(alias)) {
      bestScore = Math.max(bestScore, 80)
    }
    if (normalizedChannel.includes(alias)) {
      bestScore = Math.max(bestScore, 40)
    }
  }

  if (bestScore > 0 && normalizedChannel.includes('imu')) {
    return bestScore + 10
  }

  return bestScore
}
