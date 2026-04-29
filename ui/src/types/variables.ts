export type UiDefinitionStatus = 'seeded' | 'draft' | 'observed'

export type VariableSourceKind = 'protocol-text' | 'telemetry-sample'

export type VariableExtractorKind = 'parser-field' | 'sample-field'

export type VariableDefinitionVisibility = 'both' | 'runtime' | 'variables' | 'hidden'

export type UiHookDefinitionEvent = 'schema' | 'sample' | 'trigger'

export interface UiScriptHookExample {
  name: string
  snippet: string
}

export interface ScriptDefinition {
  id: string
  name: string
  summary: string
  language: 'typescript' | 'javascript'
  source: string
  status: UiDefinitionStatus
  updatedAtMs: number
}

export interface HookDefinition {
  id: string
  name: string
  event: UiHookDefinitionEvent
  summary: string
  source: string
  enabled: boolean
  status: UiDefinitionStatus
  updatedAtMs: number
}

export interface VariableDefinition {
  id: string
  name: string
  deviceRef?: string | null
  sourceKind: VariableSourceKind
  sourceLabel: string
  summary: string
  extractor: string
  extractorKind: VariableExtractorKind
  bindingField: string
  alias?: string | null
  presentationUnit?: string | null
  visibility: VariableDefinitionVisibility
  parserName?: string | null
  lastObservedValue?: string | null
  lastObservedAtMs?: number | null
  status: UiDefinitionStatus
  updatedAtMs: number
}

export interface UiScriptsDefinitionModel {
  scripts: ScriptDefinition[]
  hooks: HookDefinition[]
  variables: VariableDefinition[]
}
