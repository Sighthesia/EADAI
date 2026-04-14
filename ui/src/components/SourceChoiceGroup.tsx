type SourceChoice<T extends string> = {
  value: T
  label: string
  description: string
}

type SourceChoiceGroupProps<T extends string> = {
  ariaLabel: string
  className?: string
  value: T
  options: Array<SourceChoice<T>>
  onChange: (value: T) => void
}

export function SourceChoiceGroup<T extends string>({
  ariaLabel,
  className,
  value,
  options,
  onChange,
}: SourceChoiceGroupProps<T>) {
  return (
    <div className={className ? `source-switch ${className}` : 'source-switch'} role="tablist" aria-label={ariaLabel}>
      {options.map((option) => {
        const active = value === option.value
        return (
          <button
            key={option.value}
            type="button"
            className={`source-option ${active ? 'active' : ''}`}
            onClick={() => onChange(option.value)}
          >
            <strong>{option.label}</strong>
            <small>{option.description}</small>
          </button>
        )
      })}
    </div>
  )
}
