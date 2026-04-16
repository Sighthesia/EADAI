import { useMemo } from 'react'
import { Layout, Model, TabNode } from 'flexlayout-react'
import { ConnectionPanel } from './ConnectionPanel'
import { McpPanel } from './McpPanel'
import { VariablesPanel } from './VariablesPanel'
import { WaveformPanel } from './WaveformPanel'
import { ConsolePanel } from './ConsolePanel'
import { ImuPanel } from './ImuPanel'
import { LogicAnalyzerPage } from './LogicAnalyzerPage'

const layoutJson = {
  global: {
    tabEnableClose: false,
    tabEnableRename: false,
    tabSetEnableDrop: true,
    splitterSize: 6,
    tabSetHeaderHeight: 34,
    tabSetTabStripHeight: 34,
  },
  layout: {
    type: 'row',
    children: [
      {
        type: 'tabset',
        weight: 24,
        children: [
          { type: 'tab', name: 'Connection', component: 'connection' },
          { type: 'tab', name: 'MCP', component: 'mcp' },
          { type: 'tab', name: 'Variables', component: 'variables' },
        ],
      },
      {
        type: 'row',
        weight: 76,
        children: [
          {
            type: 'tabset',
            weight: 68,
            children: [
              { type: 'tab', name: 'Waveforms', component: 'waveforms' },
              { type: 'tab', name: 'IMU', component: 'imu' },
              { type: 'tab', name: 'Logic Analyzer', component: 'logicAnalyzer' },
            ],
          },
          {
            type: 'tabset',
            weight: 32,
            children: [{ type: 'tab', name: 'Serial Console', component: 'console' }],
          },
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
    default:
      return null
  }
}
