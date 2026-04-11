import { useEffect, useMemo, useRef, useState } from 'react'
import uPlot from 'uplot'
import { useAppStore } from '../store/appStore'

const MIN_TIME_WINDOW_MS = 2_000
const MAX_TIME_WINDOW_MS = 120_000
const DEFAULT_TIME_WINDOW_MS = 15_000

type PlotSeries = {
  name: string
  color: string
  points: Array<{ timestampMs: number; value: number }>
}

export function WaveformPanel() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const toggleChannel = useAppStore((state) => state.toggleChannel)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const [timeWindowMs, setTimeWindowMs] = useState(DEFAULT_TIME_WINDOW_MS)
  const series = useMemo<PlotSeries[]>(
    () =>
      selectedChannels
        .map((channel) => variables[channel])
        .filter(Boolean)
        .map((variable) => ({
          name: variable.name,
          color: colorForChannel(variable.name),
          points: variable.points,
        })),
    [colorForChannel, selectedChannels, variables],
  )

  return (
    <section className="panel waveform-panel">
      <div className="toolbar-row waveform-toolbar">
        <div className="toolbar-title-group">
          <strong>Waveforms</strong>
          <small>{series.length > 0 ? 'Drag panels to reshape your workspace.' : 'Select variables to plot.'}</small>
        </div>
        <div className="waveform-controls">
          <button className="ghost-button" onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 0.8))}>
            Zoom In
          </button>
          <label className="waveform-zoom-label">
            <span>Window {formatTimeWindow(timeWindowMs)}</span>
            <input
              type="range"
              min={MIN_TIME_WINDOW_MS}
              max={MAX_TIME_WINDOW_MS}
              step={1_000}
              value={timeWindowMs}
              onChange={(event) => setTimeWindowMs(Number(event.target.value))}
            />
          </label>
          <button className="ghost-button" onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 1.25))}>
            Zoom Out
          </button>
        </div>
        <div className="chip-row">
          {Object.keys(variables).map((channel) => {
            const selected = selectedChannels.includes(channel)
            return (
              <button
                key={channel}
                className={`channel-chip ${selected ? 'active' : ''}`}
                onClick={() => toggleChannel(channel)}
              >
                <span className="variable-color" style={{ backgroundColor: colorForChannel(channel) }} />
                {channel}
              </button>
            )
          })}
        </div>
      </div>
      <WavePlot series={series} timeWindowMs={timeWindowMs} onTimeWindowChange={setTimeWindowMs} />
    </section>
  )
}

function WavePlot({
  series,
  timeWindowMs,
  onTimeWindowChange,
}: {
  series: PlotSeries[]
  timeWindowMs: number
  onTimeWindowChange: (value: number | ((current: number) => number)) => void
}) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const structureKeyRef = useRef('')
  const [size, setSize] = useState({ width: 900, height: 440 })
  const model = useMemo(() => buildPlotModel(series, timeWindowMs), [series, timeWindowMs])
  const structureKey = useMemo(
    () => (series.length === 0 ? '__empty__' : series.map((item) => item.name).join('|')),
    [series],
  )

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
          scales: { x: { time: false } },
          axes: [
            {
              stroke: '#5f6b7a',
              grid: { stroke: '#20242d' },
              values: (_, ticks) => ticks.map((tick) => `${tick.toFixed(tick >= 10 ? 0 : 1)}s`),
            },
            { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
          ],
          series: model.series,
        },
        model.data,
        hostRef.current,
      )
      structureKeyRef.current = structureKey
    }

    return () => {
      if (!hostRef.current?.isConnected) {
        plotRef.current?.destroy()
        plotRef.current = null
        structureKeyRef.current = ''
      }
    }
}, [model.data, model.series, size, structureKey])

  useEffect(() => {
    plotRef.current?.setSize(size)
  }, [size])

  useEffect(() => {
    plotRef.current?.setData(model.data)
  }, [model.data])

  useEffect(
    () => () => {
      plotRef.current?.destroy()
      plotRef.current = null
      structureKeyRef.current = ''
    },
    [],
  )

  return <div className="wave-plot" ref={hostRef} />
}

function buildPlotModel(series: PlotSeries[], timeWindowMs: number) {
  if (series.length === 0) {
    return {
      data: [[0], [0]] as uPlot.AlignedData,
      series: [
        {} as uPlot.Series,
        {
          label: 'No channel selected',
          stroke: '#5f6b7a',
          width: 2,
        } as uPlot.Series,
      ] as uPlot.Series[],
    }
  }

  const latestTimestamp = Math.max(...series.flatMap((item) => item.points.map((point) => point.timestampMs)))
  const windowStart = latestTimestamp - timeWindowMs
  const visibleSeries = series.map((item) => ({
    ...item,
    points: item.points.filter((point) => point.timestampMs >= windowStart),
  }))
  const timestamps = Array.from(new Set(visibleSeries.flatMap((item) => item.points.map((point) => point.timestampMs)))).sort(
    (left, right) => left - right,
  )
  const normalizedTimestamps = timestamps.map((timestamp) => (timestamp - windowStart) / 1000)
  const showPoints = normalizedTimestamps.length <= 240
  const data: Array<number[] | Array<number | null>> = [normalizedTimestamps]
  const plotSeries = [{ label: 'time' } as uPlot.Series]

  for (const item of visibleSeries) {
    const valueByTimestamp = new Map(item.points.map((point) => [point.timestampMs, point.value]))
    data.push(timestamps.map((timestamp) => valueByTimestamp.get(timestamp) ?? null))
    plotSeries.push({
      label: item.name,
      stroke: item.color,
      width: 2,
      points: showPoints ? { show: true, size: 4, width: 1 } : { show: false },
    } as uPlot.Series)
  }

  return { data: data as unknown as uPlot.AlignedData, series: plotSeries }
}

function scaleTimeWindow(current: number, factor: number) {
  return clampTimeWindow(Math.round(current * factor))
}

function clampTimeWindow(value: number) {
  return Math.min(MAX_TIME_WINDOW_MS, Math.max(MIN_TIME_WINDOW_MS, value))
}

function formatTimeWindow(timeWindowMs: number) {
  if (timeWindowMs < 10_000) {
    return `${(timeWindowMs / 1000).toFixed(1)}s`
  }
  if (timeWindowMs < 60_000) {
    return `${Math.round(timeWindowMs / 1000)}s`
  }
  const minutes = timeWindowMs / 60_000
  return `${minutes.toFixed(minutes >= 10 ? 0 : 1)}m`
}
