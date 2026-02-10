/// Audio mixer — combines DSP voices into stereo output.
pub struct AudioMixer {
    pub master_volume: f32,
    pub sample_rate: u32,
    buffer: Vec<[f32; 2]>, // Stereo samples
    buffer_pos: usize,
}

impl AudioMixer {
    const BUFFER_SIZE: usize = 4096;

    pub fn new(sample_rate: u32) -> Self {
        Self {
            master_volume: 1.0,
            sample_rate,
            buffer: vec![[0.0; 2]; Self::BUFFER_SIZE],
            buffer_pos: 0,
        }
    }

    /// Mix a mono voice into the stereo buffer with volume panning.
    pub fn mix_voice(&mut self, samples: &[i16], volume_left: f32, volume_right: f32) {
        for &sample in samples {
            if self.buffer_pos >= self.buffer.len() {
                break;
            }
            let s = sample as f32 / 32768.0;
            self.buffer[self.buffer_pos][0] += s * volume_left;
            self.buffer[self.buffer_pos][1] += s * volume_right;
            self.buffer_pos += 1;
        }
    }

    /// Mix raw PCM stereo data (interleaved i16) into the buffer.
    pub fn mix_stereo_pcm(&mut self, data: &[i16]) {
        for chunk in data.chunks(2) {
            if chunk.len() < 2 || self.buffer_pos >= self.buffer.len() {
                break;
            }
            let left = chunk[0] as f32 / 32768.0;
            let right = chunk[1] as f32 / 32768.0;
            self.buffer[self.buffer_pos][0] += left;
            self.buffer[self.buffer_pos][1] += right;
            self.buffer_pos += 1;
        }
    }

    /// Finalize the current buffer: apply master volume, clamp, and return.
    pub fn finalize(&mut self) -> Vec<f32> {
        let mut output = Vec::with_capacity(self.buffer_pos * 2);
        for i in 0..self.buffer_pos {
            let left = (self.buffer[i][0] * self.master_volume).clamp(-1.0, 1.0);
            let right = (self.buffer[i][1] * self.master_volume).clamp(-1.0, 1.0);
            output.push(left);
            output.push(right);
        }
        // Reset buffer for next frame
        self.clear();
        output
    }

    /// Pull exactly `count` interleaved stereo samples for the audio output thread.
    pub fn pull_samples(&mut self, count: usize) -> Vec<f32> {
        let available = self.buffer_pos.min(count);
        let mut output = Vec::with_capacity(available * 2);
        for i in 0..available {
            let left = (self.buffer[i][0] * self.master_volume).clamp(-1.0, 1.0);
            let right = (self.buffer[i][1] * self.master_volume).clamp(-1.0, 1.0);
            output.push(left);
            output.push(right);
        }
        // Shift remaining samples to start
        if available < self.buffer_pos {
            self.buffer.copy_within(available..self.buffer_pos, 0);
        }
        self.buffer_pos -= available;
        output
    }

    pub fn clear(&mut self) {
        for sample in &mut self.buffer {
            *sample = [0.0; 2];
        }
        self.buffer_pos = 0;
    }

    /// Resample from source rate to destination rate using linear interpolation.
    pub fn resample(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
        if src_rate == dst_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let ratio = src_rate as f64 / dst_rate as f64;
        let output_len = (samples.len() as f64 / ratio) as usize;
        let mut output = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let frac = src_pos - idx as f64;
            let a = samples.get(idx).copied().unwrap_or(0.0);
            let b = samples.get(idx + 1).copied().unwrap_or(a);
            output.push(a + (b - a) * frac as f32);
        }
        output
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(48000)
    }
}
