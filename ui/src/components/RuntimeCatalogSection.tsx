import { useMemo } from 'react'
import type { UiRuntimeCatalogSnapshot, UiRuntimeTelemetryCatalogField } from '../types'
import { RuntimeSectionHeader } from './runtimeUtils'

export function RuntimeCatalogSection({ runtimeCatalog }: { runtimeCatalog: UiRuntimeCatalogSnapshot }) {
  const telemetryGroups = useMemo(
    () => groupTelemetryFields(runtimeCatalog.telemetry.fields),
    [runtimeCatalog.telemetry.fields],
  )

  return (
    <section className="runtime-section runtime-catalog-section">
      <RuntimeSectionHeader title="Device catalog" description="Device-centered command and telemetry metadata for the active runtime" />
      <div className="runtime-summary-grid runtime-catalog-summary-grid">
        <article className="runtime-card runtime-device-card">
          <span className="mcp-label">Device</span>
          <strong>{runtimeCatalog.device.label}</strong>
          <small>{runtimeCatalog.device.detail}</small>
          <small>{runtimeCatalog.device.transportLabel} · {runtimeCatalog.device.status}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Commands</span>
          <strong>{runtimeCatalog.commands.length}</strong>
          <small>Host actions for the active device</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Telemetry</span>
          <strong>{runtimeCatalog.telemetry.fields.length}</strong>
          <small>{runtimeCatalog.telemetry.rateHz ? `${runtimeCatalog.telemetry.rateHz} Hz` : 'Schema pending'}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Parser</span>
          <strong>{runtimeCatalog.telemetry.parserName}</strong>
          <small>Shared device schema source</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Freshness</span>
          <strong>{runtimeCatalog.telemetry.lastSampleAtMs ? 'Live schema' : 'Idle catalog'}</strong>
          <small>Schema {runtimeCatalog.telemetry.lastSchemaAtMs ? 'seen' : 'pending'} · Sample {runtimeCatalog.telemetry.lastSampleAtMs ? 'seen' : 'pending'}</small>
        </article>
      </div>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Command catalog</strong>
          <small>Stable device actions for the command center</small>
        </div>
        <div className="runtime-catalog-list">
          {runtimeCatalog.commands.map((item) => (
            <article key={item.command} className="runtime-example-item">
              <strong>
                {item.command} · {item.label}
              </strong>
              <small>{item.description}</small>
              {item.payloadPreview ? <code>{item.payloadPreview}</code> : null}
            </article>
          ))}
        </div>
      </section>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Telemetry catalog</strong>
          <small>{runtimeCatalog.telemetry.sampleLen ?? '-'} bytes · grouped by readable sections</small>
        </div>
        <div className="runtime-telemetry-group-stack">
          {telemetryGroups.length > 0 ? (
            telemetryGroups.map((group) => <TelemetryGroupCard key={group.key} group={group} />)
          ) : (
            <div className="protocol-empty">Awaiting telemetry schema.</div>
          )}
        </div>
      </section>

      <details className="runtime-telemetry-disclosure">
        <summary>
          <strong>Raw schema order</strong>
          <small>Exact protocol field sequence for drill-down inspection</small>
        </summary>
        <div className="protocol-field-grid runtime-telemetry-raw-grid">
          {runtimeCatalog.telemetry.fields.map((field) => (
            <div key={`${field.index}-${field.name}`} className="protocol-field-pill">
              <strong>
                {field.index + 1}. {field.name}
              </strong>
              <small>
                {field.unit} · q{field.scaleQ}
              </small>
            </div>
          ))}
        </div>
      </details>
    </section>
  )
}

function TelemetryGroupCard({ group }: { group: TelemetryFieldGroup }) {
  return (
    <details className="runtime-telemetry-group" open={group.openByDefault}>
      <summary className="runtime-telemetry-group-summary">
        <div className="runtime-telemetry-group-copy">
          <strong>{group.label}</strong>
          <small>{group.summary}</small>
        </div>
        <span className="metric-chip">{group.fields.length} fields</span>
      </summary>
      <div className="protocol-field-grid runtime-telemetry-field-grid">
        {group.fields.map((field) => (
          <div key={`${field.index}-${field.name}`} className="protocol-field-pill runtime-telemetry-field-pill">
            <strong>
              {field.index + 1}. {field.name}
            </strong>
            <small>
              {field.unit} · q{field.scaleQ}
            </small>
          </div>
        ))}
      </div>
    </details>
  )
}

type TelemetryFieldGroup = {
  key: string
  label: string
  summary: string
  fields: UiRuntimeTelemetryCatalogField[]
  openByDefault: boolean
  order: number
}

function groupTelemetryFields(fields: UiRuntimeTelemetryCatalogField[]) {
  const prefixCounts = new Map<string, number>()

  for (const field of fields) {
    const prefix = field.name.toLowerCase().split(/[_\-.]/)[0]
    if (!prefix) continue
    prefixCounts.set(prefix, (prefixCounts.get(prefix) ?? 0) + 1)
  }

  const groups = new Map<string, TelemetryFieldGroup>()

  for (const field of fields) {
    const classification = classifyTelemetryField(field, prefixCounts)
    const existing = groups.get(classification.key)

    if (existing) {
      existing.fields.push(field)
      continue
    }

      groups.set(classification.key, {
        ...classification,
        fields: [field],
      } as TelemetryFieldGroup)
    }

  return [...groups.values()]
    .map((group) => ({
      ...group,
      fields: group.fields.sort((left, right) => left.index - right.index),
      summary: summarizeTelemetryGroup(group.label, group.fields),
    }))
    .sort((left, right) => left.order - right.order || left.label.localeCompare(right.label, undefined, { sensitivity: 'base' }))
}

function classifyTelemetryField(
  field: UiRuntimeTelemetryCatalogField,
  prefixCounts: Map<string, number>,
): Omit<TelemetryFieldGroup, 'fields' | 'summary'> {
  const normalized = field.name.toLowerCase()

  if (/^(acc|accel|accelerometer)/.test(normalized)) {
    return { key: 'acceleration', label: 'Acceleration', openByDefault: true, order: 0 }
  }

  if (/^(gyro|gy)/.test(normalized)) {
    return { key: 'gyroscope', label: 'Gyroscope', openByDefault: true, order: 1 }
  }

  if (/^(roll|pitch|yaw)/.test(normalized)) {
    return { key: 'attitude', label: 'Attitude', openByDefault: true, order: 2 }
  }

  if (/^(quat|qw|qx|qy|qz)/.test(normalized)) {
    return { key: 'quaternion', label: 'Quaternion', openByDefault: false, order: 3 }
  }

  const prefix = normalized.split(/[_\-.]/)[0]
  if (prefix && (prefixCounts.get(prefix) ?? 0) > 1) {
    return {
      key: `prefix:${prefix}`,
      label: toTitleCase(prefix),
      openByDefault: false,
      order: 10,
    }
  }

  return { key: 'other', label: 'Other telemetry', openByDefault: false, order: 20 }
}

function summarizeTelemetryGroup(label: string, fields: UiRuntimeTelemetryCatalogField[]) {
  const preview = fields.slice(0, 3).map((field) => field.name).join(' · ')
  if (!preview) {
    return label
  }

  return fields.length > 3 ? `${preview} · +${fields.length - 3} more` : preview
}

function toTitleCase(value: string) {
  return value
    .split('_')
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(' ')
}
