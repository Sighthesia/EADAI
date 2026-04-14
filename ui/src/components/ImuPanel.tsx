import type { CSSProperties } from 'react'
import { useMemo, useState } from 'react'
import {
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
  const [menuOpen, setMenuOpen] = useState(true)
  const channelOptions = useMemo(() => Object.keys(variables).sort(), [variables])
  const accel = useMemo(() => buildVector(variables, imuChannelMap, 'accel'), [imuChannelMap, variables])
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
    <section className="panel imu-panel">
      <div className="imu-stage-shell">
        {preview ? (
          <svg viewBox="0 0 280 220" className="imu-stage imu-stage-surface" aria-label="IMU orientation preview">
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
          <div className="imu-empty-state imu-stage-surface">{emptyStateMessage(imuOrientationSource)}</div>
        )}

        <div className="imu-stage-hud">
          <div className="imu-stage-meta">
            <strong>IMU</strong>
            <span>{orientation.sourceLabel}</span>
            <small>{formatTimestamp(orientation.timestampMs)}</small>
          </div>
          <div className="imu-attitude-grid imu-attitude-grid-overlay">
            <MetricTile label="Roll" value={formatAngle(orientation.rollDeg)} accent="#4FC3F7" />
            <MetricTile label="Pitch" value={formatAngle(orientation.pitchDeg)} accent="#C792EA" />
            <MetricTile label="Yaw" value={formatAngle(orientation.yawDeg)} accent="#F78C6C" />
          </div>
        </div>

        <aside className={`imu-floating-menu ${menuOpen ? 'open' : 'collapsed'}`}>
          <div className="imu-floating-top">
            {menuOpen ? (
              <div className="imu-floating-heading">
                <strong>Controls</strong>
                <small>{sourceDescription(imuOrientationSource)}</small>
              </div>
            ) : null}
            <button type="button" className="ghost-button imu-floating-toggle" onClick={() => setMenuOpen((value) => !value)}>
              {menuOpen ? 'Hide' : 'Controls'}
            </button>
          </div>

          {menuOpen ? (
            <div className="imu-floating-scroll">
              <section className="imu-floating-section">
                <div className="imu-floating-section-header">
                  <strong>Status</strong>
                  <small>{formatTimestamp(imuQuality.timestampMs)}</small>
                </div>
                <div className="imu-floating-chip-row">
                  <span className={`status-pill tone-${imuQuality.level}`}>{imuQuality.label}</span>
                  <span className="imu-chip">{mappedCount}/{requiredMappingCount(imuOrientationSource)} mapped</span>
                  <span className="imu-chip">{imuMapMode}</span>
                </div>
                <p className="imu-floating-copy">{imuQuality.details}</p>
              </section>

              <section className="imu-floating-section">
                <div className="imu-floating-section-header">
                  <strong>Orientation Source</strong>
                  <button type="button" className="ghost-button" onClick={() => autoMapImuChannels()}>
                    Auto Detect
                  </button>
                </div>
                <SourceChoiceGroup
                  ariaLabel="IMU orientation source"
                  className="imu-mode-switch"
                  value={imuOrientationSource}
                  options={SOURCE_OPTIONS}
                  onChange={(value) => setImuOrientationSource(value)}
                />
              </section>

              <section className="imu-floating-section">
                <div className="imu-floating-section-header">
                  <strong>Channel Mapping</strong>
                  <small>{mappingSectionLabel(imuOrientationSource)}</small>
                </div>
                {imuOrientationSource === 'directAngles' ? (
                  <AttitudeSourceGrid channelOptions={channelOptions} map={imuAttitudeMap} onChange={setImuAttitudeChannel} />
                ) : imuOrientationSource === 'directQuaternion' ? (
                  <QuaternionSourceGrid channelOptions={channelOptions} map={imuQuaternionMap} onChange={setImuQuaternionChannel} />
                ) : (
                  <RawSourceGrid channelOptions={channelOptions} map={imuChannelMap} onChange={setImuChannel} />
                )}
              </section>

              <section className="imu-floating-section">
                <div className="imu-floating-section-header">
                  <strong>Calibration</strong>
                  <small>{imuCalibration.lastCalibratedAtMs ? formatTimestamp(imuCalibration.lastCalibratedAtMs) : 'Not stored'}</small>
                </div>
                <div className="imu-calibration-chips">
                  <span className={`imu-chip ${imuCalibration.accelBiasApplied ? 'active' : ''}`}>Accel bias {imuCalibration.accelBiasApplied ? 'set' : 'unset'}</span>
                  <span className={`imu-chip ${imuCalibration.gyroBiasApplied ? 'active' : ''}`}>Gyro bias {imuCalibration.gyroBiasApplied ? 'set' : 'unset'}</span>
                  <span className="imu-chip">Source {imuCalibration.sourceLabel ?? 'none'}</span>
                </div>
                <div className="toolbar-row imu-calibration-actions">
                  <button type="button" className="ghost-button" onClick={() => calibrateImuFromCurrentState()}>
                    Capture Calibration
                  </button>
                  <button type="button" className="ghost-button" onClick={() => resetImuCalibration()}>
                    Reset
                  </button>
                </div>
                <p className="imu-floating-copy">Acceleration unit: {accel.unit ?? 'raw'}</p>
              </section>
            </div>
          ) : null}
        </aside>
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
    return 'Use solved roll, pitch, and yaw channels directly.'
  }
  if (source === 'directQuaternion') {
    return 'Use solved quaternion channels for the most stable 3D pose.'
  }
  return 'Fuse mapped accel and gyro channels, with local fallback when needed.'
}

function emptyStateMessage(source: ImuOrientationSource) {
  if (source === 'directAngles') {
    return 'Map `Roll / Pitch / Yaw` to drive the IMU stage.'
  }
  if (source === 'directQuaternion') {
    return 'Map `Quat W / X / Y / Z` to drive the IMU stage.'
  }
  return 'Map `Accel X/Y/Z` and `Gyro X/Y/Z` to unlock fused attitude.'
}

function mappingSectionLabel(source: ImuOrientationSource) {
  if (source === 'directAngles') {
    return '3 channels'
  }
  if (source === 'directQuaternion') {
    return '4 channels'
  }
  return '6 channels'
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

function formatTimestamp(timestampMs: number | null) {
  if (timestampMs === null) {
    return 'Awaiting samples'
  }
  return `${(timestampMs / 1000).toFixed(2)}s`
}
