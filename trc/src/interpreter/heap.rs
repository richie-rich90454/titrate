// Phase 4: Memory simulation for the Titrate interpreter
// Precision in every step – richie-rich90454, 2026

use super::Value;

#[allow(dead_code)]
pub struct Memory {
    pub heap: Vec<Value>,
    #[allow(dead_code)]
    pub raw_buffer: Vec<u8>,
    pub region_stack: Vec<Vec<usize>>,
}

#[allow(dead_code)]
impl Memory {
    pub fn new() -> Self {
        Memory {
            heap: Vec::new(),
            raw_buffer: Vec::new(),
            region_stack: Vec::new(),
        }
    }

    pub fn alloc(&mut self, value: Value) -> usize {
        let idx = self.heap.len();
        self.heap.push(value);
        idx
    }

    pub fn read(&self, idx: usize) -> Result<Value, String> {
        if idx < self.heap.len() {
            Ok(self.heap[idx].clone())
        } else {
            Err(format!("Memory access out of bounds: index {}", idx))
        }
    }

    pub fn write(&mut self, idx: usize, value: Value) -> Result<(), String> {
        if idx < self.heap.len() {
            self.heap[idx] = value;
            Ok(())
        } else {
            Err(format!("Memory write out of bounds: index {}", idx))
        }
    }

    pub fn push_region(&mut self) {
        self.region_stack.push(Vec::new());
    }

    pub fn pop_region(&mut self) {
        if let Some(indices) = self.region_stack.pop() {
            for idx in indices {
                if idx < self.heap.len() {
                    self.heap[idx] = Value::Void;
                }
            }
        }
    }

    pub fn region_alloc(&mut self, value: Value) -> usize {
        let idx = self.alloc(value);
        if let Some(region) = self.region_stack.last_mut() {
            region.push(idx);
        }
        idx
    }

    pub fn raw_alloc(&mut self, data: &[u8]) -> usize {
        let start = self.raw_buffer.len();
        self.raw_buffer.extend_from_slice(data);
        start
    }

    pub fn raw_read(&self, offset: usize, len: usize) -> Result<Vec<u8>, String> {
        if offset + len <= self.raw_buffer.len() {
            Ok(self.raw_buffer[offset..offset + len].to_vec())
        } else {
            Err(format!("Raw memory read out of bounds: offset {} len {}", offset, len))
        }
    }
}
