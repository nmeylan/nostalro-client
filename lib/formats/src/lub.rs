//! Reader for compiled Lua chunks (`.lub`), Lua 5.0 and 5.1.
//!
//! The data tables the client ships as `.lub` are straight-line chunks that
//! build one global table out of constants, so only the seven opcodes they use
//! are executed. Anything else is an error and the caller falls back.

use std::collections::HashMap;
use std::rc::Rc;

pub const SIGNATURE: &[u8] = b"\x1bLua";

const LUA50: u8 = 0x50;
const LUA51: u8 = 0x51;

const OP_LOADK: u32 = 1;
const OP_GETGLOBAL: u32 = 5;
const OP_GETTABLE: u32 = 6;
const OP_SETGLOBAL: u32 = 7;
const OP_SETTABLE: u32 = 9;
const OP_NEWTABLE: u32 = 10;
const OP_RETURN_50: u32 = 27;
const OP_RETURN_51: u32 = 30;

#[derive(Debug)]
pub enum LubError {
    NotACompiledChunk,
    UnsupportedVersion(u8),
    UnsupportedLayout,
    UnexpectedEof,
    BadConstant(u8),
    UnsupportedOpcode(u32),
    TypeError,
}

impl std::fmt::Display for LubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LubError::NotACompiledChunk => write!(f, "not a compiled lua chunk"),
            LubError::UnsupportedVersion(v) => write!(f, "unsupported lua version {v:#x}"),
            LubError::UnsupportedLayout => write!(f, "unsupported chunk layout"),
            LubError::UnexpectedEof => write!(f, "unexpected end of chunk"),
            LubError::BadConstant(t) => write!(f, "unknown constant type {t}"),
            LubError::UnsupportedOpcode(op) => write!(f, "unsupported opcode {op}"),
            LubError::TypeError => write!(f, "value is not of the expected type"),
        }
    }
}

impl std::error::Error for LubError {}

pub fn is_compiled_chunk(data: &[u8]) -> bool {
    data.starts_with(SIGNATURE)
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Str(Rc<[u8]>),
    Table(usize),
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Number(u64),
    Str(Rc<[u8]>),
}

impl Key {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Key::Number(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Key::Str(s) => Some(s),
            _ => None,
        }
    }

    fn from_value(value: &Value) -> Result<Self, LubError> {
        match value {
            Value::Number(n) => Ok(Key::Number(n.to_bits())),
            Value::Str(s) => Ok(Key::Str(s.clone())),
            _ => Err(LubError::TypeError),
        }
    }
}

pub type Table = HashMap<Key, Value>;

#[derive(Default)]
pub struct LuaState {
    globals: HashMap<Rc<[u8]>, Value>,
    tables: Vec<Table>,
}

impl LuaState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global_table(&self, name: &str) -> Option<&Table> {
        match self.globals.get(name.as_bytes()) {
            Some(Value::Table(index)) => self.tables.get(*index),
            _ => None,
        }
    }

    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.tables.iter()
    }
}

/// Runs one chunk. State is shared so a chunk can read globals an earlier one
/// left behind — `accname.lub` indexes `ACCESSORY_IDs` that way.
pub fn load_chunk(data: &[u8], state: &mut LuaState) -> Result<(), LubError> {
    let mut reader = Reader::new(data);
    let version = read_header(&mut reader)?;
    let function = read_function(&mut reader, version)?;
    execute(&function, version, state)
}

