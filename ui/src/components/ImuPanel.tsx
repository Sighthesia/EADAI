import type { CSSProperties } from 'react'
import { useEffect, useMemo, useRef, useState } from 'react'
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
  buildMotionTrajectory,
  buildImuCoordinateScene,
  buildOrientationFromAngles,
  buildOrientationFromQuaternion,
  buildOrientationFromRaw,
  buildVector,
} from '../lib/imu-view'
import { ImuStageScene } from './ImuStageScene'
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

const MIN_IMU_ZOOM = 0.75
const MAX_IMU_ZOOM = 2.5
const DEFAULT_IMU_ZOOM = 1
const AXIS_FALLBACK_COLORS = {
  accelX: '#4FC3F7',
  accelY: '#C792EA',
  accelZ: '#F78C6C',
} satisfies Record<'accelX' | 'accelY' | 'accelZ', string>

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
  const stageShellRef = useRef<HTMLDivElement | null>(null)
  const [menuOpen, setMenuOpen] = useState(true)
  const [zoom, setZoom] = useState(DEFAULT_IMU_ZOOM)
  const channelOptions = useMemo(() => Object.keys(variables).sort(), [variables])
  const accel = useMemo(() => buildVector(variables, imuChannelMap, 'accel'), [imuChannelMap, variables])
  const trajectory = useMemo(
    () => buildMotionTrajectory(variables, imuChannelMap, imuOrientationSource, imuAttitudeMap, imuQuaternionMap),
    [imuAttitudeMap, imuChannelMap, imuOrientationSource, imuQuaternionMap, variables],
  )
  const orientation = useMemo(
    () =>
      imuOrientationSource === 'directAngles'
        ? buildOrientationFromAngles(variables, imuAttitudeMap)
        : imuOrientationSource === 'directQuaternion'
          ? buildOrientationFromQuaternion(variables, imuQuaternionMap)
          : buildOrientationFromRaw(variables, imuChannelMap, accel),
    [accel, imuAttitudeMap, imuChannelMap, imuOrientationSource, imuQuaternionMap, variables],
  )
  const coordinateScene = useMemo(() => buildImuCoordinateScene(), [])
  const axisColors = useMemo(
    () => ({
      accelX: imuChannelMap.accelX ? colorForChannel(imuChannelMap.accelX) : AXIS_FALLBACK_COLORS.accelX,
      accelY: imuChannelMap.accelY ? colorForChannel(imuChannelMap.accelY) : AXIS_FALLBACK_COLORS.accelY,
      accelZ: imuChannelMap.accelZ ? colorForChannel(imuChannelMap.accelZ) : AXIS_FALLBACK_COLORS.accelZ,
    }),
    [colorForChannel, imuChannelMap.accelX, imuChannelMap.accelY, imuChannelMap.accelZ],
  )
  const mappedCount =
    imuOrientationSource === 'directAngles'
      ? countMappedImuAttitudeChannels(imuAttitudeMap)
      : imuOrientationSource === 'directQuaternion'
        ? countMappedImuQuaternionChannels(imuQuaternionMap)
        : countMappedImuChannels(imuChannelMap)

  useEffect(() => {
    const host = stageShellRef.current
    if (!host) {
      return
    }

    const onWheel = (event: WheelEvent) => {
      const target = event.target
      if (!(target instanceof Element)) {
        return
      }

      if (target.closest('.imu-floating-menu')) {
        return
      }

      event.preventDefault()
      setZoom((value) => clampZoom(value * (event.deltaY < 0 ? 1.08 : 0.92)))
    }

    host.addEventListener('wheel', onWheel, { passive: false })
    return () => host.removeEventListener('wheel', onWheel)
  }, [])

  const hasStageContent = Boolean(trajectory || orientation.quaternion || orientation.rollDeg !== null || orientation.pitchDeg !== null || orientation.yawDeg !== null)

  return (
    <section className="panel imu-panel">
      <div className="imu-stage-shell" ref={stageShellRef}>
        <ImuStageScene
          zoom={zoom}
          coordinateScene={coordinateScene}
          orientation={orientation}
          trajectory={trajectory}
          axisColors={axisColors}
        />

        {!hasStageContent ? (
          <div className="imu-empty-state imu-stage-surface">
            <div>{emptyStateMessage(imuOrientationSource)}</div>
            <small>Stage keeps a readable fallback grid until full motion data arrives.</small>
            {trajectory ? <small>{trajectory.sampleCount} trajectory samples ready</small> : null}
          </div>
        ) : null}

        <div className="imu-stage-hud">
          <div className="imu-stage-meta">
            <strong>IMU</strong>
            <span>{orientation.sourceLabel}</span>
            <small>{formatTimestamp(orientation.timestampMs)}</small>
          </div>
          <div className="imu-attitude-grid imu-attitude-grid-overlay">
            <MetricTile label="Roll" value={formatAngle(orientation.rollDeg)} angle={orientation.rollDeg} accent="#4FC3F7" />
            <MetricTile label="Pitch" value={formatAngle(orientation.pitchDeg)} angle={orientation.pitchDeg} accent="#C792EA" />
            <MetricTile label="Yaw" value={formatAngle(orientation.yawDeg)} angle={orientation.yawDeg} accent="#F78C6C" />
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
                  <span className="imu-chip">
                    {mappedCount}/{requiredMappingCount(imuOrientationSource)} mapped
                  </span>
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
                  <strong>Stage Zoom</strong>
                  <small>{Math.round(zoom * 100)}%</small>
                </div>
                <div className="imu-controls-row">
                  <button type="button" className="ghost-button imu-zoom-button" onClick={() => setZoom((value) => clampZoom(value * 1.12))} aria-label="Zoom in">
                    +
                  </button>
                  <input
                    className="imu-zoom-slider"
                    type="range"
                    min={MIN_IMU_ZOOM}
                    max={MAX_IMU_ZOOM}
                    step={0.01}
                    value={zoom}
                    onChange={(event) => setZoom(clampZoom(Number(event.target.value)))}
                    aria-label="IMU stage zoom"
                  />
                  <button type="button" className="ghost-button imu-zoom-button" onClick={() => setZoom((value) => clampZoom(value / 1.12))} aria-label="Zoom out">
                    −
                  </button>
                </div>
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
  angle,
  accent,
  secondary,
}: {
  label: string
  value: string
  angle: number | null
  accent: string
  secondary?: string
}) {
  return (
    <div className="imu-metric-tile" style={{ '--imu-accent': accent } as CSSProperties}>
      <div className="imu-metric-heading-row">
        <small>{label}</small>
        <AngleGauge angle={angle} accent={accent} />
      </div>
      <strong>{value}</strong>
      {secondary ? <span>{secondary}</span> : null}
    </div>
  )
}

