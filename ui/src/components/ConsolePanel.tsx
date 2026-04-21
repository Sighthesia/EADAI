import { memo, useEffect, useMemo, useRef } from 'react'
import { useAppStore } from '../store/appStore'
import type { ConsoleEntry } from '../types'

export function ConsolePanel() {
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const consoleDisplayMode = useAppStore((state) => state.consoleDisplayMode)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const setConsoleDisplayMode = useAppStore((state) => state.setConsoleDisplayMode)
  const send = useAppStore((state) => state.send)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)
  const scrollRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [consoleEntries])

  const displayModes = useMemo(() => ['text', 'hex', 'binary'] as const, [])

  return (
    <section className="panel console-panel">
      <div className="console-stream" ref={scrollRef}>
        {consoleEntries.map((entry) => (
          <ConsoleEntryRow key={entry.id} entry={entry} mode={consoleDisplayMode} />
        ))}
      </div>
      <div className="toolbar-row console-toolbar-row">
        <div className="console-display-switch" role="group" aria-label="Console display mode">
          {displayModes.map((mode) => (
            <button key={mode} type="button" className={`metric-display-button ${consoleDisplayMode === mode ? 'active' : ''}`} onClick={() => setConsoleDisplayMode(mode)}>
              {mode.toUpperCase()}
            </button>
          ))}
        </div>
        <div className="console-command-strip" role="group" aria-label="BMI088 commands">
          {(['ACK', 'START', 'STOP', 'REQ_SCHEMA'] as const).map((command) => (
            <button key={command} type="button" className="ghost-button" onClick={() => void sendBmi088Command(command)}>
              {command}
            </button>
          ))}
        </div>
      </div>
      <div className="console-compose">
        <textarea
          value={commandInput}
          onChange={(event) => setCommandInput(event.target.value)}
          placeholder="Type payload, for example: motor_pwm:120"
          rows={3}
        />
        <div className="toolbar-row">
          <label className="checkbox-row">
            <input type="checkbox" checked={appendNewline} onChange={(event) => setAppendNewline(event.target.checked)} />
            <span>Append newline</span>
          </label>
          <button className="primary-button" onClick={() => void send()}>
            Send
          </button>
        </div>
      </div>
    </section>
  )
}

const ConsoleEntryRow = memo(function ConsoleEntryRow({ entry, mode }: { entry: ConsoleEntry; mode: 'text' | 'hex' | 'binary' }) {
  const timestamp = useMemo(() => formatTime(entry.timestampMs), [entry.timestampMs])

  return (
    <div className={`console-line ${entry.direction}`}>
      <div className="console-line-meta">
        <span className="console-badge">{entry.direction.toUpperCase()}</span>
        <span className="console-time">{timestamp}</span>
        {entry.parser?.parserName ? <span className="metric-chip">{entry.parser.parserName}</span> : null}
        <span className="metric-chip">{entry.raw.length} B</span>
      </div>
      <ConsoleEntryBody entry={entry} mode={mode} />
    </div>
  )
})

function formatTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleTimeString()
}

const ConsoleEntryBody = memo(function ConsoleEntryBody({ entry, mode }: { entry: ConsoleEntry; mode: 'text' | 'hex' | 'binary' }) {
  const display = useMemo(() => formatEntry(entry, mode), [entry, mode])
  const bmi088Frame = useMemo(() => decodeBmi088Frame(entry.raw), [entry.raw])
  const bmi088FrameIssue = useMemo(() => diagnoseBmi088Frame(entry), [entry])

  return (
    <div className="console-line-body">
      <div className="console-line-summary">{display.summary}</div>
      {bmi088Frame ? <Bmi088FrameHeader frame={bmi088Frame} /> : null}
      {bmi088FrameIssue ? <Bmi088FrameIssueView issue={bmi088FrameIssue} /> : null}
      {bmi088Frame?.decoded ? <Bmi088PayloadView decoded={bmi088Frame.decoded} /> : null}
      <code>{display.content}</code>
    </div>
  )
})

function Bmi088FrameIssueView({ issue }: { issue: string }) {
  return (
    <div className="console-frame-issue">
      <strong>BMI088 decode issue</strong>
      <span>{issue}</span>
    </div>
  )
}

