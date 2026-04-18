export interface LogicAnalyzerCursorLaneRect {
  left: number
  top: number
  width: number
  height: number
}

export interface LogicAnalyzerCursorSource {
  key: string
  label: string
  anchorX: number
  anchorY: number
  laneRect: LogicAnalyzerCursorLaneRect
  sampleText: string
  accentColor: string
}

export interface LogicAnalyzerCursorOverlayItem {
  key: string
  label: string
  accentColor: string
  anchorX: number
  anchorY: number
  laneRect: LogicAnalyzerCursorLaneRect
  preferredLeft: number
  labelLeft: number
  labelTop: number
  labelWidth: number
  labelHeight: number
  labelCenterY: number
  calloutEndX: number
  calloutEndY: number
  showCallout: boolean
  side: 'left' | 'right'
}

const CURSOR_LABEL_MIN_WIDTH = 128
const CURSOR_LABEL_MAX_WIDTH = 208
const CURSOR_LABEL_HEIGHT = 22
const CURSOR_LABEL_EDGE_PADDING = 8
const CURSOR_LABEL_SIDE_GAP = 12
const CURSOR_CALLOUT_THRESHOLD_PX = 6

export function buildLogicAnalyzerCursorOverlayItems(hostSize: { width: number; height: number }, cursors: LogicAnalyzerCursorSource[]) {
  if (hostSize.width <= 0 || hostSize.height <= 0 || cursors.length === 0) {
    return [] as LogicAnalyzerCursorOverlayItem[]
  }

  const prepared = cursors.map((cursor) => {
    const labelWidth = estimateCursorLabelWidth(cursor.label, cursor.sampleText)
    const side = chooseCursorLabelSide(hostSize.width, cursor.anchorX, labelWidth)
    const preferredLeft =
      side === 'right' ? cursor.anchorX + CURSOR_LABEL_SIDE_GAP : cursor.anchorX - CURSOR_LABEL_SIDE_GAP - labelWidth
    const maxLeft = Math.max(CURSOR_LABEL_EDGE_PADDING, hostSize.width - labelWidth - CURSOR_LABEL_EDGE_PADDING)

    return {
      ...cursor,
      labelWidth,
      preferredLeft,
      labelLeft: clamp(preferredLeft, CURSOR_LABEL_EDGE_PADDING, maxLeft),
      labelHeight: CURSOR_LABEL_HEIGHT,
      side,
    }
  })

  const centers = placeVerticalLabels(
    prepared.map((item) => ({
      key: item.key,
      baseCenterY: item.anchorY,
      minCenterY: CURSOR_LABEL_EDGE_PADDING + item.labelHeight / 2,
      maxCenterY: Math.max(CURSOR_LABEL_EDGE_PADDING + item.labelHeight / 2, hostSize.height - CURSOR_LABEL_EDGE_PADDING - item.labelHeight / 2),
      height: item.labelHeight,
    })),
    6,
  )

  return prepared.map((item) => {
    const labelCenterY = centers.get(item.key) ?? item.anchorY
    const labelTop = clamp(
      labelCenterY - item.labelHeight / 2,
      CURSOR_LABEL_EDGE_PADDING,
      Math.max(CURSOR_LABEL_EDGE_PADDING, hostSize.height - item.labelHeight - CURSOR_LABEL_EDGE_PADDING),
    )
    const calloutEndX = item.side === 'right' ? item.labelLeft : item.labelLeft + item.labelWidth
    const calloutEndY = labelCenterY
    const horizontalDisplacement = Math.abs(item.labelLeft - item.preferredLeft)
    const showCallout =
      Math.abs(labelCenterY - item.anchorY) > CURSOR_CALLOUT_THRESHOLD_PX ||
      horizontalDisplacement > CURSOR_CALLOUT_THRESHOLD_PX

    return {
      key: item.key,
      label: item.label,
      accentColor: item.accentColor,
      anchorX: item.anchorX,
      anchorY: item.anchorY,
      laneRect: item.laneRect,
      labelLeft: item.labelLeft,
      preferredLeft: item.preferredLeft,
      labelTop,
      labelWidth: item.labelWidth,
      labelHeight: item.labelHeight,
      labelCenterY,
      calloutEndX,
      calloutEndY,
      showCallout,
      side: item.side,
    }
  })
}

export function placeVerticalLabels(
  items: Array<{ key: string; baseCenterY: number; minCenterY: number; maxCenterY: number; height: number }>,
  gapPx: number,
) {
  const sorted = [...items].sort((left, right) => left.baseCenterY - right.baseCenterY || left.key.localeCompare(right.key))
  const centers = new Map<string, number>()
  let previousCenter = Number.NEGATIVE_INFINITY
  let previousHeight = 0

  for (const item of sorted) {
    const minimumCenter = previousCenter === Number.NEGATIVE_INFINITY ? item.minCenterY : previousCenter + (previousHeight + item.height) / 2 + gapPx
    const nextCenter = clamp(Math.max(item.baseCenterY, minimumCenter), item.minCenterY, item.maxCenterY)
    centers.set(item.key, nextCenter)
    previousCenter = nextCenter
    previousHeight = item.height
  }

  const values = sorted.map((item) => centers.get(item.key) ?? item.baseCenterY)
  const lastIndex = values.length - 1
  if (lastIndex >= 0) {
    const overflow = values[lastIndex]! + sorted[lastIndex]!.height / 2 - sorted[lastIndex]!.maxCenterY
    if (overflow > 0) {
      for (const item of sorted) {
        centers.set(item.key, (centers.get(item.key) ?? item.baseCenterY) - overflow)
      }
    }
    const firstOverflow = sorted[0]!.minCenterY - (centers.get(sorted[0]!.key) ?? sorted[0]!.baseCenterY)
    if (firstOverflow > 0) {
      for (const item of sorted) {
        centers.set(item.key, (centers.get(item.key) ?? item.baseCenterY) + firstOverflow)
      }
    }
  }

  return centers
}

function chooseCursorLabelSide(hostWidth: number, anchorX: number, labelWidth: number) {
  const rightSpace = hostWidth - anchorX - CURSOR_LABEL_SIDE_GAP - labelWidth - CURSOR_LABEL_EDGE_PADDING
  const leftSpace = anchorX - CURSOR_LABEL_SIDE_GAP - labelWidth - CURSOR_LABEL_EDGE_PADDING

  if (rightSpace >= leftSpace) {
    return 'right' as const
  }

  return 'left' as const
}

function estimateCursorLabelWidth(label: string, sampleText: string) {
  const textLength = `${label} ${sampleText}`.length
  return clamp(Math.ceil(textLength * 7.2) + 20, CURSOR_LABEL_MIN_WIDTH, CURSOR_LABEL_MAX_WIDTH)
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}
