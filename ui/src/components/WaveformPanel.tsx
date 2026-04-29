import { useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import uPlot from 'uplot'
import { formatWaveformWindowMs, MAX_WAVEFORM_WINDOW_MS, MIN_WAVEFORM_WINDOW_MS, scaleWaveformWindowMs } from '../lib/waveformWindow'
import { useAppStore } from '../store/appStore'
import type { SelectedWaveformVariable } from '../store/selectors/waveformSelectors'
import type { YScaleAnimationState } from './waveform/types'
import { buildPlotModel, isNumericVariable } from './waveform/plotModel'
import { createMeasurementOverlayPlugin } from './waveform/overlayPlugin'
import { MagnifyPlusIcon, MagnifyMinusIcon } from './waveform/icons'
import type { PlotModel } from './waveform/types'

export function WaveformPanel() {
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const variables = useAppStore((state) => state.variables)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const visualAidState = useAppStore((state) => state.visualAidState)
  const timeWindowMs = useAppStore((state) => state.waveformWindowMs)
  const setTimeWindowMs = useAppStore((state) => state.setWaveformWindowMs)
  const [menuOpen, setMenuOpen] = useState(true)

  const selectedVariables = useMemo(
    () =>
      selectedChannels.reduce<SelectedWaveformVariable[]>((acc, channel) => {
        const variable = variables[channel]
        if (!variable) {
          return acc
        }

        acc.push({
          name: variable.name,
          color: colorForChannel(variable.name),
          variable,
        })
        return acc
      }, []),
    [colorForChannel, selectedChannels, variables],
  )

  const { numericCount, textCount } = useMemo(() => {
    let numericCount = 0
    for (const item of selectedVariables) {
      if (isNumericVariable(item.variable)) {
        numericCount++
      }
    }

    return {
      numericCount,
      textCount: selectedVariables.length - numericCount,
    }
  }, [selectedVariables])

  return (
    <section className="panel waveform-panel">
      <WavePlot selectedVariables={selectedVariables} visualAidState={visualAidState} timeWindowMs={timeWindowMs} onTimeWindowChange={setTimeWindowMs} />

      <div className="waveform-stage-hud">
        <div className="waveform-stage-meta">
          <strong>Waveforms</strong>
          <span>{selectedVariables.length > 0 ? `${numericCount} waveforms · ${textCount} text tracks` : 'Idle'}</span>
          <small>{formatTimeWindow(timeWindowMs)}</small>
        </div>
      </div>

      <div className={`waveform-floating-menu ${menuOpen ? '' : 'collapsed'}`}>
        <div className="waveform-floating-top">
          {menuOpen ? (
            <div className="waveform-floating-heading">
              <strong>Waveform Controls</strong>
            </div>
          ) : null}
          <button type="button" className="ghost-button waveform-floating-toggle" onClick={() => setMenuOpen((value) => !value)}>
            {menuOpen ? 'Hide' : 'Waveforms'}
          </button>
        </div>

        {menuOpen ? (
          <div className="waveform-floating-scroll">
            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Window</strong>
              </div>
              <div className="waveform-controls">
                <button
                  type="button"
                  className="ghost-button waveform-zoom-icon-button"
                  onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 0.8))}
                  aria-label="Zoom in"
                  title="Zoom in"
                >
                  <MagnifyPlusIcon />
                </button>
                <label className="waveform-zoom-label">
                  <span>Visible span</span>
                  <input
                    type="range"
                    min={MIN_WAVEFORM_WINDOW_MS}
                    max={MAX_WAVEFORM_WINDOW_MS}
                    step={1_000}
                    value={timeWindowMs}
                    onChange={(event) => setTimeWindowMs(Number(event.target.value))}
                  />
                </label>
                <button
                  type="button"
                  className="ghost-button waveform-zoom-icon-button"
                  onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 1.25))}
                  aria-label="Zoom out"
                  title="Zoom out"
                >
                  <MagnifyMinusIcon />
                </button>
              </div>
            </section>
            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Overlays</strong>
              </div>
            </section>
          </div>
        ) : null}
      </div>
    </section>
  )
}

