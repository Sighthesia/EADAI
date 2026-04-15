import { type ReactNode, useEffect, useMemo, useState } from 'react'
import {
  IMU_ATTITUDE_LABELS,
  IMU_ATTITUDE_ROLES,
  IMU_CHANNEL_ROLES,
  IMU_QUATERNION_LABELS,
  IMU_QUATERNION_ROLES,
  IMU_ROLE_LABELS,
} from '../lib/imu'
import { useAppStore } from '../store/appStore'
import type {
  ImuAttitudeMap,
  ImuAttitudeRole,
  ImuChannelMap,
  ImuChannelRole,
  ImuQuaternionMap,
  ImuQuaternionRole,
  UiAnalysisPayload,
  VariableEntry,
} from '../types'

type ContextMenuState = {
  channel: string
  x: number
  y: number
}

type MetricDisplayMode = 'text' | 'icon' | 'mixed'

type MappingAction =
  | { type: 'raw'; role: ImuChannelRole; label: string }
  | { type: 'attitude'; role: ImuAttitudeRole; label: string }
  | { type: 'quaternion'; role: ImuQuaternionRole; label: string }

const RAW_MAPPING_ACTIONS: MappingAction[] = IMU_CHANNEL_ROLES.map((role) => ({
  type: 'raw',
  role,
  label: IMU_ROLE_LABELS[role],
}))

const ATTITUDE_MAPPING_ACTIONS: MappingAction[] = IMU_ATTITUDE_ROLES.map((role) => ({
  type: 'attitude',
  role,
  label: IMU_ATTITUDE_LABELS[role],
}))

const QUATERNION_MAPPING_ACTIONS: MappingAction[] = IMU_QUATERNION_ROLES.map((role) => ({
  type: 'quaternion',
  role,
  label: IMU_QUATERNION_LABELS[role],
}))

const METRIC_DISPLAY_MODES: MetricDisplayMode[] = ['text', 'icon', 'mixed']
const METRIC_DISPLAY_LABELS: Record<MetricDisplayMode, string> = {
  text: 'Text',
  icon: 'Icon',
  mixed: 'Mixed',
}
const METRIC_DISPLAY_STORAGE_KEY = 'eadai:variables:metricDisplayMode'

