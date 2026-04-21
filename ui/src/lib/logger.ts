// Minimal dev-time logger wrapper. Use this instead of console.log so
// we can easily control verbosity and replace with a real logger later.
const isDev = Boolean(import.meta.env && import.meta.env.DEV)

const nowMs = () => (typeof performance !== 'undefined' ? performance.now() : Date.now())

type DevTimingOptions = {
  slowThresholdMs?: number
  summaryEvery?: number
  summaryIntervalMs?: number
}

export const logger = {
  debug: (...args: unknown[]) => {
    // Only emit in dev builds to avoid noisy production logs.
    // Keep implementation minimal to avoid adding dependencies.
    // eslint-disable-next-line no-console
    if (isDev) console.debug(...args)
  },
}

export const createDevTimingLogger = (label: string, options: DevTimingOptions = {}) => {
  const slowThresholdMs = options.slowThresholdMs ?? 8
  const summaryEvery = options.summaryEvery ?? 120
  const summaryIntervalMs = options.summaryIntervalMs ?? 5_000

  let sampleCount = 0
  let totalMs = 0
  let maxMs = 0
  let lastSummaryAtMs = nowMs()

  return (durationMs: number, details?: Record<string, unknown>) => {
    if (!isDev) {
      return
    }

    sampleCount += 1
    totalMs += durationMs
    maxMs = Math.max(maxMs, durationMs)

    const currentNowMs = nowMs()
    const shouldLogSlow = durationMs >= slowThresholdMs
    const shouldLogSummary = sampleCount >= summaryEvery || currentNowMs - lastSummaryAtMs >= summaryIntervalMs

    if (!shouldLogSlow && !shouldLogSummary) {
      return
    }

    logger.debug(`[perf] ${label}`, {
      durationMs: Number(durationMs.toFixed(2)),
      samples: sampleCount,
      avgMs: Number((totalMs / sampleCount).toFixed(2)),
      maxMs: Number(maxMs.toFixed(2)),
      mode: shouldLogSlow ? 'slow' : 'summary',
      ...details,
    })

    sampleCount = 0
    totalMs = 0
    maxMs = 0
    lastSummaryAtMs = currentNowMs
  }
}
