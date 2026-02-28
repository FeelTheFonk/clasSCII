//! Mel-Frequency Cepstral Coefficients (MFCC) extraction.
//!
//! 26 triangular Mel-spaced filters (300 Hz – 8000 Hz), DCT-II for 5 coefficients.
//! All buffers pre-allocated, zero runtime allocation.

/// Mel filterbank for MFCC extraction.
pub struct MelFilterbank {
    /// Pre-computed filter descriptors (start_bin, center_bin, end_bin).
    filters: [(usize, usize, usize); 26],
    /// Pre-computed filter weights per bin for each filter.
    weights: Vec<Vec<f32>>,
    /// Working buffers.
    mel_energies: [f32; 26],
    log_energies: [f32; 26],
}

impl MelFilterbank {
    /// Create a new filterbank for the given FFT size and sample rate.
    #[must_use]
    pub fn new(fft_size: usize, sample_rate: u32) -> Self {
        let num_filters = 26;
        let mel_low = hz_to_mel(300.0);
        let mel_high = hz_to_mel(8000.0_f32.min(sample_rate as f32 / 2.0));

        let bin_hz = sample_rate as f32 / fft_size as f32;
        let num_bins = fft_size / 2 + 1;

        // Compute mel center points
        let mut mel_points = [0.0f32; 28]; // 26 filters + 2 endpoints
        for (i, mp) in mel_points.iter_mut().enumerate() {
            *mp = mel_low + (mel_high - mel_low) * i as f32 / 27.0;
        }

        let mut filters = [(0usize, 0usize, 0usize); 26];
        let mut weights = Vec::with_capacity(num_filters);

        for f in 0..num_filters {
            let start_hz = mel_to_hz(mel_points[f]);
            let center_hz = mel_to_hz(mel_points[f + 1]);
            let end_hz = mel_to_hz(mel_points[f + 2]);

            let start_bin = (start_hz / bin_hz) as usize;
            let center_bin = (center_hz / bin_hz) as usize;
            let end_bin = ((end_hz / bin_hz) as usize).min(num_bins.saturating_sub(1));

            filters[f] = (start_bin, center_bin, end_bin);

            let mut w = Vec::with_capacity(end_bin.saturating_sub(start_bin) + 1);
            for bin in start_bin..=end_bin {
                let freq = bin as f32 * bin_hz;
                let weight = if freq < center_hz {
                    if (center_hz - start_hz).abs() < f32::EPSILON {
                        1.0
                    } else {
                        (freq - start_hz) / (center_hz - start_hz)
                    }
                } else if (end_hz - center_hz).abs() < f32::EPSILON {
                    1.0
                } else {
                    (end_hz - freq) / (end_hz - center_hz)
                };
                w.push(weight.max(0.0));
            }
            weights.push(w);
        }

        Self {
            filters,
            weights,
            mel_energies: [0.0; 26],
            log_energies: [0.0; 26],
        }
    }

    /// Compute first 5 MFCC coefficients from a magnitude spectrum.
    pub fn compute(&mut self, spectrum: &[f32]) -> [f32; 5] {
        // Apply mel filterbank
        for (f, (start, _, end)) in self.filters.iter().enumerate() {
            let mut energy = 0.0f32;
            for (i, bin) in (*start..=(*end).min(spectrum.len().saturating_sub(1))).enumerate() {
                if let Some(&w) = self.weights[f].get(i) {
                    energy += spectrum[bin] * w;
                }
            }
            self.mel_energies[f] = energy;
        }

        // Log compression
        for i in 0..26 {
            self.log_energies[i] = (self.mel_energies[i] + 1e-10).ln();
        }

        // DCT-II: extract first 5 coefficients
        let mut mfcc = [0.0f32; 5];
        for (k, coeff) in mfcc.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for n in 0..26 {
                sum += self.log_energies[n]
                    * (std::f32::consts::PI * (k as f32) * (n as f32 + 0.5) / 26.0).cos();
            }
            *coeff = sum;
        }

        mfcc
    }
}

/// Hz to Mel scale conversion.
#[inline]
fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// Mel to Hz conversion.
#[inline]
fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}
