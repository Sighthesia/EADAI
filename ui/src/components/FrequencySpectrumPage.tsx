import { useEffect, useMemo, useRef, useState } from 'react'
import uPlot from 'uplot'
import { computeFrequencySpectrum, type FrequencySpectrumResult } from '../lib/frequencySpectrum'
import { formatWaveformWindowMs } from '../lib/waveformWindow'
import { useAppStore } from '../store/appStore'

type SpectrumChannel = {
  name: string
  color: string
  sampleCount: number
  result: FrequencySpectrumResult
}

export function FrequencySpectrumPage() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const waveformWindowMs = useAppStore((state) => state.waveformWindowMs)
  const [menuOpen, setMenuOpen] = useState(true)
  const [activeChannel, setActiveChannel] = useState<string | null>(null)
  const windowEndMs = useMemo(() => {
    const latestPointTimestampMs = selectedChannels.reduce((max, channel) => {
      const points = variables[channel]?.points ?? []
      const pointMax = points.reduce((latest, point) => Math.max(latest, point.timestampMs), Number.NEGATIVE_INFINITY)
      return Math.max(max, pointMax)
    }, Number.NEGATIVE_INFINITY)

    const latestUpdateTimestampMs = selectedChannels.reduce((max, channel) => {
      const updatedAtMs = variables[channel]?.updatedAtMs ?? Number.NEGATIVE_INFINITY
      return Math.max(max, updatedAtMs)
    }, Number.NEGATIVE_INFINITY)

    if (Number.isFinite(latestPointTimestampMs)) {
      return latestPointTimestampMs
    }

    if (Number.isFinite(latestUpdateTimestampMs)) {
      return latestUpdateTimestampMs
    }

    return Date.now()
  }, [selectedChannels, variables])

  const availableChannels = useMemo(
    () =>
      selectedChannels
        .map((channel) => variables[channel])
        .filter((variable): variable is NonNullable<typeof variable> => Boolean(variable))
        .filter((variable) => variable.points.length > 0)
        .map((variable) => ({
          name: variable.name,
          color: colorForChannel(variable.name),
          sampleCount: variable.points.length,
          result: computeFrequencySpectrum(windowedPoints(variable.points, waveformWindowMs, windowEndMs)),
        })),
    [colorForChannel, selectedChannels, variables, waveformWindowMs, windowEndMs],
  )

  useEffect(() => {
    if (availableChannels.length === 0) {
      setActiveChannel(null)
      return
    }

    if (!activeChannel || !availableChannels.some((channel) => channel.name === activeChannel)) {
      setActiveChannel(availableChannels[0]!.name)
    }
  }, [activeChannel, availableChannels])

  const activeSpectrum = availableChannels.find((channel) => channel.name === activeChannel) ?? availableChannels[0] ?? null

  return (
    <section className="panel waveform-panel frequency-spectrum-panel">
      <SpectrumPlot spectrum={activeSpectrum} />

      <div className="waveform-stage-hud">
        <div className="waveform-stage-meta">
          <strong>FFT Spectrum</strong>
          <span>{activeSpectrum ? `${activeSpectrum.name} · ${activeSpectrum.sampleCount} samples` : 'Idle'}</span>
          <small>{formatWaveformWindowMs(waveformWindowMs)}</small>
        </div>
        {activeSpectrum?.result.peakFrequencyHz !== null ? (
          <div className="waveform-stage-legend">
            <strong>Peak</strong>
            <span>{formatFrequency(activeSpectrum.result.peakFrequencyHz)}</span>
            <small>{formatAmplitude(activeSpectrum.result.peakAmplitude)}</small>
          </div>
        ) : (
          <div className="waveform-stage-legend">
            <strong>Peak</strong>
            <span>—</span>
            <small>Waiting for enough samples</small>
          </div>
        )}
      </div>

      <div className={`waveform-floating-menu ${menuOpen ? '' : 'collapsed'}`}>
        <div className="waveform-floating-top">
          {menuOpen ? (
            <div className="waveform-floating-heading">
              <strong>Spectrum Controls</strong>
              <small>{activeSpectrum ? 'Spectrum follows the shared waveform window.' : 'Select a numeric waveform to build a spectrum.'}</small>
            </div>
          ) : null}
          <button type="button" className="ghost-button waveform-floating-toggle" onClick={() => setMenuOpen((value) => !value)}>
            {menuOpen ? 'Hide' : 'Spectrum'}
          </button>
        </div>

        {menuOpen ? (
          <div className="waveform-floating-scroll">
            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Channels</strong>
                <small>{availableChannels.length > 0 ? `${availableChannels.length} spectrum-ready` : 'No numeric channel selected'}</small>
              </div>
              <div className="waveform-controls">
                {availableChannels.map((channel) => (
                  <button
                    key={channel.name}
                    type="button"
                    className={`channel-chip ${activeChannel === channel.name ? 'selected' : ''}`}
                    onClick={() => setActiveChannel(channel.name)}
                  >
                    <span className="variable-color" style={{ background: channel.color }} aria-hidden="true" />
                    {channel.name}
                  </button>
                ))}
              </div>
            </section>

            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Peak Readout</strong>
                <small>{activeSpectrum?.result.sampleRateHz ? `${formatFrequency(activeSpectrum.result.sampleRateHz)} sample rate` : 'Waiting for samples'}</small>
              </div>
              <p className="waveform-floating-note">
                {activeSpectrum
                  ? `Peak at ${formatFrequency(activeSpectrum.result.peakFrequencyHz)} with ${formatAmplitude(activeSpectrum.result.peakAmplitude)}.`
                  : 'Select a waveform channel with numeric samples to reveal frequency components.'}
              </p>
            </section>
          </div>
        ) : null}
      </div>

      {activeSpectrum ? null : (
        <div className="logic-bottom-banner" role="status">
          <strong>Spectrum unavailable</strong>
          <span>Select a numeric waveform channel and let it collect enough samples.</span>
        </div>
      )}
    </section>
  )
}

