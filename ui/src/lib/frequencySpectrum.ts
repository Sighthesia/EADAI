export type SpectrumSamplePoint = {
  timestampMs: number
  value: number
}

export type FrequencySpectrumBin = {
  frequencyHz: number
  amplitude: number
}

export type FrequencySpectrumResult = {
  bins: FrequencySpectrumBin[]
  sampleCount: number
  sampleRateHz: number | null
  peakFrequencyHz: number | null
  peakAmplitude: number | null
}

export function computeFrequencySpectrum(points: SpectrumSamplePoint[]): FrequencySpectrumResult {
  const ordered = points
    .filter((point) => Number.isFinite(point.timestampMs) && Number.isFinite(point.value))
    .slice()
    .sort((left, right) => left.timestampMs - right.timestampMs)

  if (ordered.length < 4) {
    return emptySpectrum(ordered.length)
  }

  const sampleRateHz = estimateSampleRateHz(ordered)
  if (!sampleRateHz || sampleRateHz <= 0) {
    return emptySpectrum(ordered.length)
  }

  const sampleCount = ordered.length
  const fftSize = nextPowerOfTwo(sampleCount)
  const mean = ordered.reduce((sum, point) => sum + point.value, 0) / sampleCount
  const re = new Array<number>(fftSize).fill(0)
  const im = new Array<number>(fftSize).fill(0)

  for (let index = 0; index < sampleCount; index += 1) {
    const value = ordered[index]!.value - mean
    const window = sampleCount > 1 ? 0.5 * (1 - Math.cos((2 * Math.PI * index) / (sampleCount - 1))) : 1
    re[index] = value * window
  }

  fft(re, im)

  const halfSize = Math.floor(fftSize / 2)
  const bins: FrequencySpectrumBin[] = []
  let peakFrequencyHz: number | null = null
  let peakAmplitude = Number.NEGATIVE_INFINITY
  const coherentGain = sampleCount > 1 ? 0.5 : 1

  for (let bin = 0; bin <= halfSize; bin += 1) {
    const rawMagnitude = Math.hypot(re[bin] ?? 0, im[bin] ?? 0)
    const isEdgeBin = bin === 0 || (fftSize % 2 === 0 && bin === halfSize)
    const amplitude = isEdgeBin ? rawMagnitude / (fftSize * coherentGain) : (2 * rawMagnitude) / (fftSize * coherentGain)
    const frequencyHz = (bin * sampleRateHz) / fftSize

    bins.push({ frequencyHz, amplitude })

    if (bin > 0 && amplitude > peakAmplitude && amplitude > 1e-12) {
      peakAmplitude = amplitude
      peakFrequencyHz = frequencyHz
    }
  }

  if (!Number.isFinite(peakAmplitude) || peakAmplitude < 0) {
    return {
      bins,
      sampleCount,
      sampleRateHz,
      peakFrequencyHz: null,
      peakAmplitude: null,
    }
  }

  return {
    bins,
    sampleCount,
    sampleRateHz,
    peakFrequencyHz,
    peakAmplitude,
  }
}

function emptySpectrum(sampleCount: number): FrequencySpectrumResult {
  return {
    bins: [],
    sampleCount,
    sampleRateHz: null,
    peakFrequencyHz: null,
    peakAmplitude: null,
  }
}

function estimateSampleRateHz(points: SpectrumSamplePoint[]) {
  const deltas: number[] = []
  for (let index = 1; index < points.length; index += 1) {
    const delta = points[index]!.timestampMs - points[index - 1]!.timestampMs
    if (delta > 0) {
      deltas.push(delta)
    }
  }

  if (deltas.length === 0) {
    return null
  }

  deltas.sort((left, right) => left - right)
  const midpoint = Math.floor(deltas.length / 2)
  const medianDeltaMs = deltas.length % 2 === 0 ? (deltas[midpoint - 1]! + deltas[midpoint]!) / 2 : deltas[midpoint]!

  return medianDeltaMs > 0 ? 1_000 / medianDeltaMs : null
}

function nextPowerOfTwo(value: number) {
  return 1 << Math.ceil(Math.log2(Math.max(1, value)))
}

function fft(re: number[], im: number[]) {
  const size = re.length
  const levels = Math.log2(size)
  if (!Number.isInteger(levels)) {
    throw new Error('FFT size must be a power of two.')
  }

  for (let index = 0; index < size; index += 1) {
    const target = reverseBits(index, levels)
    if (target > index) {
      ;[re[index], re[target]] = [re[target]!, re[index]!]
      ;[im[index], im[target]] = [im[target]!, im[index]!]
    }
  }

  for (let sizeStep = 2; sizeStep <= size; sizeStep <<= 1) {
    const halfStep = sizeStep >> 1
    const phaseStep = (-2 * Math.PI) / sizeStep

    for (let start = 0; start < size; start += sizeStep) {
      for (let offset = 0; offset < halfStep; offset += 1) {
        const evenIndex = start + offset
        const oddIndex = evenIndex + halfStep
        const phase = phaseStep * offset
        const cos = Math.cos(phase)
        const sin = Math.sin(phase)
        const oddRe = re[oddIndex]!
        const oddIm = im[oddIndex]!
        const tre = cos * oddRe - sin * oddIm
        const tim = cos * oddIm + sin * oddRe

        re[oddIndex] = re[evenIndex]! - tre
        im[oddIndex] = im[evenIndex]! - tim
        re[evenIndex] = re[evenIndex]! + tre
        im[evenIndex] = im[evenIndex]! + tim
      }
    }
  }
}

function reverseBits(value: number, bitCount: number) {
  let reversed = 0
  for (let index = 0; index < bitCount; index += 1) {
    reversed = (reversed << 1) | (value & 1)
    value >>>= 1
  }
  return reversed
}
