// Texture cache with LRU eviction
use image::RgbaImage;
use std::collections::{HashMap, VecDeque};

pub struct TextureCache {
    cache: HashMap<String, RgbaImage>,
    /// Access-order tracker: most-recently-used at the back, LRU at front.
    access_order: VecDeque<String>,
    max_size: usize,
    current_size: usize,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            access_order: VecDeque::new(),
            max_size: 512 * 1024 * 1024, // 512MB default
            current_size: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&RgbaImage> {
        if self.cache.contains_key(key) {
            // Move to back (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push_back(key.to_string());
            self.cache.get(key)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, texture: RgbaImage) {
        let size = (texture.width() * texture.height() * 4) as usize;

        // If key already exists, remove old entry first
        if let Some(old) = self.cache.remove(&key) {
            let old_size = (old.width() * old.height() * 4) as usize;
            self.current_size -= old_size;
            self.access_order.retain(|k| k != &key);
        }

        // Evict LRU entries until we have room
        while self.current_size + size > self.max_size && !self.access_order.is_empty() {
            if let Some(evict_key) = self.access_order.pop_front() {
                if let Some(old_texture) = self.cache.remove(&evict_key) {
                    let old_size = (old_texture.width() * old_texture.height() * 4) as usize;
                    self.current_size -= old_size;
                }
            }
        }

        self.cache.insert(key.clone(), texture);
        self.access_order.push_back(key);
        self.current_size += size;
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
        self.current_size = 0;
    }

    pub fn set_max_size(&mut self, size: usize) {
        self.max_size = size;
        while self.current_size > self.max_size {
            if let Some(evict_key) = self.access_order.pop_front() {
                if let Some(texture) = self.cache.remove(&evict_key) {
                    let texture_size = (texture.width() * texture.height() * 4) as usize;
                    self.current_size -= texture_size;
                }
            } else {
                break;
            }
        }
    }
}
