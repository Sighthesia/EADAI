import type { UiRuntimeCommandCatalogItem } from '../types'

export const MAX_CONSOLE_ENTRIES = 400
export const MAX_SENT_CONSOLE_ENTRIES = 400
export const MAX_SAMPLES_PER_CHANNEL = 960
export const CHANNEL_COLORS = ['#4FC3F7', '#C792EA', '#F78C6C', '#A5E075', '#E6C07B', '#82AAFF']
export const DEFAULT_FAKE_PROFILE = 'telemetry-lab'
export const BMI088_PROTOCOL_NAME = 'bmi088_uart4'
export const PROTOCOL_TIMELINE_LIMIT = 64
export const DEFINITION_SEED_TIMESTAMP_MS = Date.now()
export const MCP_STATUS_POLL_INTERVAL_MS = 1000
export const DEFAULT_LOGIC_SAMPLE_COUNT = 2048

export const BMI088_PROTOCOL_SCRIPT = `// BMI088 UART4 definition seed
onSchema((fields, rateHz, sampleLen) => {
  // Use logger.debug to avoid leaving console.log in production code
  // eslint-disable-next-line no-undef
  if (import.meta.env && import.meta.env.DEV) console.debug('schema', { rateHz, sampleLen, fields })
})

onSample((record) => {
  // eslint-disable-next-line no-undef
  if (import.meta.env && import.meta.env.DEV) console.debug('sample', record)
})`

export const RUNTIME_COMMAND_CATALOG: UiRuntimeCommandCatalogItem[] = [
  {
    command: 'REQ_IDENTITY',
    label: 'Request identity',
    description: 'Ask the device to publish identity metadata before loading schema.',
    recommendedPhase: 'awaitingIdentity',
    payloadPreview: 'REQ_IDENTITY',
  },
  {
    command: 'REQ_SCHEMA',
    label: 'Request schema',
    description: 'Ask the device to publish the current telemetry layout before sampling.',
    recommendedPhase: 'awaitingSchema',
    payloadPreview: 'REQ_SCHEMA',
  },
  {
    command: 'ACK',
    label: 'Acknowledge schema',
    description: 'Confirm the received schema and advance the handshake.',
    recommendedPhase: 'awaitingAck',
    payloadPreview: 'ACK',
  },
  {
    command: 'START',
    label: 'Start streaming',
    description: 'Transition from handshake into continuous telemetry streaming.',
    recommendedPhase: 'awaitingStart',
    payloadPreview: 'START',
  },
  {
    command: 'STOP',
    label: 'Stop streaming',
    description: 'Pause streaming and return the runtime to an idle state.',
    recommendedPhase: 'streaming',
    payloadPreview: 'STOP',
  },
  {
    command: 'REQ_TUNING',
    label: 'Request tuning',
    description: 'Fetch current tuning state before editing values.',
    recommendedPhase: null,
    payloadPreview: 'REQ_TUNING',
  },
  {
    command: 'SET_TUNING',
    label: 'Apply tuning',
    description: 'Send a tuning payload to update device parameters.',
    recommendedPhase: null,
    parameters: [
      {
        name: 'payload',
        label: 'Tuning payload',
        kind: 'text',
        placeholder: 'key=value',
        description: 'ASCII tuning payload without trailing newline.',
      },
    ],
    payloadPreview: 'SET_TUNING · payload=value',
  },
  {
    command: 'SHELL_EXEC',
    label: 'Run shell command',
    description: 'Bridge one firmware shell command over the framed UART path.',
    recommendedPhase: null,
    parameters: [
      {
        name: 'payload',
        label: 'Shell command',
        kind: 'text',
        placeholder: 'echo hello',
        description: 'ASCII command line without trailing newline.',
      },
    ],
    payloadPreview: 'SHELL_EXEC · payload=echo hello',
  },
]
