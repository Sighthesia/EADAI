import { inferFusedQuaternionMapFromRawMap } from './imu'
import type { ImuAttitudeMap, ImuChannelMap, ImuChannelRole, ImuQuaternionMap, ImuQuaternionRole, VariableEntry } from '../types'

export type AxisVector = {
  unit: string | null
  timestampMs: number | null
  x: number | null
  y: number | null
  z: number | null
  magnitude: number | null
}

export type OrientationState = {
  rollDeg: number | null
  pitchDeg: number | null
  yawDeg: number | null
  timestampMs: number | null
  sourceLabel: string
  quaternion: [number, number, number, number] | null
}

export const buildVector = (
  variables: Record<string, VariableEntry>,
  map: ImuChannelMap,
  family: 'accel' | 'gyro',
): AxisVector => {
  const roles = family === 'accel' ? ['accelX', 'accelY', 'accelZ'] : ['gyroX', 'gyroY', 'gyroZ']
  const entries = roles.map((role) => resolveVariable(variables, map[role as ImuChannelRole]))
  const [x, y, z] = entries.map((entry) => entry?.numericValue ?? null)
  const timestampMs = entries.reduce<number | null>((latest, entry) => {
    const timestamp = entry?.updatedAtMs ?? null
    if (timestamp === null) {
      return latest
    }
    return latest === null ? timestamp : Math.max(latest, timestamp)
  }, null)
  const magnitude = [x, y, z].every((value) => value !== null)
    ? Math.sqrt((x ?? 0) ** 2 + (y ?? 0) ** 2 + (z ?? 0) ** 2)
    : null

  return {
    unit: entries.find((entry) => entry?.unit)?.unit ?? null,
    timestampMs,
    x,
    y,
    z,
    magnitude,
  }
}

export const buildOrientationFromRaw = (
  variables: Record<string, VariableEntry>,
  map: ImuChannelMap,
  accel: AxisVector,
): OrientationState => {
  const fusedQuaternionMap = inferFusedQuaternionMapFromRawMap(map)
  const fusedQuaternionOrientation = buildOrientationFromQuaternion(variables, fusedQuaternionMap)
  if (fusedQuaternionOrientation.quaternion) {
    return {
      ...fusedQuaternionOrientation,
      sourceLabel: 'Rust fused quaternion',
    }
  }

  const yawVariable = resolveVariable(variables, map.gyroZ)
  const yawDeg = integrateYawDeg(yawVariable)

  if (accel.x === null || accel.y === null || accel.z === null) {
    return {
      rollDeg: null,
      pitchDeg: null,
      yawDeg,
      timestampMs: yawVariable?.updatedAtMs ?? accel.timestampMs,
      sourceLabel: 'Local accel/gyro fallback',
      quaternion: null,
    }
  }

  return {
    rollDeg: radiansToDegrees(Math.atan2(accel.y, accel.z)),
    pitchDeg: radiansToDegrees(Math.atan2(-accel.x, Math.sqrt(accel.y * accel.y + accel.z * accel.z))),
    yawDeg,
    timestampMs: Math.max(accel.timestampMs ?? 0, yawVariable?.updatedAtMs ?? 0) || null,
    sourceLabel: 'Local accel/gyro fallback',
    quaternion: null,
  }
}

export const buildOrientationFromAngles = (
  variables: Record<string, VariableEntry>,
  map: ImuAttitudeMap,
): OrientationState => {
  const roll = resolveAngleDeg(resolveVariable(variables, map.roll))
  const pitch = resolveAngleDeg(resolveVariable(variables, map.pitch))
  const yaw = resolveAngleDeg(resolveVariable(variables, map.yaw))
  const timestampMs = [map.roll, map.pitch, map.yaw]
    .map((channel) => resolveVariable(variables, channel)?.updatedAtMs ?? null)
    .reduce<number | null>((latest, timestamp) => {
      if (timestamp === null) {
        return latest
      }
      return latest === null ? timestamp : Math.max(latest, timestamp)
    }, null)

  return {
    rollDeg: roll,
    pitchDeg: pitch,
    yawDeg: yaw,
    timestampMs,
    sourceLabel: 'Direct solved attitude',
    quaternion: null,
  }
}

