// Minimal dev-time logger wrapper. Use this instead of console.log so
// we can easily control verbosity and replace with a real logger later.
export const logger = {
  debug: (...args: unknown[]) => {
    // Only emit in dev builds to avoid noisy production logs.
    // Keep implementation minimal to avoid adding dependencies.
    // eslint-disable-next-line no-console
    if (import.meta.env && import.meta.env.DEV) console.debug(...args)
  },
}
