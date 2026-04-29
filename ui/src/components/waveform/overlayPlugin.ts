import type { MutableRefObject } from 'react'
import type uPlot from 'uplot'
import type { PlotModel, PlotTrack, OverlayState, OverlayItemElements, OverlayAnimationTarget, CursorAnimationState, LatestAnimationState } from './types'
import { SVG_NS, CURSOR_LABEL_GAP_PX, CURSOR_LABEL_MIN_CENTER_Y, CURSOR_LABEL_MAX_WIDTH_PX, CURSOR_CALLOUT_THRESHOLD_PX, CURSOR_SMOOTHING_X, CURSOR_SMOOTHING_Y, CURSOR_SNAP_DISTANCE_PX, CURSOR_ANIMATION_EPSILON_PX, LATEST_SMOOTHING_X, LATEST_SMOOTHING_Y, MAX_CURSOR_LABEL_TRACKS, MAX_PERSISTENT_LABEL_TRACKS, OVERLAY_MOTION_DEBUG_STORAGE_KEY } from './types'
import { formatDisplayValue } from './plotModel'

// ── Overlay plugin factory ──────────────────────────────────────────────────

export function createMeasurementOverlayPlugin(modelRef: MutableRefObject<PlotModel | null>): uPlot.Plugin {
  return {
    hooks: {
      init: [
        (u) => {
          const overlayRoot = document.createElement('div')
          overlayRoot.className = 'waveform-overlay'

          u.root.querySelectorAll<HTMLElement>('.u-cursor-pt').forEach((element) => {
            element.style.display = 'none'
          })

          const linesLayer = document.createElementNS(SVG_NS, 'svg')
          linesLayer.classList.add('waveform-overlay-lines')
          linesLayer.setAttribute('aria-hidden', 'true')

          const labelsLayer = document.createElement('div')
          labelsLayer.className = 'waveform-overlay-labels'

          const textTrackLayer = document.createElement('div')
          textTrackLayer.className = 'waveform-text-track'

          overlayRoot.append(linesLayer, labelsLayer, textTrackLayer)
          u.over.appendChild(overlayRoot)

          const state: OverlayState = {
            plot: u,
            overlayRoot,
            linesLayer,
            labelsLayer,
            textTrackLayer,
            items: new Map<string, OverlayItemElements>(),
            cursorAnimations: new Map<string, CursorAnimationState>(),
            cursorFrameId: null,
            latestAnimations: new Map<string, LatestAnimationState>(),
            latestFrameId: null,
            periodLines: new Map<string, SVGLineElement[]>(),
            textTrackSignature: '',
            debugLogIntervalId: null,
          }

          if (isOverlayMotionDebugEnabled()) {
            state.debugLogIntervalId = window.setInterval(() => {
              logOverlayMotionDebug(state)
            }, 500)
          }

            ; (u as uPlot & { __waveformOverlayState?: typeof state }).__waveformOverlayState = state
          syncOverlayItems(u)
        },
      ],
      setSize: [syncOverlaySize, syncOverlayItems, syncOverlayCursor],
      draw: [syncOverlayItems, syncOverlayCursor],
      setCursor: [syncOverlayCursor],
      destroy: [
        (u) => {
          const state = getOverlayState(u)
          if (state && state.cursorFrameId !== null) {
            cancelAnimationFrame(state.cursorFrameId)
          }
          if (state && state.latestFrameId !== null) {
            cancelAnimationFrame(state.latestFrameId)
          }
          if (state && state.debugLogIntervalId !== null) {
            window.clearInterval(state.debugLogIntervalId)
          }
          state?.overlayRoot.remove()
          delete (u as uPlot & { __waveformOverlayState?: unknown }).__waveformOverlayState
        },
      ],
    },
  }

  function syncOverlaySize(u: uPlot) {
    const state = getOverlayState(u)
    if (!state) {
      return null
    }

    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))
    setSvgAttributeIfChanged(state.linesLayer, 'viewBox', `0 0 ${width} ${height}`)
    setSvgAttributeIfChanged(state.linesLayer, 'width', `${width}`)
    setSvgAttributeIfChanged(state.linesLayer, 'height', `${height}`)
    setSvgAttributeIfChanged(state.linesLayer, 'preserveAspectRatio', 'none')
    return { width, height }
  }

  function syncOverlayItems(u: uPlot) {
    const state = getOverlayState(u)
    const model = modelRef.current
    if (!state || !model) {
      return
    }

    const overlaySize = syncOverlaySize(u)
    const width = overlaySize?.width ?? Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = overlaySize?.height ?? Math.max(0, Math.floor(u.over.getBoundingClientRect().height))

    const activeNumericTracks = model.numericTracks.filter(
      (track) => track.labelsEnabled || track.rangeEnabled || track.meanEnabled || track.medianEnabled || track.periodEnabled || track.slopeEnabled,
    )
    const activeTextTracks = model.textTracks.filter((track) => track.textEnabled)
    const latestLabelsEnabled = model.numericTracks.length <= MAX_PERSISTENT_LABEL_TRACKS
    const textTrackSignature = activeTextTracks.map((track) => `${track.name}\u0000${track.value}\u0000${track.color}`).join('|')

    const measurementSlotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []
    for (const track of activeNumericTracks) {
      if (track.meanEnabled) {
        measurementSlotInputs.push({ key: measurementSlotKey(track.name, 'mean'), baseY: safePos(u, track.stats.meanValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
      }
      if (track.medianEnabled) {
        measurementSlotInputs.push({ key: measurementSlotKey(track.name, 'median'), baseY: safePos(u, track.stats.medianValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
      }
    }
    const measurementSlots = placeVerticalLabels(measurementSlotInputs)

    const requiredKeys = new Set<string>()

    for (const track of activeNumericTracks) {
      requiredKeys.add(track.name)
      const elements = getOrCreateOverlayElements(state, track.name)
      const latestPoint = track.points[track.points.length - 1]
      const meanValue = track.stats.meanValue
      const medianValue = track.stats.medianValue
      const minValue = track.stats.minValue
      const maxValue = track.stats.maxValue

      const meanY = safePos(u, meanValue, 'y', height)
      const medianY = safePos(u, medianValue, 'y', height)
      const minY = safePos(u, minValue, 'y', height)
      const maxY = safePos(u, maxValue, 'y', height)

    if (track.rangeEnabled) {
      setLine(elements.minLine, 0, minY, width, minY, colorToRgba(track.color, 0.52), '5 6')
      setLine(elements.maxLine, 0, maxY, width, maxY, colorToRgba(track.color, 0.52), '5 6')
    } else {
      setSvgVisibilityIfChanged(elements.minLine, 'hidden')
      setSvgVisibilityIfChanged(elements.maxLine, 'hidden')
    }

      if (track.periodEnabled && track.period.periodMs !== null) {
        syncPeriodLines(state, u, track, width, minY, maxY, model.windowStartMs, model.windowEndMs)
      } else {
        hidePeriodLines(state.periodLines.get(track.name))
      }

      if (track.meanEnabled) {
        setLine(elements.meanLine, 0, meanY, width, meanY, colorToRgba(track.color, 0.64), '10 6', 1.8)
        updateMeasurementLabel(
          elements.meanLabel,
          width,
          measurementSlots.get(measurementSlotKey(track.name, 'mean')) ?? meanY,
          track.color,
          track.name,
          'AVG',
          formatDisplayValue(track.variable, meanValue),
          'mean',
        )
    } else {
      setSvgVisibilityIfChanged(elements.meanLine, 'hidden')
      setStyleIfChanged(elements.meanLabel.style, 'display', 'none')
    }

      if (track.medianEnabled) {
        setLine(elements.medianLine, 0, medianY, width, medianY, colorToRgba(track.color, 0.54), '3 5', 1.8)
        updateMeasurementLabel(
          elements.medianLabel,
          width,
          measurementSlots.get(measurementSlotKey(track.name, 'median')) ?? medianY,
          track.color,
          track.name,
          'MED',
          formatDisplayValue(track.variable, medianValue),
          'median',
        )
    } else {
      setSvgVisibilityIfChanged(elements.medianLine, 'hidden')
      setStyleIfChanged(elements.medianLabel.style, 'display', 'none')
    }

      setStyleIfChanged(elements.leftLabel.style, 'display', 'none')
      setSvgVisibilityIfChanged(elements.calloutLine, 'hidden')

      if (latestPoint) {
        const latestX = safePos(u, (latestPoint.timestampMs - model.windowStartMs) / 1000, 'x', width)
        const latestY = safePos(u, latestPoint.value, 'y', height)
        const latestLabelX = latestX > width - 170 ? Math.max(12, latestX - 150) : Math.min(width - 150, latestX + 12)
        const latestLabelY = latestY

        if (track.labelsEnabled && latestLabelsEnabled) {
          setStyleIfChanged(elements.latestLabel.style, 'display', 'flex')
          setStyleIfChanged(elements.latestLabel.style, 'borderColor', colorToRgba(track.color, 0.4))
          setStyleIfChanged(elements.latestLabel.style, 'background', colorToRgba(track.color, 0.14))
          setStyleIfChanged(elements.latestLabel.style, 'color', '#f3f7fc')
          setOverlayLabelContent(elements.latestLabel, track.color, track.name, track.displayValue)
          setStyleIfChanged(elements.latestLabel.style, 'left', `${latestLabelX}px`)
          setStyleIfChanged(elements.latestLabel.style, 'top', `${latestLabelY}px`)
          setStyleIfChanged(elements.latestLabel.style, 'transform', 'translateY(-50%)')
          setCircle(elements.latestDot, latestX, latestY, 3.5, track.color)
        } else {
          setCircleRadiusIfChanged(elements.latestDot, 0)
          setStyleIfChanged(elements.latestLabel.style, 'display', 'none')
        }
      } else {
        setCircleRadiusIfChanged(elements.latestDot, 0)
        setStyleIfChanged(elements.latestLabel.style, 'display', 'none')
      }

      if (track.slopeEnabled && track.stats.changeRate !== null && track.stats.changeRate !== undefined && latestPoint) {
        const latestX = (latestPoint.timestampMs - model.windowStartMs) / 1000
        const latestY = latestPoint.value
        const clipped = clipSlopeSegment(
          latestX,
          latestY,
          track.stats.changeRate,
          0,
          minValue,
          maxValue,
        )
        if (clipped) {
          setLine(elements.slopeLine, safePos(u, clipped.startX, 'x', width), safePos(u, clipped.startY, 'y', height), safePos(u, clipped.endX, 'x', width), safePos(u, clipped.endY, 'y', height), colorToRgba(track.color, 0.82), '4 4', 2.2)
        } else {
          setSvgVisibilityIfChanged(elements.slopeLine, 'hidden')
        }
      } else {
        setSvgVisibilityIfChanged(elements.slopeLine, 'hidden')
      }
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        setStyleIfChanged(element.leftLabel.style, 'display', 'none')
        setStyleIfChanged(element.latestLabel.style, 'display', 'none')
        setCircleRadiusIfChanged(element.latestDot, 0)
        setSvgVisibilityIfChanged(element.calloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setStyleIfChanged(element.meanLabel.style, 'display', 'none')
        setStyleIfChanged(element.medianLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.meanLine, 'hidden')
        setSvgVisibilityIfChanged(element.medianLine, 'hidden')
        setSvgVisibilityIfChanged(element.minLine, 'hidden')
        setSvgVisibilityIfChanged(element.maxLine, 'hidden')
        setSvgVisibilityIfChanged(element.slopeLine, 'hidden')
        hidePeriodLines(state.periodLines.get(name))
        state.latestAnimations.delete(name)
      }
    }

    if (state.textTrackSignature !== textTrackSignature) {
      state.textTrackSignature = textTrackSignature
      state.textTrackLayer.replaceChildren()
      for (const track of activeTextTracks) {
        const row = document.createElement('div')
        row.className = 'waveform-text-track-item'
        row.style.borderColor = colorToRgba(track.color, 0.35)
        row.style.background = colorToRgba(track.color, 0.12)
        row.innerHTML = `
          <span class="variable-color" style="background:${track.color}"></span>
          <div class="waveform-text-track-copy">
            <strong>${escapeHtml(track.name)}</strong>
            <small title="${escapeHtml(track.value)}">${escapeHtml(track.value)}</small>
          </div>
        `
        state.textTrackLayer.appendChild(row)
      }
    }
  }

  function syncOverlayCursor(u: uPlot) {
    const state = getOverlayState(u)
    const model = modelRef.current
    if (!state || !model) {
      return
    }

    const { left, top, idx } = u.cursor
    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))

    if (left == null || top == null || left < 0 || top < 0 || idx === null) {
      stopCursorAnimation(state)
      for (const element of state.items.values()) {
        setStyleIfChanged(element.cursorLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.cursorGuideLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorDot, 'hidden')
      }
      return
    }

    const activeNumericTracks = model.numericTracks.filter((track) => track.labelsEnabled || track.slopeEnabled)
    const cursorAnchors = activeNumericTracks.map((track) => {
      const anchor = resolveCursorAnchor(u, track, model.windowStartMs, width)
      const anchorY = safePos(u, anchor.value, 'y', height)
      return {
        track,
        anchor,
        distanceToCursor: Math.abs(anchorY - top),
      }
    })
    const visibleCursorAnchors =
      cursorAnchors.length > MAX_CURSOR_LABEL_TRACKS
        ? [...cursorAnchors].sort((left, right) => left.distanceToCursor - right.distanceToCursor).slice(0, MAX_CURSOR_LABEL_TRACKS)
        : cursorAnchors
    const requiredKeys = new Set<string>()
    for (const { track, anchor: cursorAnchor } of visibleCursorAnchors) {
      requiredKeys.add(track.name)
      const element = getOrCreateOverlayElements(state, track.name)
      const cursorLabelAnchorX = cursorAnchor.x + 14
      const cursorX = clamp(cursorLabelAnchorX, 12, Math.max(12, width - CURSOR_LABEL_MAX_WIDTH_PX - 12))

      setStyleIfChanged(element.cursorLabel.style, 'display', 'flex')
      setStyleIfChanged(element.cursorLabel.style, 'borderColor', colorToRgba(track.color, 0.42))
      setStyleIfChanged(element.cursorLabel.style, 'background', colorToRgba(track.color, 0.16))
      setStyleIfChanged(element.cursorLabel.style, 'color', '#f6f8fb')
      setOverlayLabelContent(element.cursorLabel, track.color, track.name, formatDisplayValue(track.variable, cursorAnchor.value))
      setLine(element.cursorGuideLine, cursorAnchor.x, 0, cursorAnchor.x, height, colorToRgba(track.color, 0.34), '5 5', 1)
      updateCursorAnimationTarget(state, track.name, {
        color: track.color,
        anchorX: cursorAnchor.x,
        labelX: cursorX,
        value: cursorAnchor.value,
      })
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        state.cursorAnimations.delete(name)
        state.latestAnimations.delete(name)
        setStyleIfChanged(element.cursorLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.cursorGuideLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorDot, 'hidden')
        setStyleIfChanged(element.latestLabel.style, 'display', 'none')
        setCircleRadiusIfChanged(element.latestDot, 0)
      }
    }

    ensureCursorAnimationFrame(state)
  }

  function getOverlayState(u: uPlot) {
    return (u as uPlot & { __waveformOverlayState?: OverlayState }).__waveformOverlayState ?? null
  }

  function syncPeriodLines(
    state: OverlayState,
    u: uPlot,
    track: PlotTrack,
    width: number,
    minY: number,
    maxY: number,
    windowStartMs: number,
    windowEndMs: number,
  ) {
    const lines = state.periodLines.get(track.name) ?? []
    const cycleStarts = track.period.cycleStartsMs
      .filter((timestampMs) => timestampMs >= windowStartMs && timestampMs <= windowEndMs)
      .sort((left, right) => left - right)

    for (let index = 0; index < cycleStarts.length; index += 1) {
      const timestampMs = cycleStarts[index]!
      const line = lines[index] ?? createSvgLine(state.linesLayer, 'waveform-overlay-period')
      lines[index] = line
      const x = safePos(u, (timestampMs - windowStartMs) / 1000, 'x', width)
      setLine(line, x, minY, x, maxY, colorToRgba(track.color, 0.42), '4 6', 1.4)
    }

    for (let index = cycleStarts.length; index < lines.length; index += 1) {
      lines[index]?.setAttribute('visibility', 'hidden')
    }

    state.periodLines.set(track.name, lines)
  }

  function hidePeriodLines(lines?: SVGLineElement[]) {
    if (!lines) {
      return
    }

    for (const line of lines) {
      line.setAttribute('visibility', 'hidden')
    }
  }

  function getOrCreateOverlayElements(state: OverlayState, name: string) {
    const existing = state.items.get(name)
    if (existing) {
      return existing
    }

    const leftLabel = document.createElement('div')
    leftLabel.className = 'waveform-overlay-label waveform-overlay-label--left'

    const latestLabel = document.createElement('div')
    latestLabel.className = 'waveform-overlay-label waveform-overlay-label--latest'

    const cursorLabel = document.createElement('div')
    cursorLabel.className = 'waveform-overlay-label waveform-overlay-label--cursor'

    const meanLabel = document.createElement('div')
    meanLabel.className = 'waveform-overlay-label waveform-overlay-label--measurement'

    const medianLabel = document.createElement('div')
    medianLabel.className = 'waveform-overlay-label waveform-overlay-label--measurement'

    const calloutLine = createSvgLine(state.linesLayer, 'waveform-overlay-callout')
    const cursorGuideLine = createSvgLine(state.linesLayer, 'waveform-overlay-cursor-guide')
    const cursorCalloutLine = createSvgLine(state.linesLayer, 'waveform-overlay-callout')
    const meanLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const medianLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const minLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const maxLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const slopeLine = createSvgLine(state.linesLayer, 'waveform-overlay-slope')
    const latestDot = createSvgCircle(state.linesLayer, 'waveform-overlay-latest-dot')
    const cursorDot = createSvgCircle(state.linesLayer, 'waveform-overlay-cursor-dot')

    const elements: OverlayItemElements = {
      leftLabel,
      latestLabel,
      cursorLabel,
      meanLabel,
      medianLabel,
      calloutLine,
      cursorGuideLine,
      cursorCalloutLine,
      meanLine,
      medianLine,
      minLine,
      maxLine,
      slopeLine,
      latestDot,
      cursorDot,
    }

    state.labelsLayer.append(leftLabel, latestLabel, cursorLabel, meanLabel, medianLabel)
    state.items.set(name, elements)
    return elements
  }

  function updateCursorAnimationTarget(state: OverlayState, name: string, next: OverlayAnimationTarget) {
    const current = state.cursorAnimations.get(name)
    if (!current) {
      state.cursorAnimations.set(name, {
        currentX: next.labelX,
        currentValue: next.value,
        target: next,
      })
      return
    }

    current.target = next
    if (Math.abs(current.currentX - next.labelX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentX = next.labelX
    }
    if (Math.abs(current.currentValue - next.value) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentValue = next.value
    }
  }

  function updateLatestAnimationTarget(state: OverlayState, name: string, next: OverlayAnimationTarget) {
    const current = state.latestAnimations.get(name)
    if (!current) {
      state.latestAnimations.set(name, {
        currentAnchorX: next.anchorX,
        currentX: next.labelX,
        currentLabelValue: next.value,
        target: next,
      })
      return
    }

    current.target = next
    if (Math.abs(current.currentAnchorX - next.anchorX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentAnchorX = next.anchorX
    }
    if (Math.abs(current.currentX - next.labelX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentX = next.labelX
    }
    if (Math.abs(current.currentLabelValue - next.value) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentLabelValue = next.value
    }
  }

  function ensureCursorAnimationFrame(state: OverlayState) {
    if (state.cursorFrameId !== null) {
      return
    }

    const tick = () => {
      state.cursorFrameId = null
      let needsNextFrame = false
      const height = Math.max(0, Math.floor(state.plot.over.getBoundingClientRect().height))
      const slotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []

      for (const [name, animation] of state.cursorAnimations) {
        const baseY = safePos(state.plot, animation.currentValue, 'y', height)
        slotInputs.push({
          key: name,
          baseY,
          minY: CURSOR_LABEL_MIN_CENTER_Y,
          maxY: Math.max(CURSOR_LABEL_MIN_CENTER_Y, height - CURSOR_LABEL_MIN_CENTER_Y),
        })
      }

      const cursorSlots = placeVerticalLabels(slotInputs, CURSOR_LABEL_GAP_PX)

      for (const [name, animation] of state.cursorAnimations) {
        const element = state.items.get(name)
        if (!element) {
          continue
        }

        const nextX = stepCursorValue(animation.currentX, animation.target.labelX, CURSOR_SMOOTHING_X)
        const nextValue = stepCursorValue(animation.currentValue, animation.target.value, CURSOR_SMOOTHING_Y)
        const settledX = Math.abs(nextX - animation.target.labelX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledValue = Math.abs(nextValue - animation.target.value) <= CURSOR_ANIMATION_EPSILON_PX

        animation.currentX = settledX ? animation.target.labelX : nextX
        animation.currentValue = settledValue ? animation.target.value : nextValue

        renderCursorAnimationFrame(state.plot, element, animation, cursorSlots.get(name))

        if (!settledX || !settledValue) {
          needsNextFrame = true
        }
      }

      if (needsNextFrame) {
        ensureCursorAnimationFrame(state)
      }
    }

    state.cursorFrameId = requestAnimationFrame(tick)
  }

  function ensureLatestAnimationFrame(state: OverlayState) {
    if (state.latestFrameId !== null) {
      return
    }

    const tick = () => {
      state.latestFrameId = null
      let needsNextFrame = false
      const height = Math.max(0, Math.floor(state.plot.over.getBoundingClientRect().height))
      const slotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []

      for (const [name, animation] of state.latestAnimations) {
        const baseY = safePos(state.plot, animation.currentLabelValue, 'y', height)
        slotInputs.push({
          key: name,
          baseY,
          minY: 10,
          maxY: Math.max(10, height - 22),
        })
      }

      const latestSlots = placeVerticalLabels(slotInputs)

      for (const [name, animation] of state.latestAnimations) {
        const element = state.items.get(name)
        if (!element) {
          continue
        }

        const nextAnchorX = stepCursorValue(animation.currentAnchorX, animation.target.anchorX, LATEST_SMOOTHING_X)
        const nextX = stepCursorValue(animation.currentX, animation.target.labelX, LATEST_SMOOTHING_X)
        const nextLabelValue = stepCursorValue(animation.currentLabelValue, animation.target.value, LATEST_SMOOTHING_Y)
        const settledAnchorX = Math.abs(nextAnchorX - animation.target.anchorX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledX = Math.abs(nextX - animation.target.labelX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledLabelValue = Math.abs(nextLabelValue - animation.target.value) <= CURSOR_ANIMATION_EPSILON_PX

        animation.currentAnchorX = settledAnchorX ? animation.target.anchorX : nextAnchorX
        animation.currentX = settledX ? animation.target.labelX : nextX
        animation.currentLabelValue = settledLabelValue ? animation.target.value : nextLabelValue

        renderLatestAnimationFrame(state.plot, element, animation, latestSlots.get(name))

        if (!settledAnchorX || !settledX || !settledLabelValue) {
          needsNextFrame = true
        }
      }

      if (needsNextFrame) {
        ensureLatestAnimationFrame(state)
      }
    }

    state.latestFrameId = requestAnimationFrame(tick)
  }

  function stopCursorAnimation(state: OverlayState) {
    if (state.cursorFrameId !== null) {
      cancelAnimationFrame(state.cursorFrameId)
      state.cursorFrameId = null
    }
  }

  function stopLatestAnimation(state: OverlayState) {
    if (state.latestFrameId !== null) {
      cancelAnimationFrame(state.latestFrameId)
      state.latestFrameId = null
    }
  }

  function renderCursorAnimationFrame(plot: uPlot, element: OverlayItemElements, animation: CursorAnimationState, slotY?: number) {
    const { target } = animation
    const height = Math.max(0, Math.floor(plot.over.getBoundingClientRect().height))
    const anchorY = safePos(plot, target.value, 'y', height)
    const labelY = slotY ?? safePos(plot, target.value, 'y', height)
    const shouldShowCallout =
      Math.abs(labelY - anchorY) > CURSOR_CALLOUT_THRESHOLD_PX ||
      Math.abs(animation.currentX - (target.anchorX + 14)) > CURSOR_CALLOUT_THRESHOLD_PX

    setStyleIfChanged(element.cursorLabel.style, 'left', `${animation.currentX}px`)
    setStyleIfChanged(element.cursorLabel.style, 'top', `${labelY}px`)
    setStyleIfChanged(element.cursorLabel.style, 'transform', 'translateY(-50%)')
    setLine(element.cursorGuideLine, target.anchorX, 0, target.anchorX, height, colorToRgba(target.color, 0.34), '5 5', 1)
    setCircle(element.cursorDot, target.anchorX, anchorY, 4.5, target.color)

    if (shouldShowCallout) {
      setLine(element.cursorCalloutLine, target.anchorX, anchorY, animation.currentX - 8, labelY, colorToRgba(target.color, 0.82), '4 4', 1.8)
    } else {
      element.cursorCalloutLine.setAttribute('visibility', 'hidden')
    }
  }

  function renderLatestAnimationFrame(plot: uPlot, element: OverlayItemElements, animation: LatestAnimationState, slotY?: number) {
    const height = Math.max(0, Math.floor(plot.over.getBoundingClientRect().height))
    const anchorY = safePos(plot, animation.target.value, 'y', height)
    const labelY = slotY ?? safePos(plot, animation.currentLabelValue, 'y', height)
    setSvgAttributeIfChanged(element.latestDot, 'cx', `${animation.currentAnchorX}`)
    setSvgAttributeIfChanged(element.latestDot, 'cy', `${anchorY}`)
    setCircleRadiusIfChanged(element.latestDot, 3.5)
    setSvgAttributeIfChanged(element.latestDot, 'fill', animation.target.color)
    setSvgVisibilityIfChanged(element.latestDot, 'visible')

    setStyleIfChanged(element.latestLabel.style, 'left', `${animation.currentX}px`)
    setStyleIfChanged(element.latestLabel.style, 'top', `${labelY}px`)
    setStyleIfChanged(element.latestLabel.style, 'transform', 'translateY(-50%)')
  }

  function stepCursorValue(current: number, target: number, smoothing: number) {
    return current + (target - current) * smoothing
  }

  function logOverlayMotionDebug(state: OverlayState) {
    const latestEntry = state.latestAnimations.entries().next().value as [string, LatestAnimationState] | undefined
    const cursorEntry = state.cursorAnimations.entries().next().value as [string, CursorAnimationState] | undefined

    if (!latestEntry && !cursorEntry) {
      return
    }

    const payload: Record<string, unknown> = {}

    if (latestEntry) {
      const [name, animation] = latestEntry
      payload.latest = {
        name,
        currentAnchorX: roundDebug(animation.currentAnchorX),
        targetAnchorX: roundDebug(animation.target.anchorX),
        dAnchorX: roundDebug(animation.target.anchorX - animation.currentAnchorX),
        currentX: roundDebug(animation.currentX),
        targetX: roundDebug(animation.target.labelX),
        dx: roundDebug(animation.target.labelX - animation.currentX),
        currentValue: roundDebug(animation.currentLabelValue),
        targetValue: roundDebug(animation.target.value),
        dValue: roundDebug(animation.target.value - animation.currentLabelValue),
      }
    }

    if (cursorEntry) {
      const [name, animation] = cursorEntry
      payload.cursor = {
        name,
        currentX: roundDebug(animation.currentX),
        targetX: roundDebug(animation.target.labelX),
        dx: roundDebug(animation.target.labelX - animation.currentX),
        currentValue: roundDebug(animation.currentValue),
        targetValue: roundDebug(animation.target.value),
        dValue: roundDebug(animation.target.value - animation.currentValue),
      }
    }

    console.info(`[waveform-overlay-motion] ${formatDebugPayload(payload)}`)
  }

  function roundDebug(value: number) {
    return Math.round(value * 100) / 100
  }

  function formatDebugPayload(payload: Record<string, unknown>) {
    const latest = formatDebugEntry('latest', payload.latest)
    const cursor = formatDebugEntry('cursor', payload.cursor)
    return [latest, cursor].filter(Boolean).join(' | ')
  }

  function formatDebugEntry(label: string, value: unknown) {
    if (!value || typeof value !== 'object') {
      return ''
    }

    const entry = value as Record<string, unknown>
    return `${label}(name=${entry.name}, currentAnchorX=${entry.currentAnchorX ?? '-'}, targetAnchorX=${entry.targetAnchorX ?? '-'}, dAnchorX=${entry.dAnchorX ?? '-'}, currentX=${entry.currentX}, targetX=${entry.targetX}, dx=${entry.dx}, currentValue=${entry.currentValue}, targetValue=${entry.targetValue}, dValue=${entry.dValue})`
  }

  function resolveCursorAnchor(u: uPlot, track: PlotTrack, windowStartMs: number, width: number) {
    const cursorIndex = u.cursor.idx ?? 0
    const cursorTime = u.data[0]?.[cursorIndex]
    const cursorX = Number.isFinite(cursorTime) ? safePos(u, Number(cursorTime), 'x', width) : u.cursor.left ?? 0

    if (track.slopeEnabled && !track.labelsEnabled) {
      const slopeValue = getSlopeCursorValue(track, cursorTime, windowStartMs)
      if (slopeValue !== null) {
        return {
          value: slopeValue,
          x: cursorX,
        }
      }
    }

    return {
      value: getCursorValue(u, track),
      x: cursorX,
    }
  }

  function getSlopeCursorValue(track: PlotTrack, cursorTimeSeconds: unknown, windowStartMs: number) {
    if (typeof cursorTimeSeconds !== 'number' || !Number.isFinite(cursorTimeSeconds)) {
      return null
    }

    const latestPoint = track.points[track.points.length - 1]
    const rate = track.stats.changeRate
    if (!latestPoint || rate === null || rate === undefined || !Number.isFinite(rate)) {
      return null
    }

    const anchorX = (latestPoint.timestampMs - windowStartMs) / 1000
    const clipped = clipSlopeSegment(anchorX, latestPoint.value, rate, 0, track.stats.minValue, track.stats.maxValue)
    if (!clipped) {
      return null
    }

    if (cursorTimeSeconds < clipped.startX || cursorTimeSeconds > clipped.endX) {
      return null
    }

    return latestPoint.value + rate * (cursorTimeSeconds - anchorX)
  }

  function placeVerticalLabels(items: Array<{ key: string; baseY: number; minY: number; maxY: number }>, gap = 18) {
    const sorted = [...items].sort((left, right) => left.baseY - right.baseY)
    const slots = new Map<string, number>()

    if (sorted.length === 0) {
      return slots
    }

    const positions = sorted.map((item) => clamp(item.baseY, item.minY, item.maxY))

    for (let index = 1; index < sorted.length; index += 1) {
      positions[index] = clamp(Math.max(positions[index], positions[index - 1] + gap), sorted[index].minY, sorted[index].maxY)
    }

    for (let index = sorted.length - 2; index >= 0; index -= 1) {
      positions[index] = clamp(Math.min(positions[index], positions[index + 1] - gap), sorted[index].minY, sorted[index].maxY)
    }

    for (let index = 1; index < sorted.length; index += 1) {
      if (positions[index] - positions[index - 1] < gap) {
        positions[index] = clamp(positions[index - 1] + gap, sorted[index].minY, sorted[index].maxY)
      }
    }

    for (let index = 0; index < sorted.length; index += 1) {
      slots.set(sorted[index].key, positions[index])
    }

    return slots
  }

  function measurementSlotKey(name: string, kind: 'mean' | 'median') {
    return `${name}:${kind}`
  }

  function updateMeasurementLabel(
    label: HTMLDivElement,
    width: number,
    y: number,
    color: string,
    name: string,
    title: string,
    value: string,
    lane: 'mean' | 'median',
  ) {
    const labelWidth = 180
    const gutter = 10
    const rightLaneLeft = Math.max(12, width - labelWidth)
    const leftLaneLeft = Math.max(12, width - labelWidth * 2 - gutter)

    label.style.display = 'flex'
    setStyleIfChanged(label.style, 'left', `${lane === 'mean' ? leftLaneLeft : rightLaneLeft}px`)
    setStyleIfChanged(label.style, 'top', `${y}px`)
    setStyleIfChanged(label.style, 'transform', 'translateY(-50%)')
    setStyleIfChanged(label.style, 'borderColor', colorToRgba(color, 0.36))
    setStyleIfChanged(label.style, 'background', colorToRgba(color, 0.12))
    setStyleIfChanged(label.style, 'color', '#eaf0f7')
    setOverlayLabelContent(label, color, name, `${title} ${value}`)
  }

  function setOverlayLabelContent(label: HTMLDivElement, color: string, name: string, value: string) {
    const signature = `${color}\u0000${name}\u0000${value}`
    if (label.dataset.overlaySignature === signature) {
      return
    }

    label.dataset.overlaySignature = signature
    label.innerHTML = renderOverlayValueLabel(color, name, value)
  }

  function setLine(line: SVGLineElement, x1: number, y1: number, x2: number, y2: number, stroke: string, dash?: string, width = 1.6) {
    line.setAttribute('x1', `${x1}`)
    line.setAttribute('y1', `${y1}`)
    line.setAttribute('x2', `${x2}`)
    line.setAttribute('y2', `${y2}`)
    line.setAttribute('stroke', stroke)
    line.setAttribute('stroke-width', `${width}`)
    line.setAttribute('stroke-linecap', 'round')
    line.setAttribute('visibility', 'visible')
    if (dash) {
      line.setAttribute('stroke-dasharray', dash)
    } else {
      line.removeAttribute('stroke-dasharray')
    }
  }

  function setCircle(circle: SVGCircleElement, cx: number, cy: number, r: number, color: string) {
    circle.setAttribute('cx', `${cx}`)
    circle.setAttribute('cy', `${cy}`)
    circle.setAttribute('r', `${r}`)
    circle.setAttribute('fill', color)
    circle.setAttribute('visibility', 'visible')
  }

  function safePos(u: uPlot, value: number, axis: 'x' | 'y', fallback: number) {
    if (!Number.isFinite(value)) {
      return fallback / 2
    }

    const pos = u.valToPos(value, axis)
    return Number.isFinite(pos) ? pos : fallback / 2
  }

  function getLatestValue(track: PlotTrack) {
    return track.points[track.points.length - 1]?.value ?? track.variable.numericValue ?? track.stats.meanValue
  }

  function getCursorValue(u: uPlot, track: PlotTrack) {
    const dataValue = u.data[track.seriesIndex]?.[u.cursor.idx ?? 0]
    if (typeof dataValue === 'number' && Number.isFinite(dataValue)) {
      return dataValue
    }
    return getLatestValue(track)
  }

  function clipSlopeSegment(anchorX: number, anchorY: number, rate: number, xMin: number, yMin: number, yMax: number) {
    if (!Number.isFinite(rate)) {
      return null
    }

    if (!Number.isFinite(xMin) || !Number.isFinite(anchorX) || anchorX < xMin || !Number.isFinite(yMin) || !Number.isFinite(yMax) || yMax < yMin) {
      return null
    }

    if (anchorY < yMin || anchorY > yMax) {
      return null
    }

    if (Math.abs(rate) < 1e-8) {
      return {
        startX: xMin,
        startY: anchorY,
        endX: anchorX,
        endY: anchorY,
      }
    }

    const yBoundary = rate > 0 ? yMin : yMax
    const xAtYBoundary = anchorX + (yBoundary - anchorY) / rate
    const xAtLeftBoundary = xMin
    const yAtLeftBoundary = anchorY + rate * (xAtLeftBoundary - anchorX)

    let startX = xAtLeftBoundary
    let startY = yAtLeftBoundary

    if (Number.isFinite(xAtYBoundary) && xAtYBoundary >= xMin && xAtYBoundary <= anchorX) {
      startX = xAtYBoundary
      startY = yBoundary
    } else if (yAtLeftBoundary < yMin || yAtLeftBoundary > yMax) {
      return null
    }

    return {
      startX,
      startY,
      endX: anchorX,
      endY: anchorY,
    }
  }

  function createSvgLine(parent: SVGSVGElement, className: string) {
    const line = document.createElementNS(SVG_NS, 'line')
    line.classList.add(className)
    parent.appendChild(line)
    return line
  }

  function createSvgCircle(parent: SVGSVGElement, className: string) {
    const circle = document.createElementNS(SVG_NS, 'circle')
    circle.classList.add(className)
    parent.appendChild(circle)
    return circle
  }

  function setStyleIfChanged(style: CSSStyleDeclaration, property: string, value: string) {
    const currentValue = (style as CSSStyleDeclaration & Record<string, string>)[property]
    if (currentValue === value) {
      return
    }

    ;(style as CSSStyleDeclaration & Record<string, string>)[property] = value
  }

  function setSvgAttributeIfChanged(element: SVGElement, name: string, value: string) {
    if (element.getAttribute(name) === value) {
      return
    }

    element.setAttribute(name, value)
  }

  function setSvgVisibilityIfChanged(element: SVGElement, value: 'hidden' | 'visible') {
    setSvgAttributeIfChanged(element, 'visibility', value)
  }

  function setCircleRadiusIfChanged(circle: SVGCircleElement, value: number) {
    const radius = `${value}`
    if (circle.getAttribute('r') === radius) {
      return
    }

    circle.setAttribute('r', radius)
  }

  function renderOverlayValueLabel(color: string, name: string, value: string) {
    return `
      <span class="waveform-overlay-label-chip" style="background:${color}"></span>
      <span class="waveform-overlay-label-copy">
        <strong>${escapeHtml(name)}</strong>
        <span class="waveform-overlay-label-text">${escapeHtml(value)}</span>
      </span>
    `
  }

  function escapeHtml(value: string) {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\"/g, '&quot;')
      .replace(/'/g, '&#39;')
  }

  function colorToRgba(color: string, alpha: number) {
    if (!color.startsWith('#')) {
      return color
    }

    const hex = color.slice(1)
    const normalized = hex.length === 3 ? hex.split('').map((char) => `${char}${char}`).join('') : hex
    if (normalized.length !== 6) {
      return color
    }

    const value = Number.parseInt(normalized, 16)
    const red = (value >> 16) & 255
    const green = (value >> 8) & 255
    const blue = value & 255
    return `rgba(${red}, ${green}, ${blue}, ${alpha})`
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value))
  }

  function isOverlayMotionDebugEnabled() {
    if (typeof window === 'undefined') {
      return false
    }

    try {
      const value = window.localStorage.getItem(OVERLAY_MOTION_DEBUG_STORAGE_KEY)
      return value === '1' || value === 'true'
    } catch {
      return false
    }
  }
}
