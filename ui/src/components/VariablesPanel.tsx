import { memo, type PointerEvent as ReactPointerEvent, type ReactNode, useEffect, useMemo, useRef, useState } from 'react'
import {
  IMU_ATTITUDE_LABELS,
  IMU_ATTITUDE_ROLES,
  IMU_CHANNEL_ROLES,
  IMU_QUATERNION_LABELS,
  IMU_QUATERNION_ROLES,
  IMU_ROLE_LABELS,
} from '../lib/imu'
import { computeStableCycleStats } from '../lib/waveformPeriod'
import { isWaveformVisualAidEnabled, type WaveformVisualAidKey, type WaveformVisualAidState } from '../lib/waveformVisualAids'
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
const TREND_REGRESSION_POINT_COUNT = 8

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
  const visualAidState = useAppStore((state) => state.visualAidState)
  const setVisualAidEnabled = useAppStore((state) => state.setVisualAidEnabled)
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const contextMenuRef = useRef<HTMLDivElement>(null)
  const [metricDisplayMode, setMetricDisplayMode] = useState<MetricDisplayMode>(() => readMetricDisplayMode())
  const rows = useMemo(
    () =>
      Object.values(variables).sort((left, right) =>
        left.name.localeCompare(right.name, undefined, { numeric: true, sensitivity: 'base' }),
      ),
    [variables],
  )
  const selectedChannelSet = useMemo(() => new Set(selectedChannels), [selectedChannels])
  const colorByChannel = useMemo(() => {
    const map = new Map<string, string>()

    for (const variable of rows) {
      map.set(variable.name, colorForChannel(variable.name))
    }

    return map
  }, [colorForChannel, rows])
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

    const closeMenu = (event: Event) => {
      const target = event.target
      if (target instanceof Node && contextMenuRef.current?.contains(target)) {
        return
      }
      setContextMenu(null)
    }
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
          <span>{rows.length} · Right-click to assign IMU source or tune waveform overlays</span>
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
          const channel = variable.name
          return (
            <VariableRow
              key={channel}
              variable={variable}
              selected={selectedChannelSet.has(channel)}
              roleLabels={channelRoleMap[channel] ?? []}
              metricDisplayMode={metricDisplayMode}
              color={colorByChannel.get(channel) ?? colorForChannel(channel)}
              onToggle={toggleChannel}
              onOpenContextMenu={(channel, x, y) => setContextMenu({ channel, x, y })}
            />
          )
        })}
      </div>
      {contextMenu ? (
        <VariableContextMenu
          menuRef={contextMenuRef}
          channel={contextMenu.channel}
          visualAidItems={buildVisualAidMenuItems(variables[contextMenu.channel], visualAidState, contextMenu.channel)}
          x={contextMenu.x}
          y={contextMenu.y}
          onPointerDown={(event) => event.stopPropagation()}
          onScrollCapture={(event) => event.stopPropagation()}
          onWheelCapture={(event) => event.stopPropagation()}
          onToggleVisualAid={(key, enabled) => setVisualAidEnabled(contextMenu.channel, key, enabled)}
          onClearImuRoles={() => clearImuRoles(contextMenu.channel)}
          onAssignImuRole={(action) => assignImuRole(contextMenu.channel, action)}
        />
      ) : null}
    </section>
  )
}

const VariableRow = memo(function VariableRow({
  variable,
  selected,
  roleLabels,
  metricDisplayMode,
  color,
  onToggle,
  onOpenContextMenu,
}: {
  variable: VariableEntry
  selected: boolean
  roleLabels: string[]
  metricDisplayMode: MetricDisplayMode
  color: string
  onToggle: (channel: string) => void
  onOpenContextMenu: (channel: string, x: number, y: number) => void
}) {
  const mapped = roleLabels.length > 0
  const primaryValue = useMemo(() => formatPrimaryValue(variable), [variable])
  const trendText = useMemo(() => trendLabel(variable, metricDisplayMode), [metricDisplayMode, variable])
  const summaryMetric = useMemo(() => renderSummaryMetric(variable, metricDisplayMode), [metricDisplayMode, variable])

  return (
    <article
      className={`variable-card ${selected ? 'selected' : ''} ${mapped ? 'imu-mapped' : ''}`}
      role="button"
      tabIndex={0}
      onClick={() => onToggle(variable.name)}
      onContextMenu={(event) => {
        event.preventDefault()
        onOpenContextMenu(variable.name, event.clientX, event.clientY)
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onToggle(variable.name)
        }
      }}
    >
      <span
        className={`variable-selection-bar ${selected ? 'active' : ''}`}
        style={{ backgroundColor: selected ? color : undefined }}
      />
      <span className="variable-color" style={{ backgroundColor: color }} />
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
        <strong>{primaryValue}</strong>
        <small className={`trend trend-${variable.trend}`}>{trendText}</small>
        <small>{summaryMetric}</small>
      </div>
    </article>
  )
})

