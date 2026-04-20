import { inferFusedQuaternionMapFromRawMap } from './imu'
import type {
  ImuAttitudeMap,
  ImuChannelMap,
  ImuOrientationSource,
  ImuQuaternionMap,
  ImuQuaternionRole,
  ImuChannelRole,
  VariableEntry,
} from '../types'

const MAX_TRAJECTORY_SOURCE_POINTS = 180
const MAX_TRAJECTORY_AGE_MS = 12000
const MAX_TRAJECTORY_POINTS = 220

export type ImuWorldPoint = {
  x: number
  y: number
  z: number
}

export type ImuMotionTrajectoryPoint = {
  position: ImuWorldPoint
  timestampMs: number
  dominantAxis: 'accelX' | 'accelY' | 'accelZ'
  intensity: number
}

export type ImuMotionTrajectory = {
  points: ImuMotionTrajectoryPoint[]
  latestTimestampMs: number | null
  sampleCount: number
}

type MotionFrame = {
  position: ImuWorldPoint
  normalized: ImuWorldPoint
  raw: ImuWorldPoint
  timestampMs: number
  intensity: number
  dominantAxis: 'accelX' | 'accelY' | 'accelZ'
}

export type ImuCoordinateLine = {
  key: string
  from: ImuWorldPoint
  to: ImuWorldPoint
  opacity: number
  color: string
  kind: 'major' | 'minor'
}

export type ImuCoordinateScene = {
  axes: Array<ImuCoordinateLine & { label: string; color: string }>
  grid: ImuCoordinateLine[]
}

export type ImuOrientationPreview = {
  position: ImuWorldPoint
  quaternion: [number, number, number, number] | null
}

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
  return {
    position: { x: 0, y: 0, z: 0 },
    quaternion: orientation.quaternion,
  }
}