function Bmi088FrameHeader({ frame }: { frame: Bmi088FrameHeaderView }) {
  return (
    <div className="console-frame-card">
      <div className="console-frame-header">
        <strong>BMI088 frame</strong>
        <span className={`metric-chip ${frame.crcValid ? '' : 'tone-danger'}`}>{frame.crcValid ? 'CRC OK' : 'CRC BAD'}</span>
      </div>
      <div className="console-frame-grid">
        <span>SOF</span>
        <strong>{frame.sof}</strong>
        <span>Type</span>
        <strong>{frame.typeLabel}</strong>
        <span>Cmd</span>
        <strong>{frame.commandLabel}</strong>
        <span>Len</span>
        <strong>{frame.payloadLength} B</strong>
        <span>CRC</span>
        <strong>{frame.crcHex}</strong>
        <span>Payload</span>
        <strong>{frame.payloadRange}</strong>
      </div>
    </div>
  )
}

function Bmi088PayloadView({ decoded }: { decoded: Bmi088DecodedPayload }) {
  return (
    <div className="console-payload-card">
      <div className="console-frame-header">
        <strong>Decoded payload</strong>
        <span className="metric-chip">{decoded.kind.toUpperCase()}</span>
      </div>
      {decoded.kind === 'command' ? <div className="console-payload-note">Empty request payload.</div> : null}
      {decoded.kind === 'schema' ? (
        <div className="console-payload-stack">
          <div className="console-payload-note">
            {decoded.rateHz} Hz, {decoded.sampleLen} bytes, {decoded.fieldCount} fields
          </div>
          <div className="console-payload-grid">
            {decoded.fields.map((field) => (
              <div key={`${field.index}-${field.name}`} className="console-payload-item">
                <strong>{field.index}. {field.name}</strong>
                <span>{field.unit} · q{field.scaleQ}</span>
              </div>
            ))}
          </div>
        </div>
      ) : null}
      {decoded.kind === 'sample' ? (
        <div className="console-payload-grid">
          {decoded.values.map((field) => (
            <div key={`${field.index}-${field.name}`} className="console-payload-item">
              <strong>{field.index}. {field.name}</strong>
              <span>{field.displayValue}</span>
              <span>raw {field.raw}</span>
            </div>
          ))}
        </div>
      ) : null}
      {decoded.kind === 'unknown' ? <div className="console-payload-note">{decoded.note}</div> : null}
    </div>
  )
}

function formatEntry(entry: ConsoleEntry, mode: 'text' | 'hex' | 'binary') {
  const parserLabel = entry.parser?.parserName ? `${entry.parser.parserName} · ` : ''
  const summary = `${parserLabel}${entry.raw.length} bytes`

  if (mode === 'text') {
    return {
      summary,
      content: entry.text || '[empty text payload]',
    }
  }

  if (mode === 'hex') {
    return {
      summary: `${summary} · grouped by 8 bytes`,
      content: chunkBytes(entry.raw, 8)
        .map((group) => group.map((byte) => byte.toString(16).padStart(2, '0').toUpperCase()).join(' '))
        .join('\n'),
    }
  }

  return {
    summary: `${summary} · grouped by 4 bytes`,
    content: chunkBytes(entry.raw, 4)
      .map((group) => group.map((byte) => byte.toString(2).padStart(8, '0')).join(' '))
      .join('\n'),
  }
}

function chunkBytes(bytes: number[], size: number) {
  const chunks: number[][] = []

  for (let index = 0; index < bytes.length; index += size) {
    chunks.push(bytes.slice(index, index + size))
  }

  return chunks
}

type Bmi088FrameHeaderView = {
  sof: string
  typeLabel: string
  commandLabel: string
  payloadLength: number
  crcHex: string
  payloadRange: string
  crcValid: boolean
  decoded: Bmi088DecodedPayload | null
}

type Bmi088DecodedPayload =
  | {
      kind: 'command'
    }
  | {
      kind: 'schema'
      rateHz: number
      sampleLen: number
      fieldCount: number
      fields: Array<{ index: number; name: string; unit: string; scaleQ: number }>
    }
  | {
      kind: 'sample'
      values: Array<{ index: number; name: string; raw: number; value: number; unit: string; displayValue: string }>
    }
  | {
      kind: 'unknown'
      note: string
    }