function VariableContextMenu({
  menuRef,
  channel,
  visualAidItems,
  x,
  y,
  onPointerDown,
  onScrollCapture,
  onWheelCapture,
  onToggleVisualAid,
  onAssignImuRole,
  onClearImuRoles,
}: {
  menuRef: React.RefObject<HTMLDivElement>
  channel: string
  visualAidItems: VisualAidMenuItem[]
  x: number
  y: number
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void
  onScrollCapture: (event: React.UIEvent<HTMLDivElement>) => void
  onWheelCapture: (event: React.WheelEvent<HTMLDivElement>) => void
  onToggleVisualAid: (key: WaveformVisualAidKey, enabled: boolean) => void
  onAssignImuRole: (action: MappingAction) => void
  onClearImuRoles: () => void
}) {
  return (
    <div
      ref={menuRef}
      className="variable-context-menu"
      style={{ left: clampMenuX(x), top: clampMenuY(y) }}
      onPointerDown={onPointerDown}
      onScrollCapture={onScrollCapture}
      onWheelCapture={onWheelCapture}
    >
      <div className="variable-context-header">
        <strong>{channel}</strong>
        <small>Assign this variable as an IMU source or tune its waveform overlays</small>
      </div>
      <CollapsibleContextSection title="Waveform overlays" defaultOpen>
        <div className="variable-context-scrollbox variable-context-actions-stack">
          {visualAidItems.map((item) => (
            <button
              key={item.key}
              type="button"
              className={`variable-context-toggle ${item.enabled ? 'active' : ''}`}
              onClick={() => onToggleVisualAid(item.key, !item.enabled)}
            >
              <span className="variable-context-toggle-copy">
                <strong>{item.label}</strong>
                <span className="variable-context-toggle-value">{item.value}</span>
                <span className="variable-context-toggle-detail">{item.detail}</span>
              </span>
              <strong className="variable-context-toggle-state">{item.enabled ? 'On' : 'Off'}</strong>
            </button>
          ))}
        </div>
      </CollapsibleContextSection>
      <CollapsibleContextSection title="IMU mappings" defaultOpen>
        <div className="variable-context-scrollbox">
          <ContextMenuGroup title="Raw Sensors" actions={RAW_MAPPING_ACTIONS} onSelect={onAssignImuRole} />
          <ContextMenuGroup title="Solved Angles" actions={ATTITUDE_MAPPING_ACTIONS} onSelect={onAssignImuRole} />
          <ContextMenuGroup title="Quaternion" actions={QUATERNION_MAPPING_ACTIONS} onSelect={onAssignImuRole} />
        </div>
      </CollapsibleContextSection>
      <button type="button" className="variable-context-clear" onClick={onClearImuRoles}>
        Clear IMU roles from this variable
      </button>
    </div>
  )
}

