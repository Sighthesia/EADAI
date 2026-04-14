import type { CSSProperties } from 'react'
import { useMemo } from 'react'
import {
  inferFusedQuaternionMapFromRawMap,
  IMU_ATTITUDE_LABELS,
  IMU_ATTITUDE_ROLES,
  IMU_CHANNEL_ROLES,
  IMU_QUATERNION_LABELS,
  IMU_QUATERNION_ROLES,
  IMU_ROLE_LABELS,
  countMappedImuAttitudeChannels,
  countMappedImuChannels,
  countMappedImuQuaternionChannels,
} from '../lib/imu'
import {
  AxisVector,
  buildOrientationFromAngles,
  buildOrientationFromQuaternion,
  buildOrientationFromRaw,
  buildOrientationPreview,
  buildVector,
} from '../lib/imu-view'
import { SourceChoiceGroup } from './SourceChoiceGroup'
import { useAppStore } from '../store/appStore'
import type {
  ImuAttitudeMap,
  ImuAttitudeRole,
  ImuChannelMap,
  ImuChannelRole,
  ImuOrientationSource,
  ImuQuaternionMap,
  ImuQuaternionRole,
} from '../types'

const SOURCE_OPTIONS: Array<{ value: ImuOrientationSource; label: string; description: string }> = [
  {
    value: 'rawFusion',
    label: 'Raw Fusion',
    description: 'Use mapped accel/gyro variables and derive attitude locally.',
  },
  {
    value: 'directAngles',
    label: 'Solved Angles',
    description: 'Use already solved roll/pitch/yaw variables directly.',
  },
  {
    value: 'directQuaternion',
    label: 'Quaternion',
    description: 'Use solved quaternion channels directly for stable 3D orientation.',
  },
]

