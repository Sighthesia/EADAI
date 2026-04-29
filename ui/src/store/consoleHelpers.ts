import type { ConsoleEntry, SerialBusEvent } from '../types'

const textEncoder = new TextEncoder()

export const asConsoleEntry = (event: Extract<SerialBusEvent, { kind: 'line' }>): ConsoleEntry => ({
  id: `${event.timestampMs}-${event.line.direction}-${event.line.text}`,
  direction: event.line.direction,
  text: event.line.text,
  timestampMs: event.timestampMs,
  raw: event.line.raw,
  parser: event.parser,
})

export const asProtocolConsoleEntry = (
  event: Extract<SerialBusEvent, { kind: 'telemetryIdentity' | 'telemetrySchema' | 'telemetrySample' }>,
  text: string,
): ConsoleEntry => ({
  id: `${event.timestampMs}-rx-${text}`,
  direction: 'rx',
  text,
  timestampMs: event.timestampMs,
  raw: event.rawFrame,
  parser: event.parser,
})

export const createSentConsoleEntry = (text: string, appendNewline: boolean): ConsoleEntry => {
  const payload = appendNewline ? `${text}\n` : text

  return {
    id: `${Date.now()}-tx-${Math.random().toString(36).slice(2, 10)}`,
    direction: 'tx',
    text,
    timestampMs: Date.now(),
    raw: Array.from(textEncoder.encode(payload)),
    parser: null,
  }
}

export const appendConsoleHistory = (entries: ConsoleEntry[], entry: ConsoleEntry, limit: number) =>
  [...entries, entry].slice(-limit)

export const appendConsoleHistoryBatch = (entries: ConsoleEntry[], nextEntries: ConsoleEntry[], limit: number) => {
  if (nextEntries.length === 0) {
    return entries
  }

  const combined = entries.concat(nextEntries)
  return combined.length > limit ? combined.slice(combined.length - limit) : combined
}
