use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use tokio::sync::mpsc::UnboundedSender;

pub const NUM_BARS: usize = 128;
const FFT_SIZE: usize = 2048;
const SAMPLE_RATE: f32 = 48000.0;

pub fn launch(viz_buf: Arc<Mutex<VecDeque<f32>>>, tx: UnboundedSender<Vec<f32>>) {
    std::thread::spawn(move || {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let mut scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];

        let hann: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos()))
            .collect();

        // Peak follower: decays ~94% per second at 30 fps so the display
        // adapts to both quiet and loud passages in a few seconds.
        let mut peak: f32 = 1e-6;

        loop {
            std::thread::sleep(Duration::from_millis(33));

            let samples: Vec<f32> = {
                let mut buf = viz_buf.lock().unwrap();
                let n = buf.len().min(FFT_SIZE);
                let skip = buf.len().saturating_sub(FFT_SIZE);
                let v: Vec<f32> = buf.iter().skip(skip).copied().collect();
                buf.clear();
                // last n samples, zero-padded to FFT_SIZE on the left
                let mut out = vec![0.0f32; FFT_SIZE];
                let offset = FFT_SIZE - n;
                out[offset..].copy_from_slice(&v[v.len().saturating_sub(FFT_SIZE - offset)..]);
                out
            };

            let mut buf: Vec<Complex<f32>> = samples
                .iter()
                .zip(hann.iter())
                .map(|(&s, &w)| Complex { re: s * w, im: 0.0 })
                .collect();

            fft.process_with_scratch(&mut buf, &mut scratch);

            let mags: Vec<f32> = buf[..FFT_SIZE / 2]
                .iter()
                .map(|c| c.norm() / (FFT_SIZE as f32 / 4.0))
                .collect();

            // Update peak follower: rise instantly, decay slowly
            let frame_max = mags.iter().copied().fold(0.0f32, f32::max);
            peak = peak.max(frame_max) * 0.998;
            peak = peak.max(1e-6);

            // Normalize mags by running peak before bin grouping
            let normalized: Vec<f32> = mags.iter().map(|&m| m / peak).collect();

            let bars = compute_bars(&normalized);

            if tx.send(bars).is_err() {
                break;
            }
        }
    });
}

fn compute_bars(mags: &[f32]) -> Vec<f32> {
    let log_min = (20.0f32).ln();
    let log_max = (18000.0f32).ln();

    (0..NUM_BARS)
        .map(|i| {
            let f_low = (log_min + i as f32 * (log_max - log_min) / NUM_BARS as f32).exp();
            let f_high =
                (log_min + (i + 1) as f32 * (log_max - log_min) / NUM_BARS as f32).exp();

            let bin_low = ((f_low * FFT_SIZE as f32 / SAMPLE_RATE) as usize).max(1);
            let bin_high =
                (((f_high * FFT_SIZE as f32 / SAMPLE_RATE) as usize) + 1).min(mags.len());

            let val = if bin_low < bin_high {
                mags[bin_low..bin_high].iter().copied().fold(0.0f32, f32::max)
            } else {
                mags.get(bin_low).copied().unwrap_or(0.0)
            };

            // Soft compression via tanh: maps [0, ∞) → [0, 1)
            (val * 2.5).tanh()
        })
        .collect()
}