function SpectrumPlot({ spectrum }: { spectrum: SpectrumChannel | null }) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const structureKeyRef = useRef('')
  const [size, setSize] = useState({ width: 900, height: 440 })

  const model = useMemo(() => buildSpectrumModel(spectrum), [spectrum])
  const structureKey = useMemo(() => (model.series.length <= 1 ? '__empty__' : spectrum?.name ?? '__empty__'), [model.series.length, spectrum?.name])

  useEffect(() => {
    if (!spectrum || spectrum.result.bins.length === 0) {
      plotRef.current?.destroy()
      plotRef.current = null
      structureKeyRef.current = ''
      return
    }

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
  }, [spectrum?.name, spectrum?.result.bins.length])

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
              values: (_, ticks) => ticks.map((tick) => formatFrequency(tick)),
            },
            {
              stroke: '#5f6b7a',
              grid: { stroke: '#20242d' },
              values: (_, ticks) => ticks.map((tick) => formatAmplitude(tick)),
            },
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
    if (!plotRef.current) {
      return
    }

    plotRef.current.setData(model.data)
    plotRef.current.setScale('x', { min: model.xMin, max: model.xMax })
    plotRef.current.setScale('y', { min: model.yMin, max: model.yMax })
  }, [model.data, model.xMin, model.xMax, model.yMin, model.yMax])

  useEffect(
    () => () => {
      plotRef.current?.destroy()
      plotRef.current = null
      structureKeyRef.current = ''
    },
    [],
  )

  const isEmpty = !spectrum || spectrum.result.bins.length === 0

  return (
    <div className="wave-plot waveform-stage-surface frequency-spectrum-stage-surface">
      <div ref={hostRef} style={{ width: '100%', height: '100%' }} />
      {isEmpty ? <div className="frequency-spectrum-empty-state">No spectrum data yet.</div> : null}
    </div>
  )
}

function buildSpectrumModel(spectrum: SpectrumChannel | null) {
  if (!spectrum || spectrum.result.bins.length === 0) {
    return {
      data: [[0], [0]] as uPlot.AlignedData,
      series: [{} as uPlot.Series, { label: 'No spectrum data', stroke: '#5f6b7a', width: 2 } as uPlot.Series],
      xMin: 0,
      xMax: 1,
      yMin: 0,
      yMax: 1,
    }
  }

  const frequencies = spectrum.result.bins.map((bin) => bin.frequencyHz)
  const amplitudes = spectrum.result.bins.map((bin) => bin.amplitude)
  const peakAmplitude = spectrum.result.peakAmplitude ?? Math.max(...amplitudes)
  const xMax = Math.max(frequencies[frequencies.length - 1] ?? 1, 1)
  const yMax = Math.max(peakAmplitude * 1.2, 1e-6)

  return {
    data: [frequencies, amplitudes] as unknown as uPlot.AlignedData,
    series: [
      {} as uPlot.Series,
      {
        label: spectrum.name,
        stroke: spectrum.color,
        width: 2,
        points: { show: false },
      } as uPlot.Series,
    ],
    xMin: 0,
    xMax,
    yMin: 0,
    yMax,
  }
}

function windowedPoints(points: { timestampMs: number; value: number }[], windowMs: number, windowEndMs: number) {
  const windowStartMs = Number.isFinite(windowEndMs) ? windowEndMs - windowMs : Number.NEGATIVE_INFINITY
  return points.filter((point) => point.timestampMs >= windowStartMs)
}

function formatFrequency(value: number | null | undefined) {
  if (!Number.isFinite(value ?? Number.NaN)) {
    return '—'
  }

  const frequency = value as number
  if (frequency >= 1_000_000) {
    return `${(frequency / 1_000_000).toFixed(frequency % 1_000_000 === 0 ? 0 : 2)} MHz`
  }
  if (frequency >= 1_000) {
    return `${(frequency / 1_000).toFixed(frequency % 1_000 === 0 ? 0 : 2)} kHz`
  }
  return `${frequency.toFixed(frequency >= 100 ? 0 : 2)} Hz`
}

function formatAmplitude(value: number | null | undefined) {
  if (!Number.isFinite(value ?? Number.NaN)) {
    return '—'
  }

  return `${(value as number).toFixed(4)}`
}
