import { Profiler, useEffect, useMemo, useState, type ProfilerOnRenderCallback, type ReactNode } from 'react'
import { Layout, Model, TabNode, type IJsonModel } from 'flexlayout-react'
import { createDevTimingLogger } from '../lib/logger'
import { ConnectionPanel } from './ConnectionPanel'
import { McpPanel } from './McpPanel'
import { VariablesPanel } from './VariablesPanel'
import { WaveformPanel } from './WaveformPanel'
import { FrequencySpectrumPage } from './FrequencySpectrumPage'
import { ConsolePanel } from './ConsolePanel'
import { ImuPanel } from './ImuPanel'
import { LogicAnalyzerPage } from './LogicAnalyzerPage'
import { ProtocolPanel } from './ProtocolPanel'
import { ScriptHookPanel } from './ScriptHookPanel'

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

function renderManagedPanel(node: TabNode, id: string, content: JSX.Element) {
  return (
    <TabVisibilityGate node={node}>
      {renderProfiledPanel(id, content)}
    </TabVisibilityGate>
  )
}

function TabVisibilityGate({ node, children }: { node: TabNode; children: ReactNode }) {
  const [isVisible, setIsVisible] = useState(() => node.isVisible())

  useEffect(() => {
    setIsVisible(node.isVisible())

    const onVisibilityChange = ({ visible }: { visible: boolean }) => {
      setIsVisible(visible)
    }

    node.setEventListener('visibility', onVisibilityChange)
    return () => {
      node.removeEventListener('visibility')
    }
  }, [node])

  return isVisible ? <>{children}</> : null
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
      size: 360,
      children: [
        { type: 'tab', name: 'Serial Console', component: 'console' },
        { type: 'tab', name: 'Protocol', component: 'protocol' },
        { type: 'tab', name: 'Script Hooks', component: 'scriptHooks' },
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
    case 'console':
      return renderManagedPanel(node, 'ConsolePanel', <ConsolePanel />)
    case 'protocol':
      return renderManagedPanel(node, 'ProtocolPanel', <ProtocolPanel />)
    case 'scriptHooks':
      return renderManagedPanel(node, 'ScriptHookPanel', <ScriptHookPanel />)
    default:
      return null
  }
}
