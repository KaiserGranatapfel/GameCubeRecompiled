//! AI (Audio Interface) implementation
//!
//! The AI handles audio stream output to the system speakers.
//! It manages sample rates, buffers, and audio streaming.

use anyhow::Result;
use std::sync::{Arc, Mutex, mpsc};

/// Audio Interface state
#[derive(Debug)]
pub struct AudioInterface {
    /// Sample rate (Hz)
    sample_rate: u32,
    /// Audio buffer
    buffer: Vec<i16>,
    /// Buffer size in samples
    buffer_size: usize,
    /// Initialized flag
    initialized: bool,
    /// Audio stream sender (for sending samples to audio output)
    stream_sender: Option<mpsc::Sender<Vec<i16>>>,
    /// Audio output thread handle
    _audio_thread: Option<std::thread::JoinHandle<()>>,
}

impl AudioInterface {
    /// Create a new Audio Interface
    pub fn new() -> Self {
        Self {
            sample_rate: 48000, // Default 48kHz
            buffer: Vec::new(),
            buffer_size: 4096, // 4KB buffer
            initialized: false,
            stream_sender: None,
            _audio_thread: None,
        }
    }

    /// Initialize the Audio Interface
    pub fn init(&mut self) -> Result<()> {
        log::info!("AI initialized");
        
        // Initialize audio output using cpal
        let (sender, receiver) = mpsc::channel::<Vec<i16>>();
        self.stream_sender = Some(sender);
        
        // Start audio output thread
        let sample_rate = self.sample_rate;
        let audio_thread = std::thread::spawn(move || {
            if let Err(e) = Self::audio_output_thread(receiver, sample_rate) {
                log::error!("Audio output thread error: {}", e);
            }
        });
        self._audio_thread = Some(audio_thread);
        
        self.initialized = true;
        self.buffer.clear();
        Ok(())
    }

    /// Audio output thread (runs in background)
    fn audio_output_thread(receiver: mpsc::Receiver<Vec<i16>>, sample_rate: u32) -> Result<()> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
        
        let config = device.default_output_config()?;
        log::info!("Audio device: {}, sample rate: {}", device.name()?, config.sample_rate().0);
        
        // Create audio stream
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &config.into(), receiver, sample_rate)?
            }
            cpal::SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &config.into(), receiver, sample_rate)?
            }
            cpal::SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &config.into(), receiver, sample_rate)?
            }
            _ => {
                anyhow::bail!("Unsupported sample format");
            }
        };
        
        stream.play()?;
        
        // Keep thread alive
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if receiver.try_recv().is_err() && receiver.try_iter().next().is_none() {
                // No more data and channel is closed
                break;
            }
        }
        
        Ok(())
    }

    /// Build audio stream for a specific sample type
    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        receiver: mpsc::Receiver<Vec<i16>>,
        _sample_rate: u32,
    ) -> Result<cpal::Stream>
    where
        T: cpal::Sample + From<i16>,
    {
        use cpal::traits::{DeviceTrait, StreamTrait};
        
        let mut sample_buffer: Vec<T> = Vec::new();
        let mut buffer_pos = 0;
        
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // Fill output buffer with samples from receiver
                for sample in data.iter_mut() {
                    if buffer_pos >= sample_buffer.len() {
                        // Try to get more samples from receiver
                        if let Ok(new_samples) = receiver.try_recv() {
                            sample_buffer = new_samples.into_iter().map(|s| T::from(s)).collect();
                            buffer_pos = 0;
                        } else {
                            // No samples available, output silence
                            *sample = T::from(0i16);
                            continue;
                        }
                    }
                    
                    if buffer_pos < sample_buffer.len() {
                        *sample = sample_buffer[buffer_pos];
                        buffer_pos += 1;
                    } else {
                        *sample = T::from(0i16);
                    }
                }
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )?;
        
        Ok(stream)
    }

    /// Set stream sample rate
    pub fn set_stream_sample_rate(&mut self, rate: u32) -> Result<()> {
        self.sample_rate = rate;
        log::info!("AI sample rate set to {} Hz", rate);
        Ok(())
    }

    /// Get current sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Submit audio samples for playback
    ///
    /// # Arguments
    /// * `samples` - Stereo audio samples (interleaved L/R)
    pub fn submit_samples(&mut self, samples: &[i16]) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("AI not initialized");
        }

        // Append samples to buffer
        self.buffer.extend_from_slice(samples);

        // If buffer is full, send to audio output
        if self.buffer.len() >= self.buffer_size * 2 {
            if let Some(ref sender) = self.stream_sender {
                // Send buffer to audio output thread
                let buffer_to_send = self.buffer.clone();
                if sender.send(buffer_to_send).is_err() {
                    log::warn!("Failed to send audio samples to output thread");
                }
            }
            self.buffer.clear();
        }

        Ok(())
    }

    /// Get audio buffer
    pub fn buffer(&self) -> &[i16] {
        &self.buffer
    }

    /// Clear audio buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for AudioInterface {
    fn default() -> Self {
        Self::new()
    }
}

