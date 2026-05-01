/// Bitmap codec for telemetry sample compression.
///
/// This module handles encoding and decoding of telemetry samples
/// using bitmap-based compression for unchanged fields.
use super::frame::*;

/// State tracker for bitmap-based sample compression.
pub struct BitmapCodec {
    /// Variable descriptors from the catalog.
    variables: Vec<VariableDescriptor>,
    /// Previous sample values (indexed by variable order).
    previous_values: Vec<Option<Vec<u8>>>,
    /// Total size of all variable values in bytes.
    total_value_size: usize,
}

impl BitmapCodec {
    /// Create a new bitmap codec from variable descriptors.
    pub fn new(variables: Vec<VariableDescriptor>) -> Self {
        let total_value_size: usize = variables.iter().map(|v| v.value_type.byte_size()).sum();
        let previous_values = vec![None; variables.len()];

        Self {
            variables,
            previous_values,
            total_value_size,
        }
    }

    /// Get the number of variables.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Get the total value size in bytes.
    pub fn total_value_size(&self) -> usize {
        self.total_value_size
    }

    /// Encode a sample frame with bitmap compression.
    ///
    /// Returns the compressed sample and the number of unchanged fields.
    /// Returns an error if `values` length does not match the expected total value size.
    pub fn encode(&mut self, values: &[u8], seq: u32) -> Result<(TelemetrySample, usize), String> {
        if values.len() != self.total_value_size {
            return Err(format!(
                "values length mismatch: expected {}, got {}",
                self.total_value_size,
                values.len()
            ));
        }

        let var_count = self.variables.len();
        let bitmap_byte_count = var_count.div_ceil(8);
        let mut changed_bitmap = vec![0u8; bitmap_byte_count];
        let mut compressed_values = Vec::new();
        let mut unchanged_count = 0;
        let mut value_offset = 0;

        for (i, var) in self.variables.iter().enumerate() {
            let var_size = var.value_type.byte_size();
            let current_value = &values[value_offset..value_offset + var_size];

            let changed = match &self.previous_values[i] {
                Some(prev) => prev != current_value,
                None => true,
            };

            if changed {
                // Set bit in bitmap
                changed_bitmap[i / 8] |= 1 << (i % 8);
                compressed_values.extend_from_slice(current_value);
            } else {
                unchanged_count += 1;
            }

            // Update previous values
            self.previous_values[i] = Some(current_value.to_vec());
            value_offset += var_size;
        }

        Ok((
            TelemetrySample {
                seq,
                changed_bitmap,
                values: compressed_values,
            },
            unchanged_count,
        ))
    }

    /// Decode a sample frame, filling in unchanged values from previous state.
    ///
    /// Returns the full sample values.
    pub fn decode(&self, sample: &TelemetrySample) -> Result<Vec<u8>, String> {
        let var_count = self.variables.len();
        let bitmap_byte_count = var_count.div_ceil(8);

        if sample.changed_bitmap.len() < bitmap_byte_count {
            return Err("bitmap too short".to_string());
        }

        let mut full_values = Vec::with_capacity(self.total_value_size);
        let mut compressed_offset = 0;

        for (i, var) in self.variables.iter().enumerate() {
            let var_size = var.value_type.byte_size();
            let bit = (sample.changed_bitmap[i / 8] >> (i % 8)) & 1;

            if bit == 1 {
                // Changed value - read from compressed data
                if compressed_offset + var_size > sample.values.len() {
                    return Err("compressed values too short".to_string());
                }
                full_values.extend_from_slice(
                    &sample.values[compressed_offset..compressed_offset + var_size],
                );
                compressed_offset += var_size;
            } else {
                // Unchanged value - use previous
                match &self.previous_values[i] {
                    Some(prev) => {
                        if prev.len() != var_size {
                            return Err(format!("previous value size mismatch for variable {}", i));
                        }
                        full_values.extend_from_slice(prev);
                    }
                    None => {
                        return Err(format!("no previous value for unchanged variable {}", i));
                    }
                }
            }
        }

        Ok(full_values)
    }

    /// Decode a sample and update the internal state.
    ///
    /// Returns the full sample values.
    pub fn decode_and_update(&mut self, sample: &TelemetrySample) -> Result<Vec<u8>, String> {
        let full_values = self.decode(sample)?;

        // Update previous values
        let mut value_offset = 0;
        for (i, var) in self.variables.iter().enumerate() {
            let var_size = var.value_type.byte_size();
            self.previous_values[i] =
                Some(full_values[value_offset..value_offset + var_size].to_vec());
            value_offset += var_size;
        }

        Ok(full_values)
    }

    /// Reset the codec state (clear previous values).
    pub fn reset(&mut self) {
        self.previous_values = vec![None; self.variables.len()];
    }
}