export const buildImuCoordinateScene = (): ImuCoordinateScene => {
  const origin = { x: 0, y: 0, z: 0 }
  const axisLength = 1.65
  const gridSteps = [-1.5, -1.0, -0.5, 0, 0.5, 1.0, 1.5]
  const gridPlanes = [
    { key: 'xy', axis: 'z' as const, value: -0.92, opacityScale: 1, majorColor: '#6D96B7', minorColor: '#445B6D' },
    { key: 'xz', axis: 'y' as const, value: -0.72, opacityScale: 0.66, majorColor: '#6FAF9E', minorColor: '#44645D' },
    { key: 'yz', axis: 'x' as const, value: -0.72, opacityScale: 0.56, majorColor: '#A88BC7', minorColor: '#675377' },
  ]

  const axes: ImuCoordinateScene['axes'] = [
    {
      key: 'x',
      label: 'X',
      color: '#4FC3F7',
      from: origin,
      to: { x: axisLength, y: 0, z: 0 },
      opacity: 0.92,
      kind: 'major',
    },
    {
      key: 'y',
      label: 'Y',
      color: '#C792EA',
      from: origin,
      to: { x: 0, y: axisLength, z: 0 },
      opacity: 0.92,
      kind: 'major',
    },
    {
      key: 'z',
      label: 'Z',
      color: '#F78C6C',
      from: origin,
      to: { x: 0, y: 0, z: axisLength },
      opacity: 0.92,
      kind: 'major',
    },
  ]

  const grid: ImuCoordinateLine[] = []
  for (const plane of gridPlanes) {
    const planeScale = plane.opacityScale

    for (const step of gridSteps) {
      const isMajor = step === 0
      const emphasis = step === 0 ? 1 : Math.max(0.24, 1 - Math.abs(step) / 2.1)
      const opacity = (isMajor ? 0.24 : 0.085) * planeScale * emphasis
      const color = isMajor ? plane.majorColor : plane.minorColor

      if (plane.axis === 'z') {
        grid.push({
          key: `grid-xy-x-${step}`,
          from: { x: gridSteps[0]!, y: step, z: plane.value },
          to: { x: gridSteps[gridSteps.length - 1]!, y: step, z: plane.value },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
        grid.push({
          key: `grid-xy-y-${step}`,
          from: { x: step, y: gridSteps[0]!, z: plane.value },
          to: { x: step, y: gridSteps[gridSteps.length - 1]!, z: plane.value },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
      }

      if (plane.axis === 'y') {
        grid.push({
          key: `grid-xz-x-${step}`,
          from: { x: gridSteps[0]!, y: plane.value, z: step },
          to: { x: gridSteps[gridSteps.length - 1]!, y: plane.value, z: step },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
        grid.push({
          key: `grid-xz-z-${step}`,
          from: { x: step, y: plane.value, z: gridSteps[0]! },
          to: { x: step, y: plane.value, z: gridSteps[gridSteps.length - 1]! },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
      }

      if (plane.axis === 'x') {
        grid.push({
          key: `grid-yz-y-${step}`,
          from: { x: plane.value, y: gridSteps[0]!, z: step },
          to: { x: plane.value, y: gridSteps[gridSteps.length - 1]!, z: step },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
        grid.push({
          key: `grid-yz-z-${step}`,
          from: { x: plane.value, y: step, z: gridSteps[0]! },
          to: { x: plane.value, y: step, z: gridSteps[gridSteps.length - 1]! },
          opacity,
          color,
          kind: isMajor ? 'major' : 'minor',
        })
      }
    }
  }

  return { axes, grid }
}

export const buildMotionTrajectory = (
  variables: Record<string, VariableEntry>,
  map: ImuChannelMap,
  orientationSource: ImuOrientationSource,
  attitudeMap: ImuAttitudeMap,
  quaternionMap: ImuQuaternionMap,
): ImuMotionTrajectory | null => {
  const frames = buildMotionFrames(variables, map, orientationSource, attitudeMap, quaternionMap)
  if (frames.length < 2) {
    return null
  }

  const latestTimestampMs = frames[frames.length - 1]?.timestampMs ?? null
  const cutoffTimestampMs = latestTimestampMs === null ? null : latestTimestampMs - MAX_TRAJECTORY_AGE_MS
  const visibleFrames = cutoffTimestampMs === null ? frames : frames.filter((frame) => frame.timestampMs >= cutoffTimestampMs)
  const boundedFrames = visibleFrames.slice(-MAX_TRAJECTORY_POINTS)

  return {
    points: boundedFrames.map((frame) => ({
      position: frame.position,
      timestampMs: frame.timestampMs,
      dominantAxis: frame.dominantAxis,
      intensity: frame.intensity,
    })),
    latestTimestampMs,
    sampleCount: boundedFrames.length,
  }
}

const resolveVariable = (variables: Record<string, VariableEntry>, channel: string | null | undefined) =>
  (channel ? variables[channel] : undefined)

const buildMotionFrames = (
  variables: Record<string, VariableEntry>,
  map: ImuChannelMap,
  orientationSource: ImuOrientationSource,
  attitudeMap: ImuAttitudeMap,
  quaternionMap: ImuQuaternionMap,
): MotionFrame[] => {
  const accelX = resolveVariable(variables, map.accelX)?.points.slice(-MAX_TRAJECTORY_SOURCE_POINTS) ?? []
  const accelY = resolveVariable(variables, map.accelY)?.points.slice(-MAX_TRAJECTORY_SOURCE_POINTS) ?? []
  const accelZ = resolveVariable(variables, map.accelZ)?.points.slice(-MAX_TRAJECTORY_SOURCE_POINTS) ?? []

  if (accelX.length === 0 || accelY.length === 0 || accelZ.length === 0) {
    return []
  }

  const timeline = Array.from(
    new Set([...accelX, ...accelY, ...accelZ].map((point) => point.timestampMs)),
  ).sort((left, right) => left - right)

  const maxAbs = [...accelX, ...accelY, ...accelZ].reduce((current, point) => Math.max(current, Math.abs(point.value)), 0)
  const normalizedScale = maxAbs > Number.EPSILON ? maxAbs : 1

  const frames: MotionFrame[] = []
  let xIndex = 0
  let yIndex = 0
  let zIndex = 0
  let latestX: number | null = null
  let latestY: number | null = null
  let latestZ: number | null = null
  let velocity = { x: 0, y: 0, z: 0 }
  let position = { x: 0, y: 0, z: 0 }
  let worldBias = { x: 0, y: 0, z: 0 }
  let biasReady = false
  let previousTimestampMs: number | null = null

  for (const timestampMs of timeline) {
    while (xIndex < accelX.length && accelX[xIndex]!.timestampMs <= timestampMs) {
      latestX = accelX[xIndex]!.value
      xIndex += 1
    }
    while (yIndex < accelY.length && accelY[yIndex]!.timestampMs <= timestampMs) {
      latestY = accelY[yIndex]!.value
      yIndex += 1
    }
    while (zIndex < accelZ.length && accelZ[zIndex]!.timestampMs <= timestampMs) {
      latestZ = accelZ[zIndex]!.value
      zIndex += 1
    }

    if (latestX === null || latestY === null || latestZ === null) {
      continue
    }

    const normalized = {
      x: compressMotionValue(latestX / normalizedScale),
      y: compressMotionValue(latestY / normalizedScale),
      z: compressMotionValue(latestZ / normalizedScale),
    }
    const worldAcceleration = rotateAccelerationToWorld(
      { x: latestX, y: latestY, z: latestZ },
      variables,
      timestampMs,
      map,
      orientationSource,
      attitudeMap,
      quaternionMap,
    )
    const deltaSeconds = previousTimestampMs === null ? 1 / 60 : clampValue((timestampMs - previousTimestampMs) / 1000, 1 / 240, 0.08)
    previousTimestampMs = timestampMs
    if (!biasReady) {
      worldBias = { ...worldAcceleration }
      biasReady = true
    } else {
      const residualMagnitude = vectorMagnitude({
        x: worldAcceleration.x - worldBias.x,
        y: worldAcceleration.y - worldBias.y,
        z: worldAcceleration.z - worldBias.z,
      })
      const biasAlpha = residualMagnitude < 2.2 ? clampValue(deltaSeconds * 0.9, 0.01, 0.05) : clampValue(deltaSeconds * 0.08, 0.001, 0.004)
      worldBias = {
        x: worldBias.x + (worldAcceleration.x - worldBias.x) * biasAlpha,
        y: worldBias.y + (worldAcceleration.y - worldBias.y) * biasAlpha,
        z: worldBias.z + (worldAcceleration.z - worldBias.z) * biasAlpha,
      }
    }

    const compensatedAcceleration = {
      x: applyAccelerationDeadband(worldAcceleration.x - worldBias.x),
      y: applyAccelerationDeadband(worldAcceleration.y - worldBias.y),
      z: applyAccelerationDeadband(worldAcceleration.z - worldBias.z),
    }

    velocity = {
      x: velocity.x * 0.48 - compensatedAcceleration.x * 5.2 * deltaSeconds,
      y: velocity.y * 0.48 - compensatedAcceleration.y * 5.2 * deltaSeconds,
      z: velocity.z * 0.48 - compensatedAcceleration.z * 5.2 * deltaSeconds,
    }
    position = {
      x: clampValue(position.x + velocity.x * 2.0 * deltaSeconds, -2.6, 2.6),
      y: clampValue(position.y + velocity.y * 2.0 * deltaSeconds, -2.6, 2.6),
      z: clampValue(position.z + velocity.z * 2.0 * deltaSeconds, -2.6, 2.6),
    }

    frames.push({
      position,
      normalized,
      raw: { x: latestX, y: latestY, z: latestZ },
      timestampMs,
      intensity: Math.max(Math.abs(latestX), Math.abs(latestY), Math.abs(latestZ)),
      dominantAxis: resolveDominantAxis(latestX, latestY, latestZ),
    })
  }

  return frames
}

const resolveDominantAxis = (x: number, y: number, z: number): 'accelX' | 'accelY' | 'accelZ' => {
  const magnitudes: Array<{ axis: 'accelX' | 'accelY' | 'accelZ'; magnitude: number }> = [
    { axis: 'accelX', magnitude: Math.abs(x) },
    { axis: 'accelY', magnitude: Math.abs(y) },
    { axis: 'accelZ', magnitude: Math.abs(z) },
  ]
  return magnitudes.reduce((best, current) => (current.magnitude > best.magnitude ? current : best)).axis
}

const rotateAccelerationToWorld = (
  acceleration: ImuWorldPoint,
  variables: Record<string, VariableEntry>,
  timestampMs: number,
  map: ImuChannelMap,
  orientationSource: ImuOrientationSource,
  attitudeMap: ImuAttitudeMap,
  quaternionMap: ImuQuaternionMap,
) => {
  const quaternion = resolveOrientationQuaternionAtTimestamp(variables, timestampMs, map, orientationSource, attitudeMap, quaternionMap)
  if (!quaternion) {
    return acceleration
  }

  return rotatePointByQuaternion([acceleration.x, acceleration.y, acceleration.z], quaternion)
}

const resolveOrientationQuaternionAtTimestamp = (
  variables: Record<string, VariableEntry>,
  timestampMs: number,
  map: ImuChannelMap,
  orientationSource: ImuOrientationSource,
  attitudeMap: ImuAttitudeMap,
  quaternionMap: ImuQuaternionMap,
) => {
  if (orientationSource === 'directQuaternion') {
    const quaternion = resolveQuaternionAtTimestamp(variables, quaternionMap, timestampMs)
    return quaternion ? normalizeQuaternion(quaternion) : null
  }

  if (orientationSource === 'directAngles') {
    const angles = resolveAnglesAtTimestamp(variables, attitudeMap, timestampMs)
    return angles ? anglesToQuaternion(angles.rollDeg, angles.pitchDeg, angles.yawDeg) : null
  }

  const accel = resolveAccelAtTimestamp(variables, map, timestampMs)
  if (!accel) {
    return null
  }

  const gyroZ = resolveVariableAtTimestamp(variables, map.gyroZ, timestampMs)
  const rollDeg = radiansToDegrees(Math.atan2(accel.y, accel.z))
  const pitchDeg = radiansToDegrees(Math.atan2(-accel.x, Math.sqrt(accel.y * accel.y + accel.z * accel.z)))
  const yawDeg = integrateYawDegAtTimestamp(variables, map.gyroZ, timestampMs, gyroZ)
  return anglesToQuaternion(rollDeg, pitchDeg, yawDeg)
}

const resolveQuaternionAtTimestamp = (
  variables: Record<string, VariableEntry>,
  map: ImuQuaternionMap,
  timestampMs: number,
) => {
  const w = resolveVariableAtTimestamp(variables, map.quatW, timestampMs)
  const x = resolveVariableAtTimestamp(variables, map.quatX, timestampMs)
  const y = resolveVariableAtTimestamp(variables, map.quatY, timestampMs)
  const z = resolveVariableAtTimestamp(variables, map.quatZ, timestampMs)
  if (w === null || x === null || y === null || z === null) {
    return null
  }

  return [w, x, y, z] as [number, number, number, number]
}

const resolveAnglesAtTimestamp = (variables: Record<string, VariableEntry>, map: ImuAttitudeMap, timestampMs: number) => {
  const roll = resolveAngleValueAtTimestamp(variables, map.roll, timestampMs)
  const pitch = resolveAngleValueAtTimestamp(variables, map.pitch, timestampMs)
  const yaw = resolveAngleValueAtTimestamp(variables, map.yaw, timestampMs)
  if (roll === null || pitch === null || yaw === null) {
    return null
  }

  return { rollDeg: roll, pitchDeg: pitch, yawDeg: yaw }
}

const resolveAccelAtTimestamp = (variables: Record<string, VariableEntry>, map: ImuChannelMap, timestampMs: number) => {
  const x = resolveVariableAtTimestamp(variables, map.accelX, timestampMs)
  const y = resolveVariableAtTimestamp(variables, map.accelY, timestampMs)
  const z = resolveVariableAtTimestamp(variables, map.accelZ, timestampMs)
  if (x === null || y === null || z === null) {
    return null
  }

  return { x, y, z }
}

const resolveAngleValueAtTimestamp = (variables: Record<string, VariableEntry>, channel: string | null | undefined, timestampMs: number) => {
  const value = resolveVariableAtTimestamp(variables, channel, timestampMs)
  if (value === null || !channel) {
    return null
  }

  const unit = variables[channel]?.unit?.toLowerCase() ?? ''
  return unit.includes('rad') ? radiansToDegrees(value) : value
}

const resolveVariableAtTimestamp = (variables: Record<string, VariableEntry>, channel: string | null | undefined, timestampMs: number) => {
  const points = channel ? variables[channel]?.points ?? [] : []
  let latestValue: number | null = null

  for (const point of points) {
    if (point.timestampMs > timestampMs) {
      break
    }
    latestValue = point.value
  }

  return latestValue
}

const integrateYawDegAtTimestamp = (
  variables: Record<string, VariableEntry>,
  channel: string | null | undefined,
  timestampMs: number,
  fallbackValue: number | null,
) => {
  const points = channel ? variables[channel]?.points ?? [] : []
  if (points.length === 0) {
    return fallbackValue ?? 0
  }

  const unitScale = channel && variables[channel]?.unit?.toLowerCase().includes('rad') ? 180 / Math.PI : 1
  let yawDeg = 0
  let previous = points[0]

  for (let index = 1; index < points.length; index += 1) {
    const current = points[index]!
    if (current.timestampMs > timestampMs) {
      break
    }

    const deltaSeconds = (current.timestampMs - previous.timestampMs) / 1000
    if (deltaSeconds > 0 && deltaSeconds <= 1) {
      yawDeg += ((previous.value + current.value) / 2) * deltaSeconds * unitScale
    }
    previous = current
  }

  return normalizeAngle(yawDeg)
}

const anglesToQuaternion = (rollDeg: number, pitchDeg: number, yawDeg: number) => {
  const roll = degreesToRadians(rollDeg)
  const pitch = degreesToRadians(pitchDeg)
  const yaw = degreesToRadians(yawDeg)

  const cy = Math.cos(yaw * 0.5)
  const sy = Math.sin(yaw * 0.5)
  const cp = Math.cos(pitch * 0.5)
  const sp = Math.sin(pitch * 0.5)
  const cr = Math.cos(roll * 0.5)
  const sr = Math.sin(roll * 0.5)

  return normalizeQuaternion([
    cr * cp * cy + sr * sp * sy,
    sr * cp * cy - cr * sp * sy,
    cr * sp * cy + sr * cp * sy,
    cr * cp * sy - sr * sp * cy,
  ])
}

const compressMotionValue = (value: number) => {
  if (!Number.isFinite(value)) {
    return 0
  }
  return Math.tanh(value)
}

const clampValue = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

const applyAccelerationDeadband = (value: number) => {
  if (Math.abs(value) < 3.5) {
    return 0
  }

  return value
}

const vectorMagnitude = (point: ImuWorldPoint) => Math.sqrt(point.x * point.x + point.y * point.y + point.z * point.z)

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
