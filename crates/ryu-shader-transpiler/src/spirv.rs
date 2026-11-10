pub const SPIRV_MAGIC: u32 = 0x07230203;
pub const SPIRV_VERSION_1_3: u32 = 0x00010300;
pub const SPIRV_VERSION_1_5: u32 = 0x00010500;
pub const RYUBING_GENERATOR_ID: u32 = 0x00140000;

#[repr(u32)]
pub enum SpvOp {
    Nop = 0,
    MemoryModel = 14,
    EntryPoint = 15,
    ExecutionMode = 16,
    Capability = 17,
    TypeVoid = 19,
    TypeBool = 20,
    TypeInt = 21,
    TypeFloat = 22,
    TypeVector = 23,
    TypePointer = 32,
    TypeFunction = 33,
    Constant = 43,
    ConstantComposite = 44,
    Function = 54,
    FunctionEnd = 56,
    Variable = 59,
    Load = 61,
    Store = 62,
    IAdd = 128,
    FAdd = 129,
    IMul = 132,
    FMul = 133,
    Label = 248,
    Branch = 249,
    Return = 253,
}

#[repr(u32)]
pub enum SpvCapability {
    Matrix = 0,
    Shader = 1,
    Geometry = 2,
    Tessellation = 3,
    Float64 = 4,
    Int64 = 5,
}

#[repr(u32)]
pub enum SpvExecutionModel {
    Vertex = 0,
    TessellationControl = 1,
    TessellationEvaluation = 2,
    Geometry = 3,
    Fragment = 4,
    GLCompute = 5,
}

pub struct SpirvEmitter {
    pub words: Vec<u32>,
    pub next_id: u32,
    pub version: u32,
}

impl SpirvEmitter {
    pub fn new(version: u32) -> Self {
        let mut words = Vec::with_capacity(1024);
        // Reserve 5 words for header
        words.resize(5, 0);

        Self {
            words,
            next_id: 1,
            version: if version != 0 {
                version
            } else {
                SPIRV_VERSION_1_5
            },
        }
    }

    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    #[inline]
    pub fn emit_instruction(&mut self, op: SpvOp, operands: &[u32]) {
        let word_count = (operands.len() + 1) as u32;
        let first_word = (word_count << 16) | (op as u32 & 0xFFFF);
        self.words.push(first_word);
        self.words.extend_from_slice(operands);
    }

    pub fn emit_capability(&mut self, cap: SpvCapability) {
        self.emit_instruction(SpvOp::Capability, &[cap as u32]);
    }

    pub fn emit_memory_model(&mut self, addressing: u32, memory: u32) {
        self.emit_instruction(SpvOp::MemoryModel, &[addressing, memory]);
    }

    pub fn finalize(mut self) -> Vec<u32> {
        // Write header
        self.words[0] = SPIRV_MAGIC;
        self.words[1] = self.version;
        self.words[2] = RYUBING_GENERATOR_ID;
        self.words[3] = self.next_id; // Bound ID
        self.words[4] = 0; // Reserved

        self.words
    }
}