export function VariablesPanel() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const imuChannelMap = useAppStore((state) => state.imuChannelMap)
  const imuAttitudeMap = useAppStore((state) => state.imuAttitudeMap)
  const imuQuaternionMap = useAppStore((state) => state.imuQuaternionMap)
  const toggleChannel = useAppStore((state) => state.toggleChannel)
  const setImuChannel = useAppStore((state) => state.setImuChannel)
  const setImuAttitudeChannel = useAppStore((state) => state.setImuAttitudeChannel)
  const setImuQuaternionChannel = useAppStore((state) => state.setImuQuaternionChannel)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const [metricDisplayMode, setMetricDisplayMode] = useState<MetricDisplayMode>(() => readMetricDisplayMode())
  const rows = useMemo(
    () => Object.values(variables).sort((left, right) => right.updatedAtMs - left.updatedAtMs),
    [variables],
  )
  const channelRoleMap = useMemo(
    () => buildChannelRoleMap(imuChannelMap, imuAttitudeMap, imuQuaternionMap),
    [imuAttitudeMap, imuChannelMap, imuQuaternionMap],
  )

  useEffect(() => {
    window.localStorage.setItem(METRIC_DISPLAY_STORAGE_KEY, metricDisplayMode)
  }, [metricDisplayMode])

  useEffect(() => {
    if (!contextMenu) {
      return
    }

    const closeMenu = () => setContextMenu(null)
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setContextMenu(null)
      }
    }

    window.addEventListener('pointerdown', closeMenu)
    window.addEventListener('scroll', closeMenu, true)
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      window.removeEventListener('pointerdown', closeMenu)
      window.removeEventListener('scroll', closeMenu, true)
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [contextMenu])

  const assignImuRole = (channel: string, action: MappingAction) => {
    if (action.type === 'raw') {
      setImuChannel(action.role, channel)
    } else if (action.type === 'attitude') {
      setImuAttitudeChannel(action.role, channel)
    } else {
      setImuQuaternionChannel(action.role, channel)
    }

    setContextMenu(null)
  }

  const clearImuRoles = (channel: string) => {
    for (const role of IMU_CHANNEL_ROLES) {
      if (imuChannelMap[role] === channel) {
        setImuChannel(role, null)
      }
    }
    for (const role of IMU_ATTITUDE_ROLES) {
      if (imuAttitudeMap[role] === channel) {
        setImuAttitudeChannel(role, null)
      }
    }
    for (const role of IMU_QUATERNION_ROLES) {
      if (imuQuaternionMap[role] === channel) {
        setImuQuaternionChannel(role, null)
      }
    }

    setContextMenu(null)
  }

  return (
    <section className="panel panel-scroll">
      <div className="variables-header">
        <span>Auto-discovered channels</span>
        <div className="variables-header-actions">
          <span>{rows.length} · Right-click to assign IMU source</span>
          <div className="metric-display-switch" role="group" aria-label="Metric display mode">
            {METRIC_DISPLAY_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                className={`metric-display-button ${metricDisplayMode === mode ? 'active' : ''}`}
                onClick={() => setMetricDisplayMode(mode)}
              >
                {METRIC_DISPLAY_LABELS[mode]}
              </button>
            ))}
          </div>
        </div>
      </div>
      <div className="variables-list">
        {rows.map((variable) => {
          const selected = selectedChannels.includes(variable.name)
          const roleLabels = channelRoleMap[variable.name] ?? []
          const mapped = roleLabels.length > 0
          return (
            <article
              key={variable.name}
              className={`variable-card ${selected ? 'selected' : ''} ${mapped ? 'imu-mapped' : ''}`}
              role="button"
              tabIndex={0}
              onClick={() => toggleChannel(variable.name)}
              onContextMenu={(event) => {
                event.preventDefault()
                setContextMenu({ channel: variable.name, x: event.clientX, y: event.clientY })
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  toggleChannel(variable.name)
                }
              }}
            >
              <span
                className={`variable-selection-bar ${selected ? 'active' : ''}`}
                style={{ backgroundColor: selected ? colorForChannel(variable.name) : undefined }}
              />
              <span className="variable-color" style={{ backgroundColor: colorForChannel(variable.name) }} />
              <div className="variable-main">
                <div className="variable-title-row">
                  <strong>{variable.name}</strong>
                  {roleLabels.length > 0 ? (
                    <div className="variable-role-chip-row">
                      {roleLabels.map((label) => (
                        <span key={`${variable.name}-${label}`} className={`variable-role-chip role-${roleTone(label)}`}>
                          {label}
                        </span>
                      ))}
                    </div>
                  ) : null}
                </div>
                <div className="variable-subline">
                  <small>{variable.parserName ?? 'raw'}</small>
                  <span className="metric-chip">◌ {variable.sampleCount}</span>
                  {variable.latestTrigger ? (
                    <span className={`trigger-pill trigger-pill-${variable.latestTrigger.severity}`}>
                      {variable.latestTrigger.ruleId}
                    </span>
                  ) : null}
                </div>
              </div>
              <div className="variable-metric">
                <strong>{formatPrimaryValue(variable)}</strong>
                <small className={`trend trend-${variable.trend}`}>{trendLabel(variable, metricDisplayMode)}</small>
                <small>{renderSummaryMetric(variable, metricDisplayMode)}</small>
              </div>
            </article>
          )
        })}
      </div>
      {contextMenu ? (
        <div
          className="variable-context-menu"
          style={{ left: clampMenuX(contextMenu.x), top: clampMenuY(contextMenu.y) }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <div className="variable-context-header">
            <strong>{contextMenu.channel}</strong>
            <small>Assign this variable as an IMU source</small>
          </div>
          <ContextMenuGroup
            title="Raw Sensors"
            actions={RAW_MAPPING_ACTIONS}
            onSelect={(action) => assignImuRole(contextMenu.channel, action)}
          />
          <ContextMenuGroup
            title="Solved Angles"
            actions={ATTITUDE_MAPPING_ACTIONS}
            onSelect={(action) => assignImuRole(contextMenu.channel, action)}
          />
          <ContextMenuGroup
            title="Quaternion"
            actions={QUATERNION_MAPPING_ACTIONS}
            onSelect={(action) => assignImuRole(contextMenu.channel, action)}
          />
          <button type="button" className="variable-context-clear" onClick={() => clearImuRoles(contextMenu.channel)}>
            Clear IMU roles from this variable
          </button>
        </div>
      ) : null}
    </section>
  )
}

