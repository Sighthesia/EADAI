import type {
  HookDefinition,
  SerialBusEvent,
  UiScriptHookExample,
  UiScriptsDefinitionModel,
  VariableDefinition,
  VariableDefinitionVisibility,
  VariableEntry,
  VariableExtractorKind,
  VariableSourceKind,
} from '../types'
import { DEFINITION_SEED_TIMESTAMP_MS } from './constants'

export function createProtocolHookExamples(): UiScriptHookExample[] {
  return [
    {
      name: 'Schema observation hook',
      snippet: `onSchema((fields, rateHz, sampleLen) => {
  logger.debug('Signals schema', { rateHz, sampleLen, fields })
})`,
    },
    {
      name: 'Sample observation hook',
      snippet: `onSample((record) => {
  const { roll, pitch, yaw } = record
  logger.debug('Signals sample', { roll, pitch, yaw })
})`,
    },
  ]
}

function createProtocolHookDefinitions(): HookDefinition[] {
  return createProtocolHookExamples().map((example, index) => ({
    id: index === 0 ? 'schema-hook' : 'sample-hook',
    name: example.name,
    event: index === 0 ? 'schema' : 'sample',
    summary: index === 0
      ? 'Capture the published telemetry schema and keep it available to the Signals surface.'
      : 'Draft runtime extraction rules from incoming telemetry samples without owning the protocol.',
    source: example.snippet,
    enabled: true,
    status: 'seeded',
    updatedAtMs: DEFINITION_SEED_TIMESTAMP_MS,
  }))
}

function extractorKindForVariable(variable: VariableEntry): VariableExtractorKind {
  return variable.sourceKind === 'telemetry-sample' ? 'sample-field' : 'parser-field'
}

function sourceLabelForVariable(variable: VariableEntry) {
  if (variable.sourceKind === 'telemetry-sample') {
    return variable.parserName ?? 'telemetry sample'
  }

  return variable.parserName ? `${variable.parserName} text` : 'protocol text'
}

function bindingFieldForVariable(variable: VariableEntry) {
  if (variable.sourceKind === 'telemetry-sample') {
    return 'sample.value'
  }

  return variable.parserName ? 'parser.fields.value' : 'line.parser.fields.value'
}

function aliasForVariable(variable: VariableEntry) {
  return variable.name.includes('.') ? variable.name.split('.').slice(-1)[0] ?? null : null
}

function presentationUnitForVariable(variable: VariableEntry) {
  return variable.unit ?? null
}

function visibilityForVariable(variable: VariableEntry): VariableDefinitionVisibility {
  if (variable.sourceKind === 'telemetry-sample') {
    return 'both'
  }

  if (variable.latestTrigger) {
    return 'runtime'
  }

  return variable.sampleCount > 0 ? 'variables' : 'hidden'
}

export function createVariableDefinitions(variables: Record<string, VariableEntry>): VariableDefinition[] {
  return Object.values(variables)
    .sort((left, right) => right.updatedAtMs - left.updatedAtMs)
    .map((variable) => ({
      id: `variable:${variable.name}`,
      name: variable.name,
      deviceRef: variable.deviceRef ?? null,
      sourceKind: variable.sourceKind,
      sourceLabel: sourceLabelForVariable(variable),
      summary:
        variable.sourceKind === 'telemetry-sample'
          ? variable.deviceRef
            ? `Observed on ${variable.deviceRef} as a reusable telemetry sample target.`
            : 'Observed telemetry sample promoted into a reusable extraction seed.'
          : variable.parserName
            ? `Observed in ${variable.parserName} parser output and promoted into a reusable text extraction seed.`
            : 'Observed protocol text promoted into a reusable extraction seed.',
      extractor: variable.parserName ? `${variable.parserName}.${variable.name}` : variable.name,
      extractorKind: extractorKindForVariable(variable),
      bindingField: bindingFieldForVariable(variable),
      alias: aliasForVariable(variable),
      presentationUnit: presentationUnitForVariable(variable),
      visibility: visibilityForVariable(variable),
      parserName: variable.parserName ?? null,
      lastObservedValue: variable.currentValue,
      lastObservedAtMs: variable.updatedAtMs,
      status: variable.latestTrigger ? 'observed' : variable.sampleCount > 0 ? 'draft' : 'seeded',
      updatedAtMs: variable.updatedAtMs,
    }))
}

export function createScriptDefinitions(variables: Record<string, VariableEntry>): UiScriptsDefinitionModel {
  return {
    hooks: createProtocolHookDefinitions(),
    variables: createVariableDefinitions(variables),
  }
}

export function syncScriptDefinitions(current: UiScriptsDefinitionModel, variables: Record<string, VariableEntry>): UiScriptsDefinitionModel {
  const nextVariables = createVariableDefinitions(variables)
  const currentById = new Map(current.variables.map((definition) => [definition.id, definition]))
  let changed = nextVariables.length !== current.variables.length

  const mergedVariables = nextVariables.map((definition) => {
    const existing = currentById.get(definition.id)
    if (!existing) {
      changed = true
      return definition
    }

    const mergedDefinition = {
      ...definition,
      bindingField: existing.bindingField,
      alias: existing.alias ?? null,
      presentationUnit: existing.presentationUnit ?? null,
      visibility: existing.visibility,
      status: existing.status === 'observed' ? 'observed' : existing.status === 'draft' ? 'draft' : definition.status,
      lastObservedValue: existing.lastObservedValue ?? definition.lastObservedValue ?? null,
      lastObservedAtMs: existing.lastObservedAtMs ?? definition.lastObservedAtMs ?? null,
      updatedAtMs: existing.updatedAtMs,
    }

    if (!changed && !isSameVariableDefinition(existing, mergedDefinition)) {
      changed = true
    }

    return mergedDefinition
  })

  if (!changed) {
    return current
  }

  return {
    ...current,
    variables: mergedVariables,
  }
}

function isSameVariableDefinition(left: VariableDefinition, right: VariableDefinition) {
  return left.id === right.id &&
    left.name === right.name &&
    left.deviceRef === right.deviceRef &&
    left.sourceKind === right.sourceKind &&
    left.sourceLabel === right.sourceLabel &&
    left.summary === right.summary &&
    left.extractor === right.extractor &&
    left.extractorKind === right.extractorKind &&
    left.bindingField === right.bindingField &&
    left.alias === right.alias &&
    left.presentationUnit === right.presentationUnit &&
    left.visibility === right.visibility &&
    left.parserName === right.parserName &&
    left.lastObservedValue === right.lastObservedValue &&
    left.lastObservedAtMs === right.lastObservedAtMs &&
    left.status === right.status &&
    left.updatedAtMs === right.updatedAtMs
}