function decodeBmi088Frame(raw: number[]): Bmi088FrameHeaderView | null {
  if (raw.length < 9 || raw[0] !== 0xa5 || raw[1] !== 0x5a) {
    return null
  }

  const version = raw[2]
  const frameType = raw[3]
  const command = raw[4]
  const payloadLength = (raw[5] ?? 0) | ((raw[6] ?? 0) << 8)
  const expectedLength = 7 + payloadLength + 2

  if (version !== 0x01 || raw.length < expectedLength) {
    return null
  }

  const crcValue = (raw[expectedLength - 2] ?? 0) | ((raw[expectedLength - 1] ?? 0) << 8)
  const calculatedCrc = crc16Ccitt(raw.slice(2, expectedLength - 2))
  const payload = raw.slice(7, expectedLength - 2)

  return {
    sof: 'A5 5A',
    typeLabel: bmi088TypeLabel(frameType),
    commandLabel: bmi088CommandLabel(command),
    payloadLength,
    crcHex: `0x${crcValue.toString(16).padStart(4, '0').toUpperCase()}`,
    payloadRange: payloadLength === 0 ? 'empty' : `[7..${expectedLength - 3}]`,
    crcValid: crcValue === calculatedCrc,
    decoded: decodeBmi088Payload(frameType, command, payload),
  }
}

function diagnoseBmi088Frame(entry: ConsoleEntry) {
  const raw = entry.raw
  const parserName = entry.parser?.parserName ?? ''
  const hintedAsBmi088 = parserName.startsWith('bmi088') || raw[0] === 0xa5 || raw[1] === 0x5a

  if (!hintedAsBmi088) {
    return null
  }

  if (raw.length < 2) {
    return 'frame too short to contain SOF'
  }

  if (raw[0] !== 0xa5 || raw[1] !== 0x5a) {
    return 'missing SOF A5 5A'
  }

  if (raw.length < 7) {
    return 'truncated before BMI088 header completed'
  }

  if (raw[2] !== 0x01) {
    return `unsupported version 0x${(raw[2] ?? 0).toString(16).padStart(2, '0').toUpperCase()}`
  }

  const payloadLength = (raw[5] ?? 0) | ((raw[6] ?? 0) << 8)
  const expectedLength = 7 + payloadLength + 2
  if (raw.length < expectedLength) {
    return `truncated frame: expected ${expectedLength} bytes, got ${raw.length}`
  }

  const crcValue = (raw[expectedLength - 2] ?? 0) | ((raw[expectedLength - 1] ?? 0) << 8)
  const calculatedCrc = crc16Ccitt(raw.slice(2, expectedLength - 2))
  if (crcValue !== calculatedCrc) {
    return `crc mismatch: got 0x${crcValue.toString(16).padStart(4, '0').toUpperCase()}, expected 0x${calculatedCrc.toString(16).padStart(4, '0').toUpperCase()}`
  }

  const command = raw[4] ?? 0
  const frameType = raw[3] ?? 0
  if (!isKnownBmi088Command(command) || !isKnownBmi088Type(frameType)) {
    return 'unsupported BMI088 type or command'
  }

  return null
}

function decodeBmi088Payload(frameType: number, command: number, payload: number[]): Bmi088DecodedPayload | null {
  if (frameType === 0x01) {
    return { kind: 'command' }
  }

  if (frameType !== 0x02) {
    return { kind: 'unknown', note: 'Unsupported frame type for payload decode.' }
  }

  if (command === 0x80) {
    return decodeSchemaPayload(payload)
  }

  if (command === 0x81) {
    return decodeSamplePayload(payload)
  }

  return { kind: 'unknown', note: 'No payload decoder for this command.' }
}

function decodeSchemaPayload(payload: number[]): Bmi088DecodedPayload {
  if (payload.length < 7) {
    return { kind: 'unknown', note: 'Schema payload too short.' }
  }

  const rateHz = readU32Le(payload, 0)
  const sampleLen = readU16Le(payload, 4)
  const fieldCount = payload[6] ?? 0
  let offset = 7
  const fields: Array<{ index: number; name: string; unit: string; scaleQ: number }> = []

  for (let index = 0; index < fieldCount; index += 1) {
    const nameLen = payload[offset] ?? 0
    offset += 1
    const name = decodeAscii(payload.slice(offset, offset + nameLen))
    offset += nameLen

    const scaleQ = toSignedI8(payload[offset] ?? 0)
    offset += 1

    const unitLen = payload[offset] ?? 0
    offset += 1
    const unit = decodeAscii(payload.slice(offset, offset + unitLen))
    offset += unitLen

    fields.push({
      index,
      name,
      unit,
      scaleQ,
    })
  }

  return {
    kind: 'schema',
    rateHz,
    sampleLen,
    fieldCount,
    fields,
  }
}

