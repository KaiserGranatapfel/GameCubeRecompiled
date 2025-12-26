// DMA (Direct Memory Access) system
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct DmaSystem {
    channels: Vec<DmaChannel>,
}

pub struct DmaChannel {
    active: Arc<AtomicBool>,
    source: u32,
    destination: u32,
    length: u32,
    callback: Option<Box<dyn Fn() + Send + Sync>>,
}

impl DmaSystem {
    pub fn new() -> Self {
        Self {
            channels: (0..4).map(|_| DmaChannel {
                active: Arc::new(AtomicBool::new(false)),
                source: 0,
                destination: 0,
                length: 0,
                callback: None,
            }).collect(),
        }
    }
    
    pub fn start_transfer(
        &mut self,
        channel: usize,
        source: u32,
        destination: u32,
        length: u32,
    ) -> Result<()> {
        if channel >= self.channels.len() {
            anyhow::bail!("Invalid DMA channel: {}", channel);
        }
        
        let ch = &mut self.channels[channel];
        ch.source = source;
        ch.destination = destination;
        ch.length = length;
        ch.active.store(true, Ordering::SeqCst);
        
        Ok(())
    }
    
    pub fn is_active(&self, channel: usize) -> bool {
        if channel < self.channels.len() {
            self.channels[channel].active.load(Ordering::SeqCst)
        } else {
            false
        }
    }
    
    pub fn complete_transfer(&mut self, channel: usize) {
        if channel < self.channels.len() {
            self.channels[channel].active.store(false, Ordering::SeqCst);
            if let Some(callback) = &self.channels[channel].callback {
                callback();
            }
        }
    }
}

impl DmaChannel {
    pub fn set_callback<F: Fn() + Send + Sync + 'static>(&mut self, callback: F) {
        self.callback = Some(Box::new(callback));
    }
}