export function ImuPanel() {
  const variables = useAppStore((state) => state.variables)
  const imuChannelMap = useAppStore((state) => state.imuChannelMap)
  const imuAttitudeMap = useAppStore((state) => state.imuAttitudeMap)
  const imuQuaternionMap = useAppStore((state) => state.imuQuaternionMap)
  const imuMapMode = useAppStore((state) => state.imuMapMode)
  const imuOrientationSource = useAppStore((state) => state.imuOrientationSource)
  const imuCalibration = useAppStore((state) => state.imuCalibration)
  const imuQuality = useAppStore((state) => state.imuQuality)
  const setImuChannel = useAppStore((state) => state.setImuChannel)
  const setImuAttitudeChannel = useAppStore((state) => state.setImuAttitudeChannel)
  const setImuQuaternionChannel = useAppStore((state) => state.setImuQuaternionChannel)
  const setImuOrientationSource = useAppStore((state) => state.setImuOrientationSource)
  const calibrateImuFromCurrentState = useAppStore((state) => state.calibrateImuFromCurrentState)
  const resetImuCalibration = useAppStore((state) => state.resetImuCalibration)
  const autoMapImuChannels = useAppStore((state) => state.autoMapImuChannels)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const channelOptions = useMemo(() => Object.keys(variables).sort(), [variables])
  const accel = useMemo(() => buildVector(variables, imuChannelMap, 'accel'), [imuChannelMap, variables])
  const gyro = useMemo(() => buildVector(variables, imuChannelMap, 'gyro'), [imuChannelMap, variables])
  const rawFusionQuaternionMap = useMemo(() => inferFusedQuaternionMapFromRawMap(imuChannelMap), [imuChannelMap])
  const orientation = useMemo(
    () =>
      imuOrientationSource === 'directAngles'
        ? buildOrientationFromAngles(variables, imuAttitudeMap)
        : imuOrientationSource === 'directQuaternion'
          ? buildOrientationFromQuaternion(variables, imuQuaternionMap)
        : buildOrientationFromRaw(variables, imuChannelMap, accel),
    [accel, imuAttitudeMap, imuChannelMap, imuOrientationSource, imuQuaternionMap, variables],
  )
  const preview = useMemo(() => buildOrientationPreview(orientation), [orientation])
  const mappedCount =
    imuOrientationSource === 'directAngles'
      ? countMappedImuAttitudeChannels(imuAttitudeMap)
      : imuOrientationSource === 'directQuaternion'
        ? countMappedImuQuaternionChannels(imuQuaternionMap)
        : countMappedImuChannels(imuChannelMap)

  return (
    <section className="panel imu-panel panel-scroll">
      <div className="toolbar-row imu-toolbar">
        <div className="toolbar-title-group">
          <strong>IMU</strong>
          <small>{sourceDescription(imuOrientationSource)}</small>
        </div>
        <div className="imu-toolbar-actions">
          <span className="imu-chip">{mappedCount}/{requiredMappingCount(imuOrientationSource)} mapped</span>
          <span className="imu-chip">{imuMapMode}</span>
          <button className="ghost-button" onClick={() => autoMapImuChannels()}>
            Auto Detect
          </button>
        </div>
      </div>

      <SourceChoiceGroup
        ariaLabel="IMU orientation source"
        className="imu-mode-switch"
        value={imuOrientationSource}
        options={SOURCE_OPTIONS}
        onChange={(value) => setImuOrientationSource(value)}
      />

      <div className="imu-status-grid">
        <article className={`imu-card imu-status-card tone-${imuQuality.level}`}>
          <div className="imu-card-header">
            <strong>IMU Quality</strong>
            <small>{formatTimestamp(imuQuality.timestampMs)}</small>
          </div>
          <div className="imu-status-main">
            <span className={`status-pill tone-${imuQuality.level}`}>{imuQuality.label}</span>
            <p>{imuQuality.details}</p>
          </div>
        </article>

        <article className="imu-card imu-status-card">
          <div className="imu-card-header">
            <strong>Calibration</strong>
            <small>{imuCalibration.lastCalibratedAtMs ? formatTimestamp(imuCalibration.lastCalibratedAtMs) : 'Not stored'}</small>
          </div>
          <div className="imu-status-main">
            <div className="imu-calibration-chips">
              <span className={`imu-chip ${imuCalibration.accelBiasApplied ? 'active' : ''}`}>Accel bias {imuCalibration.accelBiasApplied ? 'set' : 'unset'}</span>
              <span className={`imu-chip ${imuCalibration.gyroBiasApplied ? 'active' : ''}`}>Gyro bias {imuCalibration.gyroBiasApplied ? 'set' : 'unset'}</span>
              <span className="imu-chip">Source {imuCalibration.sourceLabel ?? 'none'}</span>
            </div>
            <div className="toolbar-row imu-calibration-actions">
              <button className="ghost-button" onClick={() => calibrateImuFromCurrentState()}>
                Capture Calibration
              </button>
              <button className="ghost-button" onClick={() => resetImuCalibration()}>
                Reset
              </button>
            </div>
          </div>
        </article>
      </div>

      {imuOrientationSource === 'directAngles' ? (
        <AttitudeSourceGrid channelOptions={channelOptions} map={imuAttitudeMap} onChange={setImuAttitudeChannel} />
      ) : imuOrientationSource === 'directQuaternion' ? (
        <QuaternionSourceGrid channelOptions={channelOptions} map={imuQuaternionMap} onChange={setImuQuaternionChannel} />
      ) : (
        <RawSourceGrid channelOptions={channelOptions} map={imuChannelMap} onChange={setImuChannel} />
      )}

      <div className="imu-layout-grid">
        <article className="imu-card imu-stage-card">
          <div className="imu-card-header">
            <strong>Orientation Preview</strong>
            <small>{orientation.sourceLabel}</small>
          </div>
          {preview ? (
            <svg viewBox="0 0 280 220" className="imu-stage" aria-label="IMU orientation preview">
              <circle cx="140" cy="110" r="78" className="imu-stage-ring" />
              <line x1="140" y1="24" x2="140" y2="196" className="imu-stage-grid" />
              <line x1="54" y1="110" x2="226" y2="110" className="imu-stage-grid" />
              {preview.edges.map((edge) => (
                <line
                  key={edge.key}
                  x1={edge.from.x}
                  y1={edge.from.y}
                  x2={edge.to.x}
                  y2={edge.to.y}
                  className="imu-stage-cube"
                />
              ))}
              {preview.axes.map((axis) => (
                <g key={axis.key}>
                  <line x1={140} y1={110} x2={axis.point.x} y2={axis.point.y} stroke={axis.color} strokeWidth="3" />
                  <circle cx={axis.point.x} cy={axis.point.y} r="4" fill={axis.color} />
                  <text x={axis.point.x + 8} y={axis.point.y + 4} fill={axis.color} className="imu-axis-label">
                    {axis.label}
                  </text>
                </g>
              ))}
            </svg>
          ) : (
            <div className="imu-empty-state">{emptyStateMessage(imuOrientationSource)}</div>
          )}

          <div className="imu-attitude-grid">
            <MetricTile label="Roll" value={formatAngle(orientation.rollDeg)} accent="#4FC3F7" />
            <MetricTile label="Pitch" value={formatAngle(orientation.pitchDeg)} accent="#C792EA" />
            <MetricTile label="Yaw" value={formatAngle(orientation.yawDeg)} accent="#F78C6C" secondary={formatTimestamp(orientation.timestampMs)} />
          </div>
        </article>

        <article className="imu-card">
          <div className="imu-card-header">
            <strong>Acceleration</strong>
            <small>{accel.unit ?? 'raw'}</small>
          </div>
          <VectorSummary vector={accel} axisPrefix="a" colorForChannel={colorForChannel} map={imuChannelMap} />
        </article>

        <article className="imu-card">
          <div className="imu-card-header">
            <strong>Gyroscope</strong>
            <small>{gyro.unit ?? 'raw'}</small>
          </div>
          <VectorSummary vector={gyro} axisPrefix="g" colorForChannel={colorForChannel} map={imuChannelMap} />
        </article>

        <article className="imu-card">
          <div className="imu-card-header">
            <strong>Quaternion</strong>
            <small>{imuOrientationSource === 'directQuaternion' || imuOrientationSource === 'rawFusion' ? 'Active source' : 'Reference'}</small>
          </div>
          <QuaternionSummary
            colorForChannel={colorForChannel}
            map={imuOrientationSource === 'rawFusion' ? rawFusionQuaternionMap : imuQuaternionMap}
            variables={variables}
          />
        </article>
      </div>
    </section>
  )
}

