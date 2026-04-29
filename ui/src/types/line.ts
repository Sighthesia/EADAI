export type UiLineDirection = 'rx' | 'tx'

export interface UiLinePayload {
  direction: UiLineDirection
  text: string
  rawLength: number
  raw: number[]
}

export interface UiParserMeta {
  parserName?: string | null
  status: 'unparsed' | 'parsed' | 'malformed'
  fields: Record<string, string>
}