function AngleGauge({ angle, accent }: { angle: number | null; accent: string }) {
  const normalizedAngle = angle === null ? 0 : ((angle % 360) + 360) % 360
  const rotation = normalizedAngle - 90

  return (
    <svg viewBox="0 0 32 32" className="imu-angle-gauge" aria-hidden="true">
      <circle cx="16" cy="16" r="11.5" className="imu-angle-gauge-ring" />
      <circle cx="16" cy="16" r="2.2" fill={accent} fillOpacity="0.92" />
      {angle === null ? (
        <path d="M11 16h10" className="imu-angle-gauge-needle imu-angle-gauge-needle-idle" />
      ) : (
        <g transform={`rotate(${rotation} 16 16)`}>
          <path d="M16 6.5v9.5" className="imu-angle-gauge-needle" style={{ stroke: accent }} />
          <circle cx="16" cy="6.5" r="1.7" fill={accent} />
        </g>
      )}
    </svg>
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
  return value === null ? '—' : `${value.toFixed(1)}°`
}

function formatTimestamp(timestampMs: number | null) {
  if (timestampMs === null) {
    return 'Awaiting samples'
  }
  return `${(timestampMs / 1000).toFixed(2)}s`
}

function clampZoom(value: number) {
  return Math.min(MAX_IMU_ZOOM, Math.max(MIN_IMU_ZOOM, value))
}