function RawSourceGrid({
  channelOptions,
  map,
  onChange,
}: {
  channelOptions: string[]
  map: ImuChannelMap
  onChange: (role: ImuChannelRole, channel: string | null) => void
}) {
  return (
    <div className="imu-source-grid">
      {IMU_CHANNEL_ROLES.map((role) => (
        <label key={role} className="imu-source-field">
          <span>{IMU_ROLE_LABELS[role]}</span>
          <select value={map[role] ?? ''} onChange={(event) => onChange(role, event.target.value || null)}>
            <option value="">Unmapped</option>
            {channelOptions.map((channel) => (
              <option key={channel} value={channel}>
                {channel}
              </option>
            ))}
          </select>
        </label>
      ))}
    </div>
  )
}

function AttitudeSourceGrid({
  channelOptions,
  map,
  onChange,
}: {
  channelOptions: string[]
  map: ImuAttitudeMap
  onChange: (role: ImuAttitudeRole, channel: string | null) => void
}) {
  return (
    <div className="imu-source-grid imu-source-grid-compact">
      {IMU_ATTITUDE_ROLES.map((role) => (
        <label key={role} className="imu-source-field">
          <span>{IMU_ATTITUDE_LABELS[role]}</span>
          <select value={map[role] ?? ''} onChange={(event) => onChange(role, event.target.value || null)}>
            <option value="">Unmapped</option>
            {channelOptions.map((channel) => (
              <option key={channel} value={channel}>
                {channel}
              </option>
            ))}
          </select>
        </label>
      ))}
    </div>
  )
}

function QuaternionSourceGrid({
  channelOptions,
  map,
  onChange,
}: {
  channelOptions: string[]
  map: ImuQuaternionMap
  onChange: (role: ImuQuaternionRole, channel: string | null) => void
}) {
  return (
    <div className="imu-source-grid">
      {IMU_QUATERNION_ROLES.map((role) => (
        <label key={role} className="imu-source-field">
          <span>{IMU_QUATERNION_LABELS[role]}</span>
          <select value={map[role] ?? ''} onChange={(event) => onChange(role, event.target.value || null)}>
            <option value="">Unmapped</option>
            {channelOptions.map((channel) => (
              <option key={channel} value={channel}>
                {channel}
              </option>
            ))}
          </select>
        </label>
      ))}
    </div>
  )
}

