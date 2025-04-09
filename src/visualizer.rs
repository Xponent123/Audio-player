use egui::{Color32, Pos2, Rect, Vec2, Stroke};
use rustfft::{FftPlanner, num_complex::Complex};
use std::collections::VecDeque;
use egui::epaint::{CornerRadius, StrokeKind}; // <-- new import

// Constants for visualization
pub const SPECTRUM_BUFFER_SIZE: usize = 4096;  // Must be power of 2 for FFT
pub const SPECTRUM_BANDS: usize = 64;          // Number of frequency bands to display
pub const WAVEFORM_POINTS: usize = 1024;       // Number of points to display in waveform

pub struct AudioVisualizer {
    pub sample_buffer: VecDeque<f32>,
    pub spectrum_data: Vec<f32>,
    pub waveform_data: Vec<f32>,
    pub peak_levels: Vec<f32>,      // For smoother animation
    pub sample_rate: u32,
    pub fft_planner: FftPlanner<f32>,
    pub update_needed: bool,
    pub peak_hold_frames: Vec<u8>,  // For peak falloff
}

impl AudioVisualizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_buffer: VecDeque::with_capacity(SPECTRUM_BUFFER_SIZE),
            spectrum_data: vec![0.0; SPECTRUM_BANDS],
            waveform_data: vec![0.0; WAVEFORM_POINTS],
            peak_levels: vec![0.0; SPECTRUM_BANDS],
            sample_rate,
            fft_planner: FftPlanner::new(),
            update_needed: true,
            peak_hold_frames: vec![0; SPECTRUM_BANDS],
        }
    }

    pub fn add_sample(&mut self, sample: f32) {
        if self.sample_buffer.len() >= SPECTRUM_BUFFER_SIZE {
            self.sample_buffer.pop_front();
        }
        self.sample_buffer.push_back(sample);
        self.update_needed = true;
        
        // Update waveform display (downsampled)
        let waveform_idx = (self.sample_buffer.len() * WAVEFORM_POINTS / SPECTRUM_BUFFER_SIZE) % WAVEFORM_POINTS;
        if waveform_idx < self.waveform_data.len() {
            self.waveform_data[waveform_idx] = sample;
        }
    }

    pub fn analyze(&mut self) {
        if !self.update_needed || self.sample_buffer.len() < SPECTRUM_BUFFER_SIZE {
            return;
        }

        // Prepare FFT input
        let fft_input: Vec<Complex<f32>> = self.sample_buffer.iter()
            .enumerate()
            .map(|(i, &sample)| {
                // Apply Hann window
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / SPECTRUM_BUFFER_SIZE as f32).cos());
                Complex { re: sample * window, im: 0.0 }
            })
            .collect();

        // Create output buffer (in-place FFT)
        let mut fft_output = fft_input.clone();

        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(SPECTRUM_BUFFER_SIZE);
        fft.process(&mut fft_output);

        // Process FFT results
        let nyquist = self.sample_rate as f32 / 2.0;
        let bin_size = nyquist / (SPECTRUM_BUFFER_SIZE as f32 / 2.0);
        
        // Temporary buffer for new values
        let mut new_spectrum = vec![0.0; SPECTRUM_BANDS];

        // Process frequency bands
        for i in 0..SPECTRUM_BUFFER_SIZE/2 {
            let re = fft_output[i].re;
            let im = fft_output[i].im;
            let magnitude = (re * re + im * im).sqrt() as f32;
            
            // Convert to decibels (range approximately -80 to 0)
            let db = 20.0 * magnitude.log10().max(-80.0);
            // Normalize to 0.0-1.0 range
            let normalized = (db + 80.0) / 80.0;

            // Map to logarithmic frequency band
            let freq = i as f32 * bin_size;
            let band_index = if freq > 0.0 {
                let log_freq = freq.log10();
                let log_min = 20.0_f32.log10(); // 20 Hz
                let log_max = nyquist.log10();
                
                let normalized_log = (log_freq - log_min) / (log_max - log_min);
                (normalized_log * (SPECTRUM_BANDS as f32 - 1.0)) as usize
            } else {
                0
            };
            
            if band_index < SPECTRUM_BANDS {
                new_spectrum[band_index] = f32::max(new_spectrum[band_index], normalized);
            }
        }

        // Update spectrum with smoother transitions
        for i in 0..SPECTRUM_BANDS {
            // Smooth the transition to new values (70% old, 30% new)
            self.spectrum_data[i] = self.spectrum_data[i] * 0.7 + new_spectrum[i] * 0.3;
            
            // Handle peak levels with falloff
            if self.spectrum_data[i] > self.peak_levels[i] {
                self.peak_levels[i] = self.spectrum_data[i];
                self.peak_hold_frames[i] = 30; // Hold peak for 30 frames
            } else if self.peak_hold_frames[i] > 0 {
                self.peak_hold_frames[i] -= 1;
            } else {
                // Gradually reduce peak levels
                self.peak_levels[i] = (self.peak_levels[i] - 0.01).max(self.spectrum_data[i]);
            }
        }

        self.update_needed = false;
    }

    pub fn draw_spectrum(&self, ui: &egui::Ui, rect: Rect, theme: &super::theme::Theme) {
        let painter = ui.painter();
        
        let bar_count = self.spectrum_data.len();
        let bar_width = rect.width() / (bar_count as f32);
        let bar_spacing = bar_width * 0.1;
        let effective_bar_width = bar_width - bar_spacing;
        
        // Draw the bars with gradient colors
        for i in 0..bar_count {
            let value = self.spectrum_data[i];
            let peak = self.peak_levels[i];
            let x = rect.left() + (i as f32 * bar_width);
            let bar_height = value * rect.height();
            let peak_y = rect.bottom() - peak * rect.height();
            
            // Gradient color based on frequency and intensity
            let intensity_factor = 0.2 + value * 0.8; // Boost low values for visibility
            
            // Color gradient from blue (low freqs) to red (high freqs)
            let hue = 210.0 - (i as f32 / bar_count as f32) * 210.0;
            let saturation = 0.8;
            let value = 0.7 + 0.3 * intensity_factor;
            
            let (r, g, b) = hsv_to_rgb(hue, saturation, value);
            let color = Color32::from_rgb(r, g, b);
            
            // Draw main bar
            let bar_rect = Rect::from_min_size(
                Pos2::new(x + bar_spacing * 0.5, rect.bottom() - bar_height),
                Vec2::new(effective_bar_width, bar_height),
            );
            
            // Draw rounded bar with gradient
            painter.rect_filled(bar_rect, theme.corner_radius, color);
            
            // Draw peak marker
            painter.line_segment(
                [Pos2::new(x + bar_spacing * 0.5, peak_y), 
                 Pos2::new(x + bar_spacing * 0.5 + effective_bar_width, peak_y)],
                Stroke::new(2.0, Color32::WHITE)
            );
        }
        
        // Draw the frame:
        painter.rect(
            rect, 
            theme.corner_radius, 
            theme.panel_color, 
            Stroke::new(1.0, theme.inactive_color),
            StrokeKind::Middle   // explicitly supply a variant
        );
    }

    pub fn draw_waveform(&self, ui: &egui::Ui, rect: Rect, theme: &super::theme::Theme) {
        let painter = ui.painter();
        
        let point_count = self.waveform_data.len();
        let point_width = rect.width() / (point_count as f32);
        
        let baseline_y = rect.center().y;
        
        // Draw waveform as connected line segments
        let mut points = Vec::with_capacity(point_count);
        for i in 0..point_count {
            let x = rect.left() + (i as f32 * point_width);
            let sample = self.waveform_data[i].clamp(-1.0, 1.0);
            let y = baseline_y - sample * rect.height() * 0.4; // Scale to 40% of height
            
            points.push(Pos2::new(x, y));
        }
        
        if points.len() >= 2 {
            // Draw waveform with gradient
            for i in 0..points.len()-1 {
                let start = points[i];
                let end = points[i+1];
                
                // Calculate color based on amplitude
                let amplitude = ((self.waveform_data[i].abs() + self.waveform_data[i+1].abs()) / 2.0).clamp(0.0, 1.0);
                let intensity = 0.4 + amplitude * 0.6; // Boost low values
                
                let (r, g, b) = hsv_to_rgb(200.0, 0.7, intensity);
                let color = Color32::from_rgb(r, g, b);
                
                painter.line_segment([start, end], Stroke::new(2.0, color));
            }
        }
        
        // Draw the frame:
        painter.rect(
            rect, 
            theme.corner_radius, 
            theme.panel_color, 
            Stroke::new(1.0, theme.inactive_color),
            StrokeKind::Middle  // explicitly supply a variant
        );
    }
}

// Helper function to convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h % 360.0;
    let hi = (h / 60.0).floor() as i32 % 6;
    let f = h / 60.0 - (h / 60.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    
    let (r, g, b) = match hi {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    
    (
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
    )
}
