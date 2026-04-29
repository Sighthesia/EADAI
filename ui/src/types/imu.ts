export type ImuChannelRole = 'accelX' | 'accelY' | 'accelZ' | 'gyroX' | 'gyroY' | 'gyroZ'

export type ImuAttitudeRole = 'roll' | 'pitch' | 'yaw'

export type ImuQuaternionRole = 'quatW' | 'quatX' | 'quatY' | 'quatZ'

export type ImuMapMode = 'auto' | 'manual'

export type ImuOrientationSource = 'rawFusion' | 'directAngles' | 'directQuaternion'

export interface ImuChannelMap {
  accelX: string | null
  accelY: string | null
  accelZ: string | null
  gyroX: string | null
  gyroY: string | null
  gyroZ: string | null
}

export interface ImuAttitudeMap {
  roll: string | null
  pitch: string | null
  yaw: string | null
}

export interface ImuQuaternionMap {
  quatW: string | null
  quatX: string | null
  quatY: string | null
  quatZ: string | null
}

export type ImuQualityLevel = 'good' | 'warning' | 'critical' | 'idle'

export interface ImuCalibrationState {
  accelBiasApplied: boolean
  gyroBiasApplied: boolean
  sourceLabel: string | null
  lastCalibratedAtMs: number | null
}

export interface ImuQualitySnapshot {
  level: ImuQualityLevel
  label: string
  details: string
  timestampMs: number | null
}