function VectorSummary({
  vector,
  axisPrefix,
  colorForChannel,
  map,
}: {
  vector: AxisVector
  axisPrefix: 'a' | 'g'
  colorForChannel: (channel: string) => string
  map: ImuChannelMap
}) {
  const roles = axisPrefix === 'a' ? (['accelX', 'accelY', 'accelZ'] as const) : (['gyroX', 'gyroY', 'gyroZ'] as const)
  const values = [vector.x, vector.y, vector.z]

  return (
    <>
      <div className="imu-vector-grid">
        {roles.map((role, index) => {
          const channel = map[role]
          return (
            <MetricTile
              key={role}
              label={`${axisPrefix}${role.slice(-1).toLowerCase()}`}
              value={formatVectorValue(values[index])}
              accent={channel ? colorForChannel(channel) : '#5f6b7a'}
              secondary={channel ?? 'Unmapped'}
            />
          )
        })}
      </div>
      <div className="imu-magnitude-bar">
        <span>Magnitude</span>
        <div className="imu-magnitude-track">
          <div className="imu-magnitude-fill" style={{ width: `${scaleMagnitude(vector.magnitude, axisPrefix)}%` }} />
        </div>
        <strong>{formatVectorValue(vector.magnitude)}</strong>
      </div>
    </>
  )
}

function QuaternionSummary({
  colorForChannel,
  map,
  variables,
}: {
  colorForChannel: (channel: string) => string
  map: ImuQuaternionMap
  variables: Record<string, { numericValue?: number; updatedAtMs: number }>
}) {
  return (
    <div className="imu-vector-grid imu-quaternion-grid">
      {IMU_QUATERNION_ROLES.map((role) => {
        const channel = map[role]
        const value = channel ? variables[channel]?.numericValue ?? null : null
        return (
          <MetricTile
            key={role}
            label={role.replace('quat', 'q').toLowerCase()}
            value={formatQuaternionValue(value)}
            accent={channel ? colorForChannel(channel) : '#5f6b7a'}
            secondary={channel ?? 'Unmapped'}
          />
        )
      })}
    </div>
  )
}

function MetricTile({
  label,
  value,
  accent,
  secondary,
}: {
  label: string
  value: string
  accent: string
  secondary?: string
}) {
  return (
    <div className="imu-metric-tile" style={{ '--imu-accent': accent } as CSSProperties}>
      <small>{label}</small>
      <strong>{value}</strong>
      {secondary ? <span>{secondary}</span> : null}
    </div>
  )
}

function sourceDescription(source: ImuOrientationSource) {
  if (source === 'directAngles') {
    return 'Use already solved roll, pitch, and yaw channels from the device or upstream pipeline.'
  }
  if (source === 'directQuaternion') {
    return 'Use solved quaternion channels directly. This is the most stable path for 3D orientation preview.'
  }
  return 'Prefer Rust fused quaternion generated from mapped accel/gyro channels, then fall back to local accel/gyro approximation if fusion output is unavailable.'
}

function emptyStateMessage(source: ImuOrientationSource) {
  if (source === 'directAngles') {
    return 'Map solved `Roll / Pitch / Yaw` variables to drive the IMU preview directly.'
  }
  if (source === 'directQuaternion') {
    return 'Map solved `Quat W / X / Y / Z` variables to drive the IMU preview directly.'
  }
  return 'Map `Accel X/Y/Z` to render attitude. `Gyro Z` adds live yaw integration.'
}

function requiredMappingCount(source: ImuOrientationSource) {
  if (source === 'directAngles') {
    return 3
  }
  if (source === 'directQuaternion') {
    return 4
  }
  return 6
}

function formatAngle(value: number | null) {
  return value === null ? '—' : `${value.toFixed(1)}deg`
}

function formatVectorValue(value: number | null) {
  return value === null ? '—' : value.toFixed(3)
}

function formatQuaternionValue(value: number | null) {
  return value === null ? '—' : value.toFixed(4)
}

function formatTimestamp(timestampMs: number | null) {
  if (timestampMs === null) {
    return 'Awaiting samples'
  }
  return `${(timestampMs / 1000).toFixed(2)}s`
}

function scaleMagnitude(magnitude: number | null, axisPrefix: 'a' | 'g') {
  if (magnitude === null) {
    return 0
  }
  const ceiling = axisPrefix === 'a' ? 2.5 : 360
  return Math.max(6, Math.min(100, (magnitude / ceiling) * 100))
}