export const buildOrientationFromQuaternion = (
  variables: Record<string, VariableEntry>,
  map: ImuQuaternionMap,
): OrientationState => {
  const quaternion = buildQuaternion(variables, map)
  if (!quaternion) {
    return {
      rollDeg: null,
      pitchDeg: null,
      yawDeg: null,
      timestampMs: latestQuaternionTimestamp(variables, map),
      sourceLabel: 'Direct quaternion attitude',
      quaternion: null,
    }
  }

  const [rollDeg, pitchDeg, yawDeg] = quaternionToEulerDegrees(quaternion)
  return {
    rollDeg,
    pitchDeg,
    yawDeg,
    timestampMs: latestQuaternionTimestamp(variables, map),
    sourceLabel: 'Direct quaternion attitude',
    quaternion,
  }
}

export const buildOrientationPreview = (orientation: OrientationState) => {
  if (!orientation.quaternion && (orientation.rollDeg === null || orientation.pitchDeg === null)) {
    return null
  }

  const vertices = [
    [-0.9, -0.45, -0.5],
    [0.9, -0.45, -0.5],
    [0.9, 0.45, -0.5],
    [-0.9, 0.45, -0.5],
    [-0.9, -0.45, 0.5],
    [0.9, -0.45, 0.5],
    [0.9, 0.45, 0.5],
    [-0.9, 0.45, 0.5],
  ] as const
  const edges = [
    [0, 1],
    [1, 2],
    [2, 3],
    [3, 0],
    [4, 5],
    [5, 6],
    [6, 7],
    [7, 4],
    [0, 4],
    [1, 5],
    [2, 6],
    [3, 7],
  ]
  const projectedVertices = vertices.map((vertex) => projectPoint(rotatePoint(vertex, orientation)))

  return {
    edges: edges.map(([fromIndex, toIndex], index) => ({
      key: `${fromIndex}-${toIndex}-${index}`,
      from: projectedVertices[fromIndex],
      to: projectedVertices[toIndex],
    })),
    axes: [
      { key: 'x', label: 'X', color: '#4FC3F7', point: projectPoint(rotatePoint([1.35, 0, 0], orientation)) },
      { key: 'y', label: 'Y', color: '#C792EA', point: projectPoint(rotatePoint([0, 1.35, 0], orientation)) },
      { key: 'z', label: 'Z', color: '#F78C6C', point: projectPoint(rotatePoint([0, 0, 1.35], orientation)) },
    ],
  }
}

const resolveVariable = (variables: Record<string, VariableEntry>, channel: string | null | undefined) =>
  (channel ? variables[channel] : undefined)

const buildQuaternion = (variables: Record<string, VariableEntry>, map: ImuQuaternionMap) => {
  const components = ['quatW', 'quatX', 'quatY', 'quatZ'].map((role) => resolveVariable(variables, map[role as ImuQuaternionRole]))
  const values = components.map((entry) => entry?.numericValue ?? null)
  if (values.some((value) => value === null)) {
    return null
  }

  const quaternion = [values[0] ?? 1, values[1] ?? 0, values[2] ?? 0, values[3] ?? 0] as [number, number, number, number]
  return normalizeQuaternion(quaternion)
}

const latestQuaternionTimestamp = (variables: Record<string, VariableEntry>, map: ImuQuaternionMap) =>
  ['quatW', 'quatX', 'quatY', 'quatZ']
    .map((role) => resolveVariable(variables, map[role as ImuQuaternionRole])?.updatedAtMs ?? null)
    .reduce<number | null>((latest, timestamp) => {
      if (timestamp === null) {
        return latest
      }
      return latest === null ? timestamp : Math.max(latest, timestamp)
    }, null)

const resolveAngleDeg = (variable: VariableEntry | undefined) => {
  if (!variable || variable.numericValue === undefined || variable.numericValue === null) {
    return null
  }

  const unit = variable.unit?.toLowerCase() ?? ''
  const angleDeg = unit.includes('rad') ? radiansToDegrees(variable.numericValue) : variable.numericValue
  return normalizeAngle(angleDeg)
}