/// Extract a typed value from a byte slice at the given offset.
pub fn extract_value(data: &[u8], offset: usize, value_type: ValueType) -> Result<f64, String> {
    match value_type {
        ValueType::U8 => {
            if data.len() < offset + 1 {
                return Err("buffer too short for u8".to_string());
            }
            Ok(data[offset] as f64)
        }
        ValueType::I8 => {
            if data.len() < offset + 1 {
                return Err("buffer too short for i8".to_string());
            }
            Ok(data[offset] as i8 as f64)
        }
        ValueType::U16 => {
            if data.len() < offset + 2 {
                return Err("buffer too short for u16".to_string());
            }
            Ok(u16::from_le_bytes([data[offset], data[offset + 1]]) as f64)
        }
        ValueType::I16 => {
            if data.len() < offset + 2 {
                return Err("buffer too short for i16".to_string());
            }
            Ok(i16::from_le_bytes([data[offset], data[offset + 1]]) as f64)
        }
        ValueType::U32 => {
            if data.len() < offset + 4 {
                return Err("buffer too short for u32".to_string());
            }
            Ok(u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as f64)
        }
        ValueType::I32 => {
            if data.len() < offset + 4 {
                return Err("buffer too short for i32".to_string());
            }
            Ok(i32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as f64)
        }
        ValueType::F32 => {
            if data.len() < offset + 4 {
                return Err("buffer too short for f32".to_string());
            }
            Ok(f32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_variables() -> Vec<VariableDescriptor> {
        vec![
            VariableDescriptor {
                name: "acc_x".to_string(),
                order: 0,
                unit: "m/s^2".to_string(),
                adjustable: false,
                value_type: ValueType::I16,
            },
            VariableDescriptor {
                name: "acc_y".to_string(),
                order: 1,
                unit: "m/s^2".to_string(),
                adjustable: false,
                value_type: ValueType::I16,
            },
            VariableDescriptor {
                name: "gain".to_string(),
                order: 2,
                unit: "".to_string(),
                adjustable: true,
                value_type: ValueType::F32,
            },
        ]
    }

    fn make_test_values_1() -> Vec<u8> {
        let mut values = Vec::new();
        values.extend_from_slice(&100i16.to_le_bytes());
        values.extend_from_slice(&200i16.to_le_bytes());
        values.extend_from_slice(&1.5f32.to_le_bytes());
        values
    }

    fn make_test_values_2() -> Vec<u8> {
        let mut values = Vec::new();
        values.extend_from_slice(&150i16.to_le_bytes());
        values.extend_from_slice(&200i16.to_le_bytes());
        values.extend_from_slice(&2.0f32.to_le_bytes());
        values
    }

    #[test]
    fn test_first_sample_all_changed() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values = make_test_values_1();

        let (sample, unchanged) = codec.encode(&values, 1).unwrap();
        assert_eq!(unchanged, 0);
        assert_eq!(sample.seq, 1);
        // All 3 bits should be set
        assert_eq!(sample.changed_bitmap[0], 0b00000111);
        assert_eq!(sample.values.len(), 8); // 2 + 2 + 4 bytes
    }

    #[test]
    fn test_second_sample_partial_changed() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values1 = make_test_values_1();
        codec.encode(&values1, 1).unwrap();

        // Change only acc_x and gain
        let values2 = make_test_values_2();
        let (sample, unchanged) = codec.encode(&values2, 2).unwrap();
        assert_eq!(unchanged, 1); // acc_y unchanged
        assert_eq!(sample.changed_bitmap[0], 0b00000101); // bits 0 and 2 set
        assert_eq!(sample.values.len(), 6); // 2 + 4 bytes (acc_y skipped)
    }

    #[test]
    fn test_decode_fills_unchanged() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values1 = make_test_values_1();
        codec.encode(&values1, 1).unwrap();

        let values2 = make_test_values_2();
        let (sample, _) = codec.encode(&values2, 2).unwrap();

        let decoded = codec.decode(&sample).unwrap();
        assert_eq!(decoded.len(), 8);
        assert_eq!(i16::from_le_bytes([decoded[0], decoded[1]]), 150);
        assert_eq!(i16::from_le_bytes([decoded[2], decoded[3]]), 200);
        assert_eq!(
            f32::from_le_bytes([decoded[4], decoded[5], decoded[6], decoded[7]]),
            2.0
        );
    }

    #[test]
    fn test_decode_and_update() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values1 = make_test_values_1();
        let (sample1, _) = codec.encode(&values1, 1).unwrap();

        let decoded1 = codec.decode_and_update(&sample1).unwrap();
        assert_eq!(decoded1, values1);

        // Now encode with no changes
        let (sample2, unchanged) = codec.encode(&values1, 2).unwrap();
        assert_eq!(unchanged, 3);
        assert_eq!(sample2.changed_bitmap[0], 0);
        assert!(sample2.values.is_empty());

        // Decode should still work
        let decoded2 = codec.decode_and_update(&sample2).unwrap();
        assert_eq!(decoded2, values1);
    }

    #[test]
    fn test_reset() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values = make_test_values_1();
        codec.encode(&values, 1).unwrap();

        codec.reset();

        // After reset, first encode should have all changed
        let (sample, unchanged) = codec.encode(&values, 2).unwrap();
        assert_eq!(unchanged, 0);
        assert_eq!(sample.changed_bitmap[0], 0b00000111);
    }

    #[test]
    fn test_encode_wrong_length_returns_error() {
        let mut codec = BitmapCodec::new(sample_variables());
        let values = vec![0u8; 4]; // Wrong length (expected 8)
        assert!(codec.encode(&values, 1).is_err());
    }

    #[test]
    fn test_extract_value() {
        let data = vec![0x64, 0x00, 0xC8, 0x00, 0x00, 0x00, 0xC0, 0x3F];
        assert_eq!(extract_value(&data, 0, ValueType::U8).unwrap(), 100.0);
        assert_eq!(extract_value(&data, 0, ValueType::I16).unwrap(), 100.0);
        assert_eq!(extract_value(&data, 2, ValueType::I16).unwrap(), 200.0);
        assert!((extract_value(&data, 4, ValueType::F32).unwrap() - 1.5).abs() < 0.001);
    }
}
