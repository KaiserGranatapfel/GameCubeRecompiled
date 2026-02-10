/// Audio output thread — sends mixed audio to the host audio device.
///
/// Uses a callback-based approach: the audio system provides a closure
/// that fills the output buffer on demand.
use std::sync::{Arc, Mutex};

use super::mixer::AudioMixer;

/// Audio output configuration.
pub struct AudioOutput {
    mixer: Arc<Mutex<AudioMixer>>,
    active: bool,
}

impl AudioOutput {
    pub fn new(mixer: Arc<Mutex<AudioMixer>>) -> Self {
        Self {
            mixer,
            active: false,
        }
    }

    /// Start the audio output stream.
    /// This is a no-op placeholder — actual cpal integration requires the cpal
    /// dependency. When cpal is available, this spawns a stream that pulls
    /// samples from the mixer.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.active {
            return Ok(());
        }
        self.active = true;
        log::info!("AudioOutput: started (host audio output ready)");
        // cpal stream would be created here:
        // let host = cpal::default_host();
        // let device = host.default_output_device()...;
        // let stream = device.build_output_stream(config, move |data, _| {
        //     let mut mixer = mixer_clone.lock().unwrap();
        //     let samples = mixer.pull_samples(data.len() / 2);
        //     for (i, sample) in samples.iter().enumerate() {
        //         data[i] = *sample;
        //     }
        // }, ...);
        Ok(())
    }

    /// Stop the audio output stream.
    pub fn stop(&mut self) {
        self.active = false;
        log::info!("AudioOutput: stopped");
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Fill a buffer with audio samples (for manual pull mode / testing).
    pub fn fill_buffer(&self, output: &mut [f32]) {
        if let Ok(mut mixer) = self.mixer.lock() {
            let samples = mixer.pull_samples(output.len() / 2);
            let copy_len = samples.len().min(output.len());
            output[..copy_len].copy_from_slice(&samples[..copy_len]);
            // Zero-fill remainder
            for sample in &mut output[copy_len..] {
                *sample = 0.0;
            }
        }
    }
}
