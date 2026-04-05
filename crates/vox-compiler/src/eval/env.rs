use std::collections::HashMap;
use super::value::VoxValue;

#[derive(Debug, Clone)]
pub struct Scope {
    frames: Vec<HashMap<String, VoxValue>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            frames: vec![HashMap::new()],
        }
    }

    pub fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    pub fn get(&self, name: &str) -> Option<&VoxValue> {
        for frame in self.frames.iter().rev() {
            if let Some(val) = frame.get(name) {
                return Some(val);
            }
        }
        None
    }

    pub fn set(&mut self, name: String, value: VoxValue) {
        if let Some(frame) = self.frames.last_mut() {
            frame.insert(name, value);
        }
    }

    pub fn set_mut(&mut self, name: &str, value: VoxValue) -> bool {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }
}
