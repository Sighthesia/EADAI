import { useMemo } from 'react'
import { Layout, Model, TabNode, type IJsonModel } from 'flexlayout-react'
import { ConnectionPanel } from './ConnectionPanel'
import { McpPanel } from './McpPanel'
import { VariablesPanel } from './VariablesPanel'
import { WaveformPanel } from './WaveformPanel'
import { ConsolePanel } from './ConsolePanel'
import { ImuPanel } from './ImuPanel'
import { LogicAnalyzerPage } from './LogicAnalyzerPage'
import { ProtocolPanel } from './ProtocolPanel'
import { ScriptHookPanel } from './ScriptHookPanel'

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
      return <ConnectionPanel />
    case 'mcp':
      return <McpPanel />
    case 'variables':
      return <VariablesPanel />
    case 'waveforms':
      return <WaveformPanel />
    case 'imu':
      return <ImuPanel />
    case 'logicAnalyzer':
      return <LogicAnalyzerPage />
    case 'console':
      return <ConsolePanel />
    case 'protocol':
      return <ProtocolPanel />
    case 'scriptHooks':
      return <ScriptHookPanel />
    default:
      return null
  }
}
