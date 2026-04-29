export type UiTriggerSeverity = 'info' | 'warning' | 'critical'

export interface UiAnalysisPayload {
  channelId: string
  windowMs: number
  sampleCount: number
  timeSpanMs?: number | null
  frequencyHz?: number | null
  periodMs?: number | null
  periodStability?: number | null
  dutyCycle?: number | null
  minValue?: number | null
  maxValue?: number | null
  meanValue?: number | null
  medianValue?: number | null
  rmsValue?: number | null
  variance?: number | null
  edgeCount: number
  risingEdgeCount: number
  fallingEdgeCount: number
  trend?: number | null
  changeRate?: number | null
  triggerHits: string[]
}

export interface UiTriggerPayload {
  channelId: string
  ruleId: string
  severity: UiTriggerSeverity
  firedAtMs: number
  reason: string
  snapshot?: UiAnalysisPayload | null
}
