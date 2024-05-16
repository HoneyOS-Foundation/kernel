use anyhow::anyhow;
use wasm_bindgen::JsCast;
use web_sys::js_sys::{Reflect, SharedArrayBuffer, Uint8Array, WebAssembly, JSON};

/// (64Kib) The size of one wasm page as specified in the spec:
/// https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory/grow
const PAGE_SIZE: u32 = 65536;

/// A region in memory
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: u32,
    pub end: u32,
    pub free: bool,
}

impl MemoryRegion {
    /// The size of the region
    pub fn size(self) -> u32 {
        self.end - self.start
    }

    /// Split the region.
    /// Input the size of the first region.
    pub fn split(self, size: u32) -> (MemoryRegion, MemoryRegion) {
        let current_size = self.size();
        if current_size <= size {
            panic!("Size of first region must be smaller than the size of the current region");
        }
        if !self.free {
            panic!("Only free memory regions should be split")
        }

        let first = MemoryRegion {
            start: self.start,
            end: self.end - current_size - size,
            free: false,
        };
        let second = MemoryRegion {
            start: first.end,
            end: self.end,
            free: false,
        };
        (first, second)
    }
}

/// The sandboxed memory of a process
#[derive(Debug)]
pub struct Memory {
    maximum: Option<u32>,
    inner: WebAssembly::Memory,

    regions: Vec<MemoryRegion>,
}

impl Memory {
    pub fn new(initial: u32, maximum: Option<u32>, shared: bool) -> anyhow::Result<Self> {
        let memory_desc = JSON::parse("{}").unwrap();
        Reflect::set(&memory_desc, &"initial".into(), &initial.into()).unwrap();
        if let Some(maximum_memory) = maximum {
            Reflect::set(&memory_desc, &"maximum".into(), &maximum_memory.into()).unwrap();
        }
        Reflect::set(&memory_desc, &"shared".into(), &shared.into()).unwrap();

        let inner = WebAssembly::Memory::new(memory_desc.unchecked_ref())
            .map_err(|e| anyhow!("Failed to allocate memory for process: {:?}", e))?;

        Ok(Self {
            inner,
            maximum,
            regions: Vec::new(),
        })
    }

    /// Read from a certain block of memory
    pub fn read(&mut self, ptr: u32, len: u32) -> Vec<u8> {
        let buffer = self.inner.buffer();
        let bytes = Uint8Array::new(&buffer);
        bytes.slice(ptr, ptr + len as u32).to_vec()
    }

    /// Write to a certain block of memory
    pub fn write(&mut self, ptr: u32, data: &[u8]) {
        let bytes = Uint8Array::new(&self.inner.buffer());
        let array = Uint8Array::from(data);
        bytes.set(&array, ptr);
    }

    /// Allocate a block of memory and return it's pointer.
    /// Returns None if the memory exceeds the 32-bit maximum of 4gb
    pub fn alloc(&mut self, size: u32) -> Option<u32> {
        let buffer = self.inner.buffer().dyn_into::<SharedArrayBuffer>().unwrap();
        let current_size = buffer.byte_length();
        let ptr = current_size;

        // Return an old region if it is free
        let mut region_index = None;
        let regions = self.regions.clone();
        for (index, region) in regions.iter().enumerate() {
            if !region.free {
                continue;
            }
            let region_size = region.size();
            if region_size == size {
                region_index = Some(index);
            }
            if region_size > size {
                let (first, second) = region.split(size);
                self.regions.insert(index, second);
                self.regions.insert(index, first);
                region_index = Some(index);
            }
        }
        if let Some(region_index) = region_index {
            let region = self.regions.get_mut(region_index).unwrap();
            region.free = false;
            return Some(region.start);
        }

        self.grow(&buffer, size);
        self.regions.push(MemoryRegion {
            start: ptr,
            end: ptr + size,
            free: false,
        });
        Some(ptr)
    }

    /// Reallocate a block of memory and returns the new pointer
    pub fn realloc(&mut self, ptr: u32, new_size: u32) -> Option<u32> {
        let new_ptr = self.alloc(new_size)?;
        self.copy(ptr, new_ptr, new_size);
        Some(new_ptr)
    }

    /// Mark a region of memory as free
    pub fn free(&mut self, ptr: u32) -> Option<()> {
        let mut index = None;
        for (i, region) in self.regions.iter().enumerate() {
            if region.start == ptr {
                index = Some(i);
                break;
            }
        }
        let Some(index) = index else {
            return None;
        };
        let region = self.regions.get_mut(index).unwrap();
        if region.free {
            return None;
        }
        region.free = true;
        Some(())
    }

    /// Grow the memory
    fn grow(&mut self, buffer: &SharedArrayBuffer, size: u32) -> Option<()> {
        let current_size = buffer.byte_length();
        let new_size = current_size + size;

        if let Some(maximum) = self.maximum {
            if new_size / PAGE_SIZE > maximum {
                log::warn!(
                    "Process attempted to allocate more than the maximum of {} pages of ram",
                    maximum
                );
                return None;
            }
        }
        self.inner.grow(size / PAGE_SIZE + 1);
        Some(())
    }

    /// Copy data from one memory region to another
    fn copy(&self, src_ptr: u32, dest_ptr: u32, size: u32) {
        let buffer = self.inner.buffer();
        let bytes = Uint8Array::new(&buffer);

        // Perform data copying
        for i in 0..size {
            bytes.set_index(dest_ptr + i, bytes.get_index(src_ptr + i));
        }
    }

    /// Get the inner wasm memory object
    pub fn inner(&self) -> &WebAssembly::Memory {
        &self.inner
    }
}
