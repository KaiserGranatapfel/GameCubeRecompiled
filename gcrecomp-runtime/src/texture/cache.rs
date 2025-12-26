// Texture cache
use image::RgbaImage;
use std::collections::HashMap;

pub struct TextureCache {
    cache: HashMap<String, RgbaImage>,
    max_size: usize,
    current_size: usize,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 512 * 1024 * 1024, // 512MB default
            current_size: 0,
        }
    }
    
    pub fn get(&self, key: &str) -> Option<&RgbaImage> {
        self.cache.get(key)
    }
    
    pub fn insert(&mut self, key: String, texture: RgbaImage) {
        let size = (texture.width() * texture.height() * 4) as usize;
        
        // Evict if needed (simple LRU - would need proper implementation)
        while self.current_size + size > self.max_size && !self.cache.is_empty() {
            if let Some((old_key, _)) = self.cache.iter().next() {
                let old_key = old_key.clone();
                if let Some(old_texture) = self.cache.remove(&old_key) {
                    let old_size = (old_texture.width() * old_texture.height() * 4) as usize;
                    self.current_size -= old_size;
                }
            } else {
                break;
            }
        }
        
        self.cache.insert(key, texture);
        self.current_size += size;
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size = 0;
    }
    
    pub fn set_max_size(&mut self, size: usize) {
        self.max_size = size;
        // Evict if over limit
        while self.current_size > self.max_size {
            if let Some((key, _)) = self.cache.iter().next() {
                let key = key.clone();
                if let Some(texture) = self.cache.remove(&key) {
                    let texture_size = (texture.width() * texture.height() * 4) as usize;
                    self.current_size -= texture_size;
                }
            } else {
                break;
            }
        }
    }
}

