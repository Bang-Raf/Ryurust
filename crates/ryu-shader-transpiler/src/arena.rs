use crate::maxwell::MaxwellInstruction;
use bumpalo::Bump;

#[derive(Debug)]
pub enum IrNode<'a> {
    Instruction(MaxwellInstruction),
    Block {
        id: u32,
        start_offset: u32,
        instructions: &'a [MaxwellInstruction],
    },
    Phi {
        dst_reg: u8,
        sources: &'a [(u32, u8)], // (block_id, reg)
    },
}

pub struct ShaderArena {
    pub bump: Bump,
}

impl ShaderArena {
    pub fn new() -> Self {
        Self {
            bump: Bump::with_capacity(64 * 1024), // 64KB initial slab
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    pub fn alloc<T>(&self, val: T) -> &mut T {
        self.bump.alloc(val)
    }

    pub fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> &mut [T] {
        self.bump.alloc_slice_copy(src)
    }

    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

impl Default for ShaderArena {
    fn default() -> Self {
        Self::new()
    }
}