function decodeSamplePayload(payload: number[]): Bmi088DecodedPayload {
  const names = ['acc_x', 'acc_y', 'acc_z', 'gyro_x', 'gyro_y', 'gyro_z', 'roll', 'pitch', 'yaw']
  const units = ['raw', 'raw', 'raw', 'raw', 'raw', 'raw', 'deg', 'deg', 'deg']
  const scales = [0, 0, 0, 0, 0, 0, -2, -2, -2]
  const values = chunkBytes(payload, 2)
    .filter((group) => group.length === 2)
    .map((group, index) => {
      const raw = readI16Le(group, 0)
      const scaleQ = scales[index] ?? 0
      const value = scaleRaw(raw, scaleQ)
      const unit = units[index] ?? 'raw'

      return {
        index,
        name: names[index] ?? `field_${index}`,
        raw,
        value,
        unit,
        displayValue: formatEngineeringValue(value, scaleQ, unit),
      }
    })

  return {
    kind: 'sample',
    values,
  }
}

function readU16Le(bytes: number[], offset: number) {
  return (bytes[offset] ?? 0) | ((bytes[offset + 1] ?? 0) << 8)
}

function readU32Le(bytes: number[], offset: number) {
  return ((bytes[offset] ?? 0)
    | ((bytes[offset + 1] ?? 0) << 8)
    | ((bytes[offset + 2] ?? 0) << 16)
    | ((bytes[offset + 3] ?? 0) << 24)) >>> 0
}

function readI16Le(bytes: number[], offset: number) {
  const value = readU16Le(bytes, offset)
  return value > 0x7fff ? value - 0x10000 : value
}

function toSignedI8(value: number) {
  return value > 0x7f ? value - 0x100 : value
}

function decodeAscii(bytes: number[]) {
  return bytes.map((byte) => String.fromCharCode(byte)).join('')
}

function scaleRaw(raw: number, scaleQ: number) {
  return raw * 10 ** scaleQ
}

function formatEngineeringValue(value: number, scaleQ: number, unit: string) {
  const digits = scaleQ < 0 ? Math.abs(scaleQ) : 0
  const rendered = digits > 0 ? value.toFixed(Math.min(digits, 4)) : value.toString()
  return `${rendered} ${unit}`.trim()
}

function bmi088TypeLabel(frameType: number) {
  switch (frameType) {
    case 0x01:
      return 'REQUEST (0x01)'
    case 0x02:
      return 'EVENT (0x02)'
    default:
      return `UNKNOWN (0x${frameType.toString(16).padStart(2, '0').toUpperCase()})`
  }
}

function isKnownBmi088Type(frameType: number) {
  return frameType === 0x01 || frameType === 0x02
}

function bmi088CommandLabel(command: number) {
  switch (command) {
    case 0x10:
      return 'ACK (0x10)'
    case 0x11:
      return 'START (0x11)'
    case 0x12:
      return 'STOP (0x12)'
    case 0x13:
      return 'REQ_SCHEMA (0x13)'
    case 0x80:
      return 'SCHEMA (0x80)'
    case 0x81:
      return 'SAMPLE (0x81)'
    default:
      return `UNKNOWN (0x${command.toString(16).padStart(2, '0').toUpperCase()})`
  }
}

function isKnownBmi088Command(command: number) {
  return [0x10, 0x11, 0x12, 0x13, 0x80, 0x81].includes(command)
}

function crc16Ccitt(bytes: number[]) {
  let crc = 0xffff

  for (const byte of bytes) {
    crc ^= byte << 8
    for (let bit = 0; bit < 8; bit += 1) {
      if ((crc & 0x8000) !== 0) {
        crc = ((crc << 1) ^ 0x1021) & 0xffff
      } else {
        crc = (crc << 1) & 0xffff
      }
    }
  }

  return crc
}
