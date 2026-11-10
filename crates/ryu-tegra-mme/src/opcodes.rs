#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOperation {
    AluReg = 0,
    AddImmediate = 1,
    BitfieldReplace = 2,
    BitfieldExtractLslImm = 3,
    BitfieldExtractLslReg = 4,
    ReadImmediate = 5,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentOperation {
    IgnoreAndFetch = 0,
    Move = 1,
    MoveAndSetMaddr = 2,
    FetchAndSend = 3,
    MoveAndSend = 4,
    FetchAndSetMaddr = 5,
    MoveAndSetMaddrThenFetchAndSend = 6,
    MoveAndSetMaddrThenSendHigh = 7,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluRegOperation {
    Add = 0,
    AddWithCarry = 1,
    Subtract = 2,
    SubtractWithBorrow = 3,
    BitwiseExclusiveOr = 8,
    BitwiseOr = 9,
    BitwiseAnd = 10,
    BitwiseAndNot = 11,
    BitwiseNotAnd = 12,
}
