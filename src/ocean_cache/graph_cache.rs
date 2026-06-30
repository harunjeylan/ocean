use std::sync::Mutex;

use crate::ocean_cache::lru::LruCache;
use crate::ocean_storage::graph_store::{Edge, Node};

pub struct GraphCache {
    l1: Mutex<LruCache<String, Vec<(Node, Edge)>>>,
}

impl GraphCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            l1: Mutex::new(LruCache::new(capacity)),
        }
    }

    pub fn get(&self, node_id: &str) -> Option<Vec<(Node, Edge)>> {
        let mut l1 = self.l1.lock().ok()?;
        l1.get(&node_id.to_string()).cloned()
    }

    pub fn set(&self, node_id: String, neighbors: Vec<(Node, Edge)>) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.put(node_id, neighbors);
        }
    }

    pub fn invalidate_node(&self, node_id: &str) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.remove(&node_id.to_string());
        }
    }

    pub fn invalidate_all(&self) {
        if let Ok(mut l1) = self.l1.lock() {
            l1.clear();
        }
    }
}
