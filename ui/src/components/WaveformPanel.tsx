import { useEffect, useMemo, useRef, useState } from 'react'
import uPlot from 'uplot'
import { useAppStore } from '../store/appStore'

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
      <WavePlot series={series} />
    </section>
  )
}

function WavePlot({ series }: { series: PlotSeries[] }) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const [size, setSize] = useState({ width: 900, height: 440 })
  const model = useMemo(() => buildPlotModel(series), [series])

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

    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (!hostRef.current || model.data.length === 0) {
      return
    }

    plotRef.current?.destroy()
    plotRef.current = new uPlot(
      {
        width: size.width,
        height: size.height,
        padding: [12, 16, 12, 16],
        scales: { x: { time: false } },
        axes: [
          { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
          { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
        ],
        series: model.series,
      },
      model.data,
      hostRef.current,
    )

    return () => {
      plotRef.current?.destroy()
      plotRef.current = null
    }
  }, [model, size])

  return <div className="wave-plot" ref={hostRef} />
}

function buildPlotModel(series: PlotSeries[]) {
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

  const timestamps = Array.from(new Set(series.flatMap((item) => item.points.map((point) => point.timestampMs)))).sort(
    (left, right) => left - right,
  )
  const data: Array<number[] | Array<number | null>> = [timestamps]
  const plotSeries = [{ label: 'time' } as uPlot.Series]

  for (const item of series) {
    const valueByTimestamp = new Map(item.points.map((point) => [point.timestampMs, point.value]))
    data.push(timestamps.map((timestamp) => valueByTimestamp.get(timestamp) ?? null))
    plotSeries.push({
      label: item.name,
      stroke: item.color,
      width: 2,
      points: { show: false },
    } as uPlot.Series)
  }

  return { data: data as unknown as uPlot.AlignedData, series: plotSeries }
}