function WavePlot({
  selectedVariables,
  visualAidState,
  timeWindowMs,
  onTimeWindowChange,
}: {
  selectedVariables: SelectedWaveformVariable[]
  visualAidState: import('../lib/waveformVisualAids').WaveformVisualAidState
  timeWindowMs: number
  onTimeWindowChange: (value: number | ((current: number) => number)) => void
}) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const structureKeyRef = useRef('')
  const modelRef = useRef<PlotModel | null>(null)
  const yScaleAnimationRef = useRef<YScaleAnimationState | null>(null)
  const [size, setSize] = useState({ width: 900, height: 440 })
  const model = useMemo(() => buildPlotModel(selectedVariables, visualAidState, timeWindowMs), [selectedVariables, timeWindowMs, visualAidState])
  const structureKey = useMemo(() => (model.numericTracks.length === 0 ? '__empty__' : model.numericTracks.map((item) => item.name).join('|')), [model.numericTracks])

  modelRef.current = model

  useEffect(() => {
    if (!hostRef.current) {
      return
    }

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0]
      if (!entry) {
        return
      }
      setSize({
        width: Math.max(480, Math.floor(entry.contentRect.width)),
        height: Math.max(320, Math.floor(entry.contentRect.height)),
      })
    })
    observer.observe(hostRef.current)

    const wheelTarget = hostRef.current
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      onTimeWindowChange((value) => scaleTimeWindow(value, event.deltaY < 0 ? 0.88 : 1.12))
    }
    wheelTarget.addEventListener('wheel', onWheel, { passive: false })

    return () => {
      observer.disconnect()
      wheelTarget.removeEventListener('wheel', onWheel)
    }
  }, [onTimeWindowChange])

  useEffect(() => {
    if (!hostRef.current) {
      return
    }

    if (!plotRef.current || structureKeyRef.current !== structureKey) {
      plotRef.current?.destroy()
      plotRef.current = new uPlot(
        {
          width: size.width,
          height: size.height,
          padding: [12, 16, 12, 16],
          scales: {
            x: { time: false, auto: false },
            y: { auto: false },
          },
          axes: [
            {
              stroke: '#5f6b7a',
              grid: { stroke: '#20242d' },
              values: (_, ticks) => {
                const visibleSpanSeconds = Math.max(modelRef.current?.xMax ?? model.xMax, 0)
                return ticks.map((tick) => {
                  const elapsedSeconds = Math.max(0, visibleSpanSeconds - tick)
                  return `${elapsedSeconds.toFixed(elapsedSeconds >= 10 ? 0 : 1)}s`
                })
              },
            },
            { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
          ],
          series: model.series,
          plugins: [createMeasurementOverlayPlugin(modelRef)],
        },
        model.data,
        hostRef.current,
      )
      yScaleAnimationRef.current = {
        currentMin: model.yMin,
        currentMax: model.yMax,
        targetMin: model.yMin,
        targetMax: model.yMax,
        frameId: null,
      }
      structureKeyRef.current = structureKey
    }

    return () => {
      if (!hostRef.current?.isConnected) {
        stopYScaleAnimation(yScaleAnimationRef.current)
        plotRef.current?.destroy()
        plotRef.current = null
        yScaleAnimationRef.current = null
        structureKeyRef.current = ''
      }
    }
  }, [model.data, model.series, size, structureKey])

  useEffect(() => {
    plotRef.current?.setSize(size)
  }, [size])

  useEffect(() => {
    if (!plotRef.current) {
      return
    }

    plotRef.current.setData(model.data)
    plotRef.current.setScale('x', { min: model.xMin, max: model.xMax })
    plotRef.current.setScale('y', { min: model.yMin, max: model.yMax })
  }, [model.data, model.xMin, model.xMax, model.yMin, model.yMax])

  useEffect(() => {
    plotRef.current?.redraw()
  }, [visualAidState])

  useEffect(
    () => () => {
      stopYScaleAnimation(yScaleAnimationRef.current)
      plotRef.current?.destroy()
      plotRef.current = null
      yScaleAnimationRef.current = null
      structureKeyRef.current = ''
    },
    [],
  )

  return <div className="wave-plot waveform-stage-surface" ref={hostRef} />
}

// ── Local helpers ───────────────────────────────────────────────────────────

function scaleTimeWindow(current: number, factor: number) {
  return clampTimeWindow(Math.round(current * factor))
}

function clampTimeWindow(value: number) {
  return Math.min(MAX_WAVEFORM_WINDOW_MS, Math.max(MIN_WAVEFORM_WINDOW_MS, value))
}

function stopYScaleAnimation(_state: YScaleAnimationState | null) {
  return
}

function formatTimeWindow(timeWindowMs: number) {
  return formatWaveformWindowMs(timeWindowMs)
}
