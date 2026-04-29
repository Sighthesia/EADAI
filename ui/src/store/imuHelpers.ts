import type {
  ImuAttitudeMap,
  ImuAttitudeRole,
  ImuChannelMap,
  ImuChannelRole,
  ImuOrientationSource,
  ImuQualitySnapshot,
  ImuQuaternionMap,
  ImuQuaternionRole,
  VariableEntry,
} from '../types'

export const isSameImuChannelMap = (left: ImuChannelMap, right: ImuChannelMap) =>
  Object.keys(left).every((key) => left[key as ImuChannelRole] === right[key as ImuChannelRole])

export const isSameImuAttitudeMap = (left: ImuAttitudeMap, right: ImuAttitudeMap) =>
  Object.keys(left).every((key) => left[key as ImuAttitudeRole] === right[key as ImuAttitudeRole])

export const isSameImuQuaternionMap = (left: ImuQuaternionMap, right: ImuQuaternionMap) =>
  Object.keys(left).every((key) => left[key as ImuQuaternionRole] === right[key as ImuQuaternionRole])

export const isSameImuQualitySnapshot = (left: ImuQualitySnapshot, right: ImuQualitySnapshot) =>
  left.level === right.level && left.label === right.label && left.details === right.details && left.timestampMs === right.timestampMs

export const computeImuQuality = (
  variables: Record<string, VariableEntry>,
  state: { imuChannelMap: ImuChannelMap; imuQuaternionMap: ImuQuaternionMap; imuAttitudeMap: ImuAttitudeMap; imuOrientationSource: ImuOrientationSource },
) => {
  const imuRawCount = ['accelX', 'accelY', 'accelZ', 'gyroX', 'gyroY', 'gyroZ'].filter((role) => {
    const key = role as keyof typeof state.imuChannelMap
    return Boolean(state.imuChannelMap[key])
  }).length
  const quaternionCount = ['quatW', 'quatX', 'quatY', 'quatZ'].filter((role) => {
    const key = role as keyof typeof state.imuQuaternionMap
    return Boolean(state.imuQuaternionMap[key])
  }).length

  if (state.imuOrientationSource === 'rawFusion') {
    if (quaternionCount === 4) {
      return {
        level: 'good' as const,
        label: 'Rust fused quaternion active',
        details: 'Quaternion output is available and preferred over local approximation.',
        timestampMs: latestUpdateForChannels(variables, Object.values(state.imuQuaternionMap)),
      }
    }
    if (imuRawCount >= 4) {
      return {
        level: 'warning' as const,
        label: 'Local fallback active',
        details: 'Using accel/gyro approximation because fused quaternion is not mapped.',
        timestampMs: latestUpdateForChannels(variables, Object.values(state.imuChannelMap)),
      }
    }
    return {
      level: 'critical' as const,
      label: 'IMU mapping incomplete',
      details: 'Not enough accel/gyro channels are mapped to compute orientation.',
      timestampMs: null,
    }
  }

  if (state.imuOrientationSource === 'directQuaternion') {
    return {
      level: quaternionCount === 4 ? ('good' as const) : ('warning' as const),
      label: quaternionCount === 4 ? 'Quaternion mapped' : 'Quaternion incomplete',
      details:
        quaternionCount === 4
          ? 'Direct quaternion mode is fully mapped.'
          : 'Map W/X/Y/Z to use direct quaternion mode.',
      timestampMs: latestUpdateForChannels(variables, Object.values(state.imuQuaternionMap)),
    }
  }

  const attitudeCount = ['roll', 'pitch', 'yaw'].filter((role) => {
    const key = role as keyof typeof state.imuAttitudeMap
    return Boolean(state.imuAttitudeMap[key])
  }).length
  return {
    level: attitudeCount === 3 ? ('good' as const) : ('warning' as const),
    label: attitudeCount === 3 ? 'Angles mapped' : 'Angles incomplete',
    details:
      attitudeCount === 3
        ? 'Direct angle mode is fully mapped.'
        : 'Map Roll/Pitch/Yaw to use direct angle mode.',
    timestampMs: latestUpdateForChannels(variables, Object.values(state.imuAttitudeMap)),
  }
}

export const latestUpdateForChannels = (variables: Record<string, VariableEntry>, channels: Array<string | null>) =>
  channels
    .filter((channel): channel is string => Boolean(channel))
    .map((channel) => variables[channel]?.updatedAtMs ?? null)
    .reduce<number | null>((latest, timestamp) => {
      if (timestamp === null) {
        return latest
      }
      return latest === null ? timestamp : Math.max(latest, timestamp)
    }, null)