type VisualAidMenuItem = {
  key: WaveformVisualAidKey
  label: string
  enabled: boolean
  value: ReactNode
  detail: ReactNode
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

function CollapsibleContextSection({
  title,
  defaultOpen,
  children,
}: {
  title: string
  defaultOpen: boolean
  children: ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <section className="variable-context-section">
      <button type="button" className="variable-context-section-toggle" onClick={() => setOpen((value) => !value)}>
        <span>{title}</span>
        <strong>{open ? 'Collapse' : 'Expand'}</strong>
      </button>
      {open ? children : null}
    </section>
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

function isNumericVariable(variable: VariableEntry) {
  if (Number.isFinite(variable.numericValue)) {
    return true
  }
  if (variable.points.length > 0) {
    return true
  }
  return [variable.analysis?.minValue, variable.analysis?.maxValue, variable.analysis?.meanValue, variable.analysis?.medianValue, variable.analysis?.changeRate].some((value) =>
    Number.isFinite(value ?? Number.NaN),
  )
}

function buildVisualAidMenuItems(variable: VariableEntry | undefined, visualAidState: WaveformVisualAidState, channel: string): VisualAidMenuItem[] {
  if (!variable) {
    return []
  }

  if (!isNumericVariable(variable)) {
    return [
      {
        key: 'text',
        label: 'Text track',
        enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'text'),
        value: formatPrimaryValue(variable),
        detail: variable.parserName ?? 'raw',
      },
    ]
  }

  return [
    {
      key: 'labels',
      label: 'Latest value',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'labels'),
      value: formatPrimaryValue(variable),
      detail: renderSummaryMetric(variable, 'mixed'),
    },
    {
      key: 'range',
      label: 'Min / max lines',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'range'),
      value: formatRangeValue(variable),
      detail: renderRangeMetric(variable),
    },
    {
      key: 'mean',
      label: 'Average line',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'mean'),
      value: formatAverageValue(variable),
      detail: renderAverageMetric(variable),
    },
    {
      key: 'median',
      label: 'Median line',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'median'),
      value: formatMedianValue(variable),
      detail: renderMedianMetric(variable),
    },
    {
      key: 'period',
      label: 'Period markers',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'period'),
      value: formatPeriodValue(variable),
      detail: renderPeriodMetric(variable),
    },
    {
      key: 'slope',
      label: 'Slope line',
      enabled: isWaveformVisualAidEnabled(visualAidState, channel, 'slope'),
      value: trendLabel(variable, 'mixed'),
      detail: renderSummaryMetric(variable, 'mixed'),
    },
  ]
}

function formatRangeValue(variable: VariableEntry) {
  const minValue = variable.analysis?.minValue
  const maxValue = variable.analysis?.maxValue
  if (minValue === undefined || minValue === null || maxValue === undefined || maxValue === null) {
    return 'No range data'
  }
  return `${formatNumericValue(variable, minValue)} to ${formatNumericValue(variable, maxValue)}`
}

function renderRangeMetric(variable: VariableEntry) {
  const minValue = variable.analysis?.minValue
  const maxValue = variable.analysis?.maxValue
  if (minValue === undefined || minValue === null || maxValue === undefined || maxValue === null) {
    return variable.parserName ?? 'raw'
  }
  return `Min ${formatNumericValue(variable, minValue)} · Max ${formatNumericValue(variable, maxValue)}`
}

function formatAverageValue(variable: VariableEntry) {
  const meanValue = resolveAverageValue(variable)
  return meanValue === null ? 'No average data' : formatNumericValue(variable, meanValue)
}

function renderAverageMetric(variable: VariableEntry) {
  const meanValue = resolveAverageValue(variable)
  if (meanValue === null) {
    return variable.parserName ?? 'raw'
  }
  return `Average ${formatNumericValue(variable, meanValue)}`
}

function formatMedianValue(variable: VariableEntry) {
  const medianValue = resolveMedianValue(variable)
  return medianValue === null ? 'No median data' : formatNumericValue(variable, medianValue)
}

function renderMedianMetric(variable: VariableEntry) {
  const medianValue = resolveMedianValue(variable)
  if (medianValue === null) {
    return variable.parserName ?? 'raw'
  }
  return `Median ${formatNumericValue(variable, medianValue)}`
}

function formatPeriodValue(variable: VariableEntry) {
  const periodMs = resolvePeriodValue(variable)
  return periodMs === null ? 'No period data' : formatPeriodDuration(periodMs)
}

function renderPeriodMetric(variable: VariableEntry) {
  const periodMs = resolvePeriodValue(variable)
  if (periodMs === null) {
    return variable.parserName ?? 'raw'
  }

  const frequencyHz = variable.analysis?.frequencyHz
  return frequencyHz === undefined || frequencyHz === null
    ? `Cycle start markers every ${formatPeriodDuration(periodMs)}`
    : `Cycle start markers · ${frequencyHz.toFixed(2)} Hz`
}

function formatNumericValue(variable: VariableEntry, value: number) {
  const abs = Math.abs(value)
  const digits = abs >= 100 ? 1 : abs >= 10 ? 2 : 3
  const text = Number.isInteger(value) ? `${value}` : value.toFixed(digits)
  const unit = variable.unit ? normalizeUnitLabel(variable.unit) : ''
  return `${text}${unit}`
}

