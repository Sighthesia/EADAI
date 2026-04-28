import { Profiler, useMemo, type ProfilerOnRenderCallback } from 'react'
import { Layout, Model, TabNode, type IJsonModel } from 'flexlayout-react'
import { createDevTimingLogger } from '../lib/logger'
import { ConnectionPanel } from './ConnectionPanel'
import { McpPanel } from './McpPanel'
import { VariablesPanel } from './VariablesPanel'
import { WaveformPanel } from './WaveformPanel'
import { FrequencySpectrumPage } from './FrequencySpectrumPage'
import { ImuPanel } from './ImuPanel'
import { LogicAnalyzerPage } from './LogicAnalyzerPage'
import { RuntimePanel } from './RuntimePanel'
import { ScriptsPanel } from './ScriptsPanel'

const panelRenderProfilers = new Map<string, ReturnType<typeof createDevTimingLogger>>()

const profilePanelRender: ProfilerOnRenderCallback = (
  id,
  phase,
  actualDuration,
  baseDuration,
) => {
  let profile = panelRenderProfilers.get(id)
  if (!profile) {
    profile = createDevTimingLogger(`panel-render ${id}`, {
      slowThresholdMs: 6,
      summaryEvery: 80,
      summaryIntervalMs: 4_000,
    })
    panelRenderProfilers.set(id, profile)
  }

  profile(actualDuration, {
    phase,
    baseDurationMs: Number(baseDuration.toFixed(2)),
  })
}

function renderProfiledPanel(id: string, node: JSX.Element) {
  return (
    <Profiler id={id} onRender={profilePanelRender}>
      {node}
    </Profiler>
  )
}

function renderManagedPanel(_node: TabNode, id: string, content: JSX.Element) {
  return renderProfiledPanel(id, content)
}

const layoutJson: IJsonModel = {
  global: {
    borderEnableAutoHide: true,
    tabEnableClose: false,
    tabEnableRename: false,
    tabSetEnableDrop: true,
    splitterSize: 6,
  },
  borders: [
    {
      type: 'border',
      location: 'left',
      selected: 0,
      size: 340,
      children: [
        { type: 'tab', name: 'Connection', component: 'connection' },
        { type: 'tab', name: 'MCP', component: 'mcp' },
        { type: 'tab', name: 'Variables', component: 'variables' },
      ],
    },
    {
      type: 'border',
      location: 'right',
      selected: 0,
      size: 420,
      children: [
        { type: 'tab', name: 'Terminal', component: 'runtime' },
        { type: 'tab', name: 'Scripts', component: 'scripts' },
      ],
    },
    {
      type: 'border',
      location: 'bottom',
      selected: -1,
      size: 280,
      children: [],
    },
  ],
  layout: {
    type: 'row',
    children: [
      {
        type: 'tabset',
        weight: 100,
        children: [
          { type: 'tab', name: 'Waveforms', component: 'waveforms' },
          { type: 'tab', name: 'FFT Spectrum', component: 'frequencySpectrum' },
          { type: 'tab', name: 'IMU', component: 'imu' },
          { type: 'tab', name: 'Logic Analyzer', component: 'logicAnalyzer' },
        ],
      },
    ],
  },
}

export function Workbench() {
  const model = useMemo(() => Model.fromJson(layoutJson), [])

  return (
    <div className="eadai-layout">
      <Layout model={model} factory={factory} />
    </div>
  )
}

function factory(node: TabNode) {
  const component = node.getComponent()

  switch (component) {
    case 'connection':
      return renderManagedPanel(node, 'ConnectionPanel', <ConnectionPanel />)
    case 'mcp':
      return renderManagedPanel(node, 'McpPanel', <McpPanel />)
    case 'variables':
      return renderManagedPanel(node, 'VariablesPanel', <VariablesPanel />)
    case 'waveforms':
      return renderManagedPanel(node, 'WaveformPanel', <WaveformPanel />)
    case 'frequencySpectrum':
      return renderManagedPanel(node, 'FrequencySpectrumPage', <FrequencySpectrumPage />)
    case 'imu':
      return renderManagedPanel(node, 'ImuPanel', <ImuPanel />)
    case 'logicAnalyzer':
      return renderManagedPanel(node, 'LogicAnalyzerPage', <LogicAnalyzerPage />)
    case 'runtime':
      return renderManagedPanel(node, 'RuntimePanel', <RuntimePanel />)
    case 'scripts':
      return renderManagedPanel(node, 'ScriptsPanel', <ScriptsPanel />)
    default:
      return null
  }
}