function ContextMenuGroup({
  title,
  actions,
  onSelect,
}: {
  title: string
  actions: MappingAction[]
  onSelect: (action: MappingAction) => void
}) {
  return (
    <div className="variable-context-group">
      <span>{title}</span>
      <div className="variable-context-actions">
        {actions.map((action) => (
          <button key={`${action.type}-${action.role}`} type="button" className="variable-context-action" onClick={() => onSelect(action)}>
            {action.label}
          </button>
        ))}
      </div>
    </div>
  )
}

function buildChannelRoleMap(
  imuChannelMap: ImuChannelMap,
  imuAttitudeMap: ImuAttitudeMap,
  imuQuaternionMap: ImuQuaternionMap,
) {
  const grouped: Record<string, string[]> = {}
  appendRoleBindings(grouped, imuChannelMap, IMU_CHANNEL_ROLES, IMU_ROLE_LABELS)
  appendRoleBindings(grouped, imuAttitudeMap, IMU_ATTITUDE_ROLES, IMU_ATTITUDE_LABELS)
  appendRoleBindings(grouped, imuQuaternionMap, IMU_QUATERNION_ROLES, IMU_QUATERNION_LABELS)
  return Object.fromEntries(Object.entries(grouped).map(([channel, labels]) => [channel, labels.sort()]))
}

function appendRoleBindings<T extends string>(
  grouped: Record<string, string[]>,
  map: Record<T, string | null>,
  roles: readonly T[],
  labels: Record<T, string>,
) {
  for (const role of roles) {
    const channel = map[role]
    if (!channel) {
      continue
    }

    grouped[channel] = [...(grouped[channel] ?? []), labels[role]]
  }
}

function clampMenuX(value: number) {
  return Math.max(12, Math.min(value, window.innerWidth - 320))
}

function clampMenuY(value: number) {
  return Math.max(12, Math.min(value, window.innerHeight - 420))
}

function renderSummaryMetric(variable: VariableEntry, mode: MetricDisplayMode) {
  if (variable.latestTrigger) {
    return variable.latestTrigger.reason
  }
  if (variable.analysis?.dutyCycle !== undefined && variable.analysis?.dutyCycle !== null) {
    return (
      <MetricInline mode={mode} icon={<DutyCycleIcon />} text={`Duty ${variable.analysis.dutyCycle.toFixed(0)}%`} compactText={`${variable.analysis.dutyCycle.toFixed(0)}%`} />
    )
  }
  if (variable.analysis?.rmsValue !== undefined && variable.analysis?.rmsValue !== null) {
    return (
      <MetricInline mode={mode} icon={<RmsIcon />} text={`RMS ${variable.analysis.rmsValue.toFixed(3)}`} compactText={variable.analysis.rmsValue.toFixed(3)} />
    )
  }
  if (variable.analysis?.frequencyHz !== undefined && variable.analysis?.frequencyHz !== null) {
    return mode === 'text' ? `Frequency ${variable.analysis.frequencyHz.toFixed(1)} Hz` : `ƒ ${variable.analysis.frequencyHz.toFixed(1)} Hz`
  }
  if (variable.unit) {
    return normalizeUnitLabel(variable.unit)
  }
  if (variable.analysis?.meanValue !== undefined && variable.analysis?.meanValue !== null) {
    return `μ ${variable.analysis.meanValue.toFixed(3)}`
  }
  return `${variable.triggerCount} triggers`
}