function resolveAverageValue(variable: VariableEntry) {
  const cycle = resolveStableCycleStats(variable)
  if (cycle.meanValue !== null) {
    return cycle.meanValue
  }
  if (variable.analysis?.meanValue !== undefined && variable.analysis?.meanValue !== null) {
    return variable.analysis.meanValue
  }
  if (variable.points.length > 0) {
    return variable.points.reduce((sum, point) => sum + point.value, 0) / variable.points.length
  }
  return Number.isFinite(variable.numericValue) ? (variable.numericValue as number) : null
}

function resolveMedianValue(variable: VariableEntry) {
  const cycle = resolveStableCycleStats(variable)
  if (cycle.medianValue !== null) {
    return cycle.medianValue
  }
  if (variable.analysis?.medianValue !== undefined && variable.analysis?.medianValue !== null) {
    return variable.analysis.medianValue
  }
  if (variable.points.length > 0) {
    const values = variable.points.map((point) => point.value).sort((left, right) => left - right)
    const center = Math.floor(values.length / 2)
    return values.length % 2 === 0 ? (values[center - 1] + values[center]) / 2 : values[center]
  }
  return Number.isFinite(variable.numericValue) ? (variable.numericValue as number) : null
}

function resolvePeriodValue(variable: VariableEntry) {
  const cycle = resolveStableCycleStats(variable)
  if (cycle.periodMs !== null) {
    return cycle.periodMs
  }
  if (variable.analysis?.periodMs !== undefined && variable.analysis?.periodMs !== null) {
    return variable.analysis.periodMs
  }

  const frequencyHz = variable.analysis?.frequencyHz
  if (frequencyHz !== undefined && frequencyHz !== null && Number.isFinite(frequencyHz) && Math.abs(frequencyHz) > 1e-6) {
    return 1000 / frequencyHz
  }

  return null
}

function resolveStableCycleStats(variable: VariableEntry) {
  return computeStableCycleStats(variable.points, variable.analysis)
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
  const meanValue = resolveAverageValue(variable)
  if (meanValue !== null) {
    return `μ ${meanValue.toFixed(3)}`
  }
  return `${variable.triggerCount} triggers`
}

const trendLabel = (variable: VariableEntry, mode: MetricDisplayMode) => {
  const changeRate = resolveChangeRateValue(variable)
  if (changeRate !== null) {
    const prefix = mode === 'text' ? trendWord(variable.trend) : trendSymbol(variable.trend)
    return `${prefix} ${Math.abs(changeRate).toFixed(2)}/s`
  }
  return mode === 'text' ? trendWord(variable.trend) : trendSymbol(variable.trend)
}

function resolveChangeRateValue(variable: VariableEntry) {
  if (variable.analysis?.changeRate !== undefined && variable.analysis?.changeRate !== null) {
    return variable.analysis.changeRate
  }

  const trendPoints = variable.points.slice(-TREND_REGRESSION_POINT_COUNT)

  if (trendPoints.length >= 3) {
    const originTimestampMs = trendPoints[0].timestampMs
    let sumX = 0
    let sumY = 0
    let sumXY = 0
    let sumXX = 0

    for (const point of trendPoints) {
      const x = (point.timestampMs - originTimestampMs) / 1000
      const y = point.value
      sumX += x
      sumY += y
      sumXY += x * y
      sumXX += x * x
    }

    const count = trendPoints.length
    const denominator = count * sumXX - sumX * sumX
    if (Number.isFinite(denominator) && Math.abs(denominator) > 1e-6) {
      return (count * sumXY - sumX * sumY) / denominator
    }
  }

  if (trendPoints.length >= 2) {
    const previousPoint = trendPoints[trendPoints.length - 2]
    const latestPoint = trendPoints[trendPoints.length - 1]
    const deltaSeconds = (latestPoint.timestampMs - previousPoint.timestampMs) / 1000
    if (Number.isFinite(deltaSeconds) && Math.abs(deltaSeconds) > 1e-6) {
      return (latestPoint.value - previousPoint.value) / deltaSeconds
    }
  }

  return null
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

function formatPeriodDuration(periodMs: number) {
  if (periodMs >= 1000) {
    const seconds = periodMs / 1000
    return `${seconds.toFixed(seconds >= 10 ? 1 : 2)} s`
  }
  if (periodMs >= 100) {
    return `${periodMs.toFixed(0)} ms`
  }
  if (periodMs >= 10) {
    return `${periodMs.toFixed(1)} ms`
  }
  return `${periodMs.toFixed(2)} ms`
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