const integrateYawDeg = (variable: VariableEntry | undefined) => {
  if (!variable || variable.points.length < 2) {
    return null
  }

  const points = variable.points.slice(-180)
  let yawDeg = 0
  const scale = variable.unit?.toLowerCase().includes('rad') ? 180 / Math.PI : 1

  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1]
    const current = points[index]
    const deltaSeconds = (current.timestampMs - previous.timestampMs) / 1000
    if (deltaSeconds <= 0 || deltaSeconds > 1) {
      continue
    }

    yawDeg += ((previous.value + current.value) / 2) * deltaSeconds * scale
  }

  return normalizeAngle(yawDeg)
}

const rotatePoint = (point: readonly [number, number, number], orientation: OrientationState) => {
  if (orientation.quaternion) {
    return rotatePointByQuaternion(point, orientation.quaternion)
  }

  const roll = degreesToRadians(orientation.rollDeg ?? 0)
  const pitch = degreesToRadians(orientation.pitchDeg ?? 0)
  const yaw = degreesToRadians(orientation.yawDeg ?? 0)
  const [x, y, z] = point

  const sinRoll = Math.sin(roll)
  const cosRoll = Math.cos(roll)
  const sinPitch = Math.sin(pitch)
  const cosPitch = Math.cos(pitch)
  const sinYaw = Math.sin(yaw)
  const cosYaw = Math.cos(yaw)

  return {
    x: x * cosYaw * cosPitch + y * (cosYaw * sinPitch * sinRoll - sinYaw * cosRoll) + z * (cosYaw * sinPitch * cosRoll + sinYaw * sinRoll),
    y: x * sinYaw * cosPitch + y * (sinYaw * sinPitch * sinRoll + cosYaw * cosRoll) + z * (sinYaw * sinPitch * cosRoll - cosYaw * sinRoll),
    z: -x * sinPitch + y * cosPitch * sinRoll + z * cosPitch * cosRoll,
  }
}

const rotatePointByQuaternion = (point: readonly [number, number, number], quaternion: [number, number, number, number]) => {
  const [w, x, y, z] = quaternion
  const qx = x
  const qy = y
  const qz = z
  const qw = w
  const [px, py, pz] = point

  const ix = qw * px + qy * pz - qz * py
  const iy = qw * py + qz * px - qx * pz
  const iz = qw * pz + qx * py - qy * px
  const iw = -qx * px - qy * py - qz * pz

  return {
    x: ix * qw + iw * -qx + iy * -qz - iz * -qy,
    y: iy * qw + iw * -qy + iz * -qx - ix * -qz,
    z: iz * qw + iw * -qz + ix * -qy - iy * -qx,
  }
}

const projectPoint = (point: { x: number; y: number; z: number }) => {
  const perspective = 3.4 / (3.4 - point.z)
  return {
    x: 140 + point.x * 56 * perspective,
    y: 110 - point.y * 56 * perspective,
  }
}

const normalizeAngle = (value: number) => {
  let normalized = value % 360
  if (normalized > 180) {
    normalized -= 360
  }
  if (normalized < -180) {
    normalized += 360
  }
  return normalized
}

const quaternionToEulerDegrees = ([w, x, y, z]: [number, number, number, number]) => {
  const roll = Math.atan2(2 * (w * x + y * z), 1 - 2 * (x * x + y * y))
  const pitch = Math.asin(clamp(2 * (w * y - z * x), -1, 1))
  const yaw = Math.atan2(2 * (w * z + x * y), 1 - 2 * (y * y + z * z))
  return [normalizeAngle(radiansToDegrees(roll)), normalizeAngle(radiansToDegrees(pitch)), normalizeAngle(radiansToDegrees(yaw))] as const
}

const normalizeQuaternion = ([w, x, y, z]: [number, number, number, number]) => {
  const norm = Math.hypot(w, x, y, z)
  if (norm <= Number.EPSILON) {
    return [1, 0, 0, 0] as [number, number, number, number]
  }
  return [w / norm, x / norm, y / norm, z / norm] as [number, number, number, number]
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

const radiansToDegrees = (value: number) => (value * 180) / Math.PI

const degreesToRadians = (value: number) => (value * Math.PI) / 180
