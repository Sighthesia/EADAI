import type uPlot from 'uplot'
import type { UiAnalysisPayload, VariableEntry } from '../../types'

// ── Plot data model ─────────────────────────────────────────────────────────

export type NumericStats = {
  minValue: number
  maxValue: number
  meanValue: number
  medianValue: number
  changeRate?: number | null
}

export type PeriodStats = {
  periodMs: number | null
  cycleStartsMs: number[]
  meanValue: number | null
  medianValue: number | null
}

export type PlotTrack = {
  name: string
  color: string
  variable: VariableEntry
  labelsEnabled: boolean
  rangeEnabled: boolean
  meanEnabled: boolean
  medianEnabled: boolean
  periodEnabled: boolean
  slopeEnabled: boolean
  points: Array<{ timestampMs: number; value: number }>
  displayValue: string
  seriesIndex: number
  stats: NumericStats
  period: PeriodStats
}

export type TextTrack = {
  name: string
  color: string
  textEnabled: boolean
  value: string
  updatedAtMs: number
}

export type PlotModel = {
  data: uPlot.AlignedData
  series: uPlot.Series[]
  numericTracks: PlotTrack[]
  textTracks: TextTrack[]
  windowStartMs: number
  windowEndMs: number
  xMin: number
  xMax: number
  yMin: number
  yMax: number
}

// ── Overlay state types ─────────────────────────────────────────────────────

export type OverlayItemElements = {
  leftLabel: HTMLDivElement
  latestLabel: HTMLDivElement
  cursorLabel: HTMLDivElement
  meanLabel: HTMLDivElement
  medianLabel: HTMLDivElement
  calloutLine: SVGLineElement
  cursorGuideLine: SVGLineElement
  cursorCalloutLine: SVGLineElement
  meanLine: SVGLineElement
  medianLine: SVGLineElement
  minLine: SVGLineElement
  maxLine: SVGLineElement
  slopeLine: SVGLineElement
  latestDot: SVGCircleElement
  cursorDot: SVGCircleElement
}

export type OverlayState = {
  plot: uPlot
  overlayRoot: HTMLDivElement
  linesLayer: SVGSVGElement
  labelsLayer: HTMLDivElement
  textTrackLayer: HTMLDivElement
  items: Map<string, OverlayItemElements>
  cursorAnimations: Map<string, CursorAnimationState>
  cursorFrameId: number | null
  latestAnimations: Map<string, LatestAnimationState>
  latestFrameId: number | null
  periodLines: Map<string, SVGLineElement[]>
  textTrackSignature: string
  debugLogIntervalId: number | null
}

export type OverlayAnimationTarget = {
  color: string
  anchorX: number
  labelX: number
  value: number
}

export type CursorAnimationState = {
  currentX: number
  currentValue: number
  target: OverlayAnimationTarget
}

export type LatestAnimationState = {
  currentAnchorX: number
  currentX: number
  currentLabelValue: number
  target: OverlayAnimationTarget
}

export type YScaleAnimationState = {
  currentMin: number
  currentMax: number
  targetMin: number
  targetMax: number
  frameId: number | null
}

// ── Shared constants ────────────────────────────────────────────────────────

export const SVG_NS = 'http://www.w3.org/2000/svg'

export const SLOPE_REGRESSION_POINT_COUNT = 8

export const CURSOR_LABEL_GAP_PX = 42
export const CURSOR_LABEL_MIN_CENTER_Y = 22
export const CURSOR_LABEL_MAX_WIDTH_PX = 180
export const CURSOR_CALLOUT_THRESHOLD_PX = 1
export const CURSOR_SMOOTHING_X = 0.45
export const CURSOR_SMOOTHING_Y = 0.2
export const CURSOR_SNAP_DISTANCE_PX = 120
export const CURSOR_ANIMATION_EPSILON_PX = 0.5
export const LATEST_SMOOTHING_X = 1
export const LATEST_SMOOTHING_Y = 1
export const MAX_CURSOR_LABEL_TRACKS = 4
export const MAX_PERSISTENT_LABEL_TRACKS = 8
export const OVERLAY_MOTION_DEBUG_STORAGE_KEY = 'eadai:waveform-overlay-motion-debug'