struct Function {
    constants: Vec<Value>,
    code: Vec<u32>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], LubError> {
        let end = self.pos.checked_add(count).ok_or(LubError::UnexpectedEof)?;
        let slice = self.data.get(self.pos..end).ok_or(LubError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, LubError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, LubError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, LubError> {
        Ok(self.u32()? as i32)
    }

    fn count(&mut self) -> Result<usize, LubError> {
        usize::try_from(self.i32()?).map_err(|_| LubError::UnsupportedLayout)
    }

    fn f64(&mut self) -> Result<f64, LubError> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// A dumped string is a size_t length counting the trailing NUL, then the
    /// bytes; length 0 means `nil`.
    fn string(&mut self) -> Result<Option<Rc<[u8]>>, LubError> {
        let len = self.count()?;
        if len == 0 {
            return Ok(None);
        }
        let bytes = self.take(len)?;
        Ok(Some(Rc::from(&bytes[..len - 1])))
    }
}

fn read_header(reader: &mut Reader) -> Result<u8, LubError> {
    if reader.take(4)? != SIGNATURE {
        return Err(LubError::NotACompiledChunk);
    }
    let version = reader.u8()?;
    if version == LUA51 {
        reader.u8()?;
    } else if version != LUA50 {
        return Err(LubError::UnsupportedVersion(version));
    }
    let little_endian = reader.u8()?;
    let size_int = reader.u8()?;
    let size_size_t = reader.u8()?;
    let size_instruction = reader.u8()?;
    if little_endian != 1 || size_int != 4 || size_size_t != 4 || size_instruction != 4 {
        return Err(LubError::UnsupportedLayout);
    }
    if version == LUA50 {
        let size_op = reader.u8()?;
        let size_a = reader.u8()?;
        let size_b = reader.u8()?;
        let size_c = reader.u8()?;
        if (size_op, size_a, size_b, size_c) != (6, 8, 9, 9) {
            return Err(LubError::UnsupportedLayout);
        }
        if reader.u8()? != 8 {
            return Err(LubError::UnsupportedLayout);
        }
        reader.f64()?;
    } else {
        if reader.u8()? != 8 {
            return Err(LubError::UnsupportedLayout);
        }
        if reader.u8()? != 0 {
            return Err(LubError::UnsupportedLayout);
        }
    }
    Ok(version)
}

fn read_function(reader: &mut Reader, version: u8) -> Result<Function, LubError> {
    reader.string()?;
    reader.i32()?;
    if version == LUA51 {
        reader.i32()?;
    }
    reader.u8()?;
    reader.u8()?;
    reader.u8()?;
    reader.u8()?;

    if version == LUA50 {
        read_debug(reader)?;
        let constants = read_constants(reader)?;
        read_nested_functions(reader, version)?;
        let code = read_code(reader)?;
        Ok(Function { constants, code })
    } else {
        let code = read_code(reader)?;
        let constants = read_constants(reader)?;
        read_nested_functions(reader, version)?;
        read_debug(reader)?;
        Ok(Function { constants, code })
    }
}

fn read_code(reader: &mut Reader) -> Result<Vec<u32>, LubError> {
    let count = reader.count()?;
    (0..count).map(|_| reader.u32()).collect()
}

fn read_constants(reader: &mut Reader) -> Result<Vec<Value>, LubError> {
    let count = reader.count()?;
    let mut constants = Vec::with_capacity(count);
    for _ in 0..count {
        let value = match reader.u8()? {
            0 => Value::Nil,
            1 => Value::Bool(reader.u8()? != 0),
            3 => Value::Number(reader.f64()?),
            4 => reader.string()?.map(Value::Str).unwrap_or(Value::Nil),
            other => return Err(LubError::BadConstant(other)),
        };
        constants.push(value);
    }
    Ok(constants)
}

fn read_nested_functions(reader: &mut Reader, version: u8) -> Result<(), LubError> {
    let count = reader.count()?;
    for _ in 0..count {
        read_function(reader, version)?;
    }
    Ok(())
}

fn read_debug(reader: &mut Reader) -> Result<(), LubError> {
    let lines = reader.count()?;
    reader.take(lines.checked_mul(4).ok_or(LubError::UnexpectedEof)?)?;
    let locals = reader.count()?;
    for _ in 0..locals {
        reader.string()?;
        reader.i32()?;
        reader.i32()?;
    }
    let upvalues = reader.count()?;
    for _ in 0..upvalues {
        reader.string()?;
    }
    Ok(())
}

/// 5.1 moved the operand fields around and changed how a constant is spelled in
/// an RK operand, so both are read through this.
struct Encoding {
    version: u8,
}

impl Encoding {
    fn a(&self, instruction: u32) -> usize {
        let shift = if self.version == LUA50 { 24 } else { 6 };
        ((instruction >> shift) & 0xff) as usize
    }

    fn b(&self, instruction: u32) -> u32 {
        let shift = if self.version == LUA50 { 15 } else { 23 };
        (instruction >> shift) & 0x1ff
    }

    fn c(&self, instruction: u32) -> u32 {
        let shift = if self.version == LUA50 { 6 } else { 14 };
        (instruction >> shift) & 0x1ff
    }

    fn bx(&self, instruction: u32) -> usize {
        let shift = if self.version == LUA50 { 6 } else { 14 };
        ((instruction >> shift) & 0x3ffff) as usize
    }

    /// `Some(constant index)` when the operand names a constant.
    fn constant_index(&self, operand: u32) -> Option<usize> {
        if self.version == LUA50 {
            (operand >= 250).then(|| (operand - 250) as usize)
        } else {
            (operand & 0x100 != 0).then_some((operand & 0xff) as usize)
        }
    }

    fn return_opcode(&self) -> u32 {
        if self.version == LUA50 {
            OP_RETURN_50
        } else {
            OP_RETURN_51
        }
    }
}

fn execute(function: &Function, version: u8, state: &mut LuaState) -> Result<(), LubError> {
    let encoding = Encoding { version };
    let mut registers = vec![Value::Nil; 256];

    let constant = |index: usize| -> Result<Value, LubError> {
        function
            .constants
            .get(index)
            .cloned()
            .ok_or(LubError::TypeError)
    };
    let register = |registers: &Vec<Value>, index: u32| -> Result<Value, LubError> {
        registers
            .get(index as usize)
            .cloned()
            .ok_or(LubError::TypeError)
    };

    for &instruction in &function.code {
        let opcode = instruction & 0x3f;
        if opcode == encoding.return_opcode() {
            break;
        }
        let a = encoding.a(instruction);
        if a >= registers.len() {
            return Err(LubError::TypeError);
        }
        let rk = |registers: &Vec<Value>, operand: u32| -> Result<Value, LubError> {
            match encoding.constant_index(operand) {
                Some(index) => constant(index),
                None => register(registers, operand),
            }
        };
        match opcode {
            OP_LOADK => registers[a] = constant(encoding.bx(instruction))?,
            OP_NEWTABLE => {
                state.tables.push(Table::new());
                registers[a] = Value::Table(state.tables.len() - 1);
            }
            OP_SETTABLE => {
                let Value::Table(index) = &registers[a] else {
                    return Err(LubError::TypeError);
                };
                let index = *index;
                let key = Key::from_value(&rk(&registers, encoding.b(instruction))?)?;
                let value = rk(&registers, encoding.c(instruction))?;
                state
                    .tables
                    .get_mut(index)
                    .ok_or(LubError::TypeError)?
                    .insert(key, value);
            }
            OP_GETTABLE => {
                let Value::Table(index) = register(&registers, encoding.b(instruction))? else {
                    return Err(LubError::TypeError);
                };
                let key = Key::from_value(&rk(&registers, encoding.c(instruction))?)?;
                registers[a] = state
                    .tables
                    .get(index)
                    .ok_or(LubError::TypeError)?
                    .get(&key)
                    .cloned()
                    .unwrap_or(Value::Nil);
            }
            OP_GETGLOBAL => {
                let name = constant(encoding.bx(instruction))?;
                let name = name.as_bytes().ok_or(LubError::TypeError)?;
                registers[a] = state.globals.get(name).cloned().unwrap_or(Value::Nil);
            }
            OP_SETGLOBAL => {
                let Value::Str(name) = constant(encoding.bx(instruction))? else {
                    return Err(LubError::TypeError);
                };
                state.globals.insert(name, registers[a].clone());
            }
            other => return Err(LubError::UnsupportedOpcode(other)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_lua_is_not_a_compiled_chunk() {
        assert!(!is_compiled_chunk(b"ACCESSORY_GOGGLES = 1,\n"));
        let mut state = LuaState::new();
        assert!(matches!(
            load_chunk(b"ACCESSORY_GOGGLES = 1,\n", &mut state),
            Err(LubError::NotACompiledChunk)
        ));
    }
}