const trendLabel = (variable: VariableEntry, mode: MetricDisplayMode) => {
  if (variable.analysis?.changeRate !== undefined && variable.analysis?.changeRate !== null) {
    const prefix = mode === 'text' ? trendWord(variable.trend) : trendSymbol(variable.trend)
    return `${prefix} ${Math.abs(variable.analysis.changeRate).toFixed(2)}/s`
  }
  return mode === 'text' ? trendWord(variable.trend) : trendSymbol(variable.trend)
}

function formatPrimaryValue(variable: VariableEntry) {
  if (!variable.unit) {
    return variable.currentValue
  }

  const normalizedUnit = normalizeUnitLabel(variable.unit)
  if (normalizedUnit === '°' && !variable.currentValue.includes('°')) {
    return `${variable.currentValue}${normalizedUnit}`
  }
  return variable.currentValue
}

function normalizeUnitLabel(unit: string) {
  const normalized = unit.trim().toLowerCase()
  if (normalized === 'deg' || normalized === 'degree' || normalized === 'degrees' || normalized === '°') {
    return '°'
  }
  if (normalized === 'percent' || normalized === 'pct' || normalized === '%') {
    return '%'
  }
  if (normalized === 'celsius' || normalized === 'degc' || normalized === '°c') {
    return '°C'
  }
  if (normalized === 'fahrenheit' || normalized === 'degf' || normalized === '°f') {
    return '°F'
  }
  return unit
}

function trendSymbol(trend: VariableEntry['trend']) {
  if (trend === 'up') {
    return '↑'
  }
  if (trend === 'down') {
    return '↓'
  }
  return '-'
}

function trendWord(trend: VariableEntry['trend']) {
  if (trend === 'up') {
    return 'Up'
  }
  if (trend === 'down') {
    return 'Down'
  }
  return 'Flat'
}

function roleTone(label: string) {
  if (label.startsWith('Accel')) {
    return 'accel'
  }
  if (label.startsWith('Gyro')) {
    return 'gyro'
  }
  if (label.startsWith('Quat')) {
    return 'quaternion'
  }
  return 'attitude'
}

function readMetricDisplayMode(): MetricDisplayMode {
  const saved = window.localStorage.getItem(METRIC_DISPLAY_STORAGE_KEY)
  return isMetricDisplayMode(saved) ? saved : 'mixed'
}

function isMetricDisplayMode(value: string | null): value is MetricDisplayMode {
  return value === 'text' || value === 'icon' || value === 'mixed'
}

function MetricInline({
  mode,
  icon,
  text,
  compactText,
}: {
  mode: MetricDisplayMode
  icon: ReactNode
  text: string
  compactText: string
}) {
  if (mode === 'text') {
    return text
  }
  return (
    <span className="metric-inline">
      {icon}
      <span>{mode === 'icon' ? compactText : text}</span>
    </span>
  )
}

function DutyCycleIcon() {
  return (
    <svg viewBox="0 0 16 16" className="metric-inline-icon" aria-hidden="true">
      <path d="M1 10V6h4v4h4V6h6" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

function RmsIcon() {
  return (
    <svg viewBox="0 0 16 16" className="metric-inline-icon" aria-hidden="true">
      <path d="M1.5 9.5 3.5 6l2.5 5 2.5-8 2.5 8 1.5-3h2" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}
