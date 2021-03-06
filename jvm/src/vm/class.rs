use crate::vm::class::attribute::{Attribute};
use crate::vm::class::constant::Constant;

use std::io::{Cursor, Read, Seek, Error};
use byteorder::{ReadBytesExt, BigEndian};
use std::collections::HashMap;
use std::mem::size_of;
use crate::vm::vm::{OperandType};
use std::rc::Rc;
use std::sync::RwLock;
use std::io;
use std::option::NoneError;

#[derive(Debug)]
pub struct Class { //Parsed info from the .class file
    pub constant_pool: ConstantPool,
    pub access_flags: u16,
    pub this_class: String,
    pub super_class: String,
    pub interfaces: Vec<u16>, //Index into the constant pool
    pub field_map: HashMap<String, ObjectField>,
    pub method_map: HashMap<String, HashMap<String, Rc<Method>>>,
    pub attribute_map: HashMap<String, Attribute>,

    pub static_been_seen: Rc<RwLock<bool>>,

    pub heap_size: usize,
    pub full_heap_size: usize //Heap size of this class plus the superclass
    //Dynamically sized, heap allocated vector of heap allocated Info instances blah blah blah
}

#[derive(Debug)]
pub enum ClassError {
    FieldNotFound,
    MethodNotFound
}

impl Class {
    pub fn get_field(&self, name: &String) -> Result<&ObjectField, ClassError> {
        self.field_map.get(name).ok_or(ClassError::FieldNotFound)
    }

    pub fn get_method(&self, name: &str, descriptor: &str) -> Result<Rc<Method>, ClassError> {
        self.method_map.get(name).ok_or(ClassError::MethodNotFound)?.get(descriptor).cloned().ok_or(ClassError::MethodNotFound)
    }

    pub fn has_method(&self, name: &str, descriptor: &str) -> bool {
        if self.method_map.contains_key(name) {
            if self.method_map.get(name).unwrap().contains_key(descriptor) {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct ObjectField {
    pub offset: isize,
    pub info: FieldInfo
}

#[derive(PartialEq, Eq)]
pub enum RefInfoType {
    MethodRef,
    FieldRef,
    InterfaceMethodRef
}

pub struct RefInfo {
    pub class_name: String,
    pub name: String,
    pub descriptor: String,
    pub info_type: RefInfoType
}

#[derive(Debug)]
pub struct ConstantPool {
    pool: Vec<Constant>
}

impl ConstantPool {
    pub fn new() -> Self {
        ConstantPool {
            pool: Vec::new()
        }
    }

    pub fn get_vec(&self) -> &Vec<Constant> {
        &self.pool
    }

    pub fn get(&self, index: usize) -> Option<&Constant> {
        self.pool.get(index)
    }

    pub fn insert(&mut self, index: usize, constant: Constant) {
        self.pool.insert(index, constant)
    }

    pub fn push(&mut self, constant: Constant) {
        self.pool.push(constant)
    }

    pub fn resolve_class_info(&self, index: u16) -> Option<&str> {
        let class_info = self.get(index as usize)?;
        if let Constant::Class(utf8_index) = class_info {
            if let Constant::Utf8(classname) = self.get(*utf8_index as usize)? {
                return Option::Some(classname);
            }
        }

        Option::None
    }

    pub fn resolve_utf8(&self, index: u16) -> Option<&str> {
        if let Constant::Utf8(string) = self.get(index as usize)? {
            Option::Some(string)
        } else {
            Option::None
        }
    }

    pub fn resolve_name_and_type(&self, index: u16) -> Option<(&str, &str)> {
        if let Constant::NameAndType(name_index, descriptor_index) = self.pool.get(index as usize)? {
            if let Constant::Utf8(name) = self.pool.get(*name_index as usize)? {
                if let Constant::Utf8(descriptor) = self.pool.get(*descriptor_index as usize)? {
                    return Option::Some((name, descriptor));
                }
            }
        }

        Option::None
    }

    pub fn resolve_ref_info(&self, index: usize) -> Option<RefInfo> {
        Option::Some(match self.get(index)? {
            Constant::MethodRef(class_index, name_and_type_index) => {
                let class = self.resolve_class_info(*class_index)?;
                let name_and_type = self.resolve_name_and_type(*name_and_type_index)?;

                RefInfo {
                    class_name: String::from(class.clone()),
                    name: String::from(name_and_type.0.clone()),
                    descriptor: String::from(name_and_type.1.clone()),
                    info_type: RefInfoType::MethodRef
                }
            },
            Constant::FieldRef(class_index, name_and_type_index) => {
                let class = self.resolve_class_info(*class_index)?;
                let name_and_type = self.resolve_name_and_type(*name_and_type_index)?;

                RefInfo {
                    class_name: String::from(class.clone()),
                    name: String::from(name_and_type.0.clone()),
                    descriptor: String::from(name_and_type.1.clone()),
                    info_type: RefInfoType::FieldRef
                }
            },
            Constant::InterfaceMethodRef(class_index, name_and_type_index) => {
                let class = self.resolve_class_info(*class_index)?;
                let name_and_type = self.resolve_name_and_type(*name_and_type_index)?;

                RefInfo {
                    class_name: String::from(class.clone()),
                    name: String::from(name_and_type.0),
                    descriptor: String::from(name_and_type.1),
                    info_type: RefInfoType::InterfaceMethodRef
                }
            },
            _ => return Option::None
        })
    }
}

// pub struct LinkedClass<'a> {
//     pub info: &'a ClassInfo,
//     pub interfaces:
// }

//Access flags for methods and classes
pub enum AccessFlags {
    PUBLIC = 0x1,
    PRIVATE = 0x2,
    PROTECTED = 0x4,
    STATIC = 0x8,
    FINAL = 0x10,
    SYNCHRONIZED = 0x20,
    BRIDGE = 0x40,
    VARARGS = 0x80,
    NATIVE = 0x100,
    ABSTRACT = 0x400,
    STRICT = 0x800,
    SYNTHETIC = 0x1000
}

impl AccessFlags {
    pub fn is_native(flags: u16) -> bool {
        flags & 0x100 == 0x100
    }

    pub fn is_protected(flags: u16) -> bool {
        flags & 0x4 == 0x4
    }
}

#[derive(Debug)]
pub struct Method {
    pub access_flags: u16,
    pub name: String,
    pub name_index: u16,
    pub descriptor: String,
    pub descriptor_index: u16,
    pub attributes_count: u16,
    pub attribute_map: HashMap<String, Attribute>
}

#[derive(Debug)]
pub enum FieldDescriptor {
    BaseType(BaseType),
    /// String will be a classpath to a class
    ObjectType(String),
    //ArrayType will be an ArrayType struct containing the amount of dimensions and a FieldDescriptor that resolves to either BaseType or ObjectType
    ArrayType(ArrayType)
}

impl FieldDescriptor {
    pub fn parse(desc: &str) -> Result<FieldDescriptor, ParseError> {
        if "BCDFIJSZ".contains(&desc[0..1]) { //BaseType
            return Ok(FieldDescriptor::BaseType(BaseType::get(&desc[0..1])?))
        } else if &desc[0..1] == "L" { //ObjectType
            return Ok(FieldDescriptor::ObjectType(String::from(&desc[1..desc.len()-1])))
        } else if &desc[0..1] == "[" {//ArrayType
            let mut dimensions: usize = 0;
            for i in 0..desc.len() {
                if &desc[i..i+1] != "[" {
                    dimensions = i;
                    break;
                } else if i == desc.len()-1 {
                    panic!("Invalid FieldDescriptor, no end of array type!");
                }
            }

            if "BCDFIJSZ".contains(&desc[dimensions..dimensions+1]) { //BaseType
                return Ok(FieldDescriptor::ArrayType(ArrayType {
                    dimensions: dimensions as u8,
                    field_descriptor: Box::new(FieldDescriptor::BaseType(BaseType::get(&desc[dimensions..dimensions+1])?))
                }));
            } else if &desc[dimensions..dimensions+1] == "L" { //ObjectType
                return Ok(FieldDescriptor::ArrayType(ArrayType {
                    dimensions: dimensions as u8,
                    field_descriptor: Box::new(FieldDescriptor::ObjectType(String::from(&desc[dimensions+1..desc.len()-1])))
                }));
            }
        }

        panic!(format!("Malformed field descriptor {}", desc));
    }

    pub fn matches_operand(&self, operand: OperandType) -> bool {
        match self {
            FieldDescriptor::BaseType(bt) => {
                match bt {
                    BaseType::Byte => operand == OperandType::Int,
                    BaseType::Char => operand == OperandType::Char,
                    BaseType::Double => operand == OperandType::Double,
                    BaseType::Float => operand == OperandType::Float,
                    BaseType::Int => operand == OperandType::Int,
                    BaseType::Long => operand == OperandType::Long,
                    BaseType::Reference => unreachable!("BaseType should not parse to a reference."),
                    BaseType::Bool => operand == OperandType::Int,
                    BaseType::Short => operand == OperandType::Int
                }
            }
            FieldDescriptor::ObjectType(_) => {
                if operand == OperandType::ClassReference {
                    true
                } else {
                    false
                }
            }
            FieldDescriptor::ArrayType(_) => {
                if operand == OperandType::ArrayReference {
                    true
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum MethodReturnType {
    Void,
    FieldDescriptor(FieldDescriptor)
}

#[derive(Debug)]
pub enum ParseError {
    InvalidBaseType,
    MalformedDescriptor,
    IOError(std::io::Error),
    NoneError,
    StringError
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<NoneError> for ParseError {
    fn from(_: NoneError) -> Self {
        Self::NoneError
    }
}

#[derive(Debug)]
pub struct MethodDescriptor {
    pub parameters: Vec<FieldDescriptor>,
    pub return_type: MethodReturnType
}

impl MethodDescriptor {
    pub fn parse(input: &str) -> Result<MethodDescriptor, ParseError> {
        let mut parameters: Vec<FieldDescriptor> = Vec::new();
        let params_end = input.find(")").ok_or(ParseError::MalformedDescriptor)?;

        let mut pos: usize = 1;

        while pos < input.len() {
            if pos == params_end { break; }

            if "BCDFIJSZ".contains(&input[pos..pos+1]) {
                parameters.push(FieldDescriptor::parse(&input[pos..pos+1])?);
                pos += 1;
            } else {
                let semicolon = (&input[pos..]).find(";");

                let desc_end = match semicolon {
                    None => params_end,
                    Some(s) => s
                };

                let end = desc_end + pos;
                parameters.push(FieldDescriptor::parse(&input[pos..=end])?);
                pos = end+1;
            }
        }

        Ok(MethodDescriptor {
            parameters,
            return_type: {
                if &input[input.len()-1..input.len()] == "V" {
                    MethodReturnType::Void
                } else {
                    MethodReturnType::FieldDescriptor(FieldDescriptor::parse(&input[params_end+1..])?)
                }
            }
        })
    }
}

#[derive(Debug)]
pub enum BaseType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Reference,
    Bool,
    Short
}

impl BaseType {
    pub fn get(char: &str) -> Result<BaseType, ParseError> {
        Ok(match char {
            "B" => BaseType::Byte,
            "C" => BaseType::Char,
            "D" => BaseType::Double,
            "F" => BaseType::Float,
            "I" => BaseType::Int,
            "J" => BaseType::Long,
            "S" => BaseType::Short,
            "Z" => BaseType::Bool,
            c => return Err(ParseError::InvalidBaseType)
        })
    }

    pub fn size_of(base: &BaseType, ptr_length: u8) -> usize {
        (match base {
            BaseType::Byte => {
                1
            },
            BaseType::Char => {
                2
            },
            BaseType::Double => {
                8
            },
            BaseType::Float => {
                4
            },
            BaseType::Int => {
                4
            },
            BaseType::Long => {
                8
            },
            BaseType::Reference => {
                ptr_length
            },
            BaseType::Bool => {
                1
            },
            BaseType::Short => {
                2
            }
        }) as usize
    }
}

#[derive(Debug)]
pub struct ArrayType {
    pub field_descriptor: Box<FieldDescriptor>,
    pub dimensions: u8
}

impl Method {
    pub fn from_bytes<R: Read + Seek>(rdr: &mut R, constant_pool: &ConstantPool) -> Result<Self, ParseError> {
        let attr_count: u16;
        let n_index;
        let d_index;

        Ok(Self {
            access_flags: rdr.read_u16::<BigEndian>()?,
            name_index: {
                n_index = rdr.read_u16::<BigEndian>()?;
                n_index
            },
            name:
                match constant_pool.get(n_index as usize)? {
                    Constant::Utf8(string) => String::from(string),
                    _ => panic!("Expected UTF8 for method name")
                },
            descriptor_index: {
                d_index = rdr.read_u16::<BigEndian>()?;
                d_index
            },
            descriptor:
                match constant_pool.get(d_index as usize)? {
                    Constant::Utf8(string) => String::from(string),
                    _ => panic!("Expected UTF8 for descriptor")
                },
            attributes_count: {
                attr_count = rdr.read_u16::<BigEndian>()?;
                attr_count
            },
            attribute_map: {
                let mut attr_map: HashMap<String, Attribute> = HashMap::new();

                for _ in 0..attr_count {
                    let attr = Attribute::from_bytes(rdr, &constant_pool)?;
                    attr_map.insert(String::from(&attr.attribute_name), attr);
                }

                attr_map
            }
        })
    }
}

#[derive(Debug)]
pub struct FieldInfo {
    pub access_flags: u16,
    pub name: String,
    pub field_descriptor: FieldDescriptor,
    pub attribute_map: HashMap<String, Attribute>
}

impl FieldInfo {
    pub fn from_bytes<R: Read + Seek>(rdr: &mut R, constant_pool: &ConstantPool) -> Result<Self, ParseError> {
        Ok(FieldInfo {
            access_flags: rdr.read_u16::<BigEndian>()?,
            name: match constant_pool.get(rdr.read_u16::<BigEndian>()? as usize)? {
                Constant::Utf8(str) => str.clone(),
                _ => panic!("Name index did not resolve to a UTF8 constant!")
            },
            field_descriptor: match constant_pool.get(rdr.read_u16::<BigEndian>()? as usize)? {
                Constant::Utf8(string) => FieldDescriptor::parse(string)?,
                _ => panic!("Descriptor must be UTF8!")
            },
            attribute_map: {
                let mut attr_map: HashMap<String, Attribute> = HashMap::new();
                let count = rdr.read_u16::<BigEndian>()?;
                for _ in 0..count {
                    let attr = Attribute::from_bytes(rdr, constant_pool)?;
                    attr_map.insert(String::from(&attr.attribute_name), attr);
                }

                attr_map
            }
        })
    }
}

//Attributes define information about a method, field, or class. Not every attribute applies to all
//of the aforementioned types, however. The most direct analogy would be they are like Java annotations
//for the JVM, ironically, because Annotations are actually a form of attribute at compile-time.
pub mod attribute {
    use crate::vm::class::attribute::stackmap::StackMapFrame;
    use std::io::{Cursor, Read, Seek, SeekFrom};
    use byteorder::{ReadBytesExt, BigEndian};
    use crate::vm::class::constant::{Constant};
    use crate::vm::class::{ConstantPool, ParseError};
    use crate::vm::vm::bytecode::Bytecode;

    //https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.7
    #[derive(Debug)]
    pub struct Attribute {
        pub attribute_name: String,
        pub info: AttributeItem
    }

    impl Attribute {
        pub fn from_bytes<R: Read + Seek>(rdr: &mut R, constant_pool: &ConstantPool) -> Result<Self, ParseError> {
            let start_pos = rdr.stream_position()?;

            let attribute_name_index = rdr.read_u16::<BigEndian>()?;
            let length = rdr.read_u32::<BigEndian>()?;

            let max_pos = start_pos+(length as u64)+6; //in bytes

            let attribute_constant: &Constant = constant_pool.get(attribute_name_index as usize)?;

            let utf8_string = match attribute_constant {
                Constant::Utf8(string) => string,
                _ => {
                    println!("Constant (index: {}) must be UTF8 in the constant pool!", attribute_name_index);
                    panic!("error");
                }
            };

            let attr_out = Attribute {
                attribute_name: String::from(utf8_string),
                info: match &utf8_string[..] {
                    "ConstantValue" => AttributeItem::ConstantValue(ConstantValue {
                        constant_value_index: rdr.read_u16::<BigEndian>()?
                    }),
                    "Code" => AttributeItem::Code({
                        let code_len: u32;
                        let exception_table_len: u16;
                        let attr_count: u16;

                        Code {
                            max_stack: rdr.read_u16::<BigEndian>()?,
                            max_locals: rdr.read_u16::<BigEndian>()?,
                            code_length: {
                                code_len = rdr.read_u32::<BigEndian>()?;
                                code_len
                            },
                            code: {
                                let mut vec: Vec<u8> = Vec::new();
                                for _ in 0..code_len {
                                    vec.push(rdr.read_u8()?)
                                }

                                vec
                            },
                            exception_table_length: {
                                exception_table_len = rdr.read_u16::<BigEndian>()?;
                                exception_table_len
                            },
                            exception_table: {
                                let mut vec: Vec<CodeExceptionTable> = Vec::new();
                                for _ in 0..exception_table_len {
                                    vec.push(CodeExceptionTable {
                                        start_pc: rdr.read_u16::<BigEndian>()?,
                                        end_pc: rdr.read_u16::<BigEndian>()?,
                                        handler_pc: rdr.read_u16::<BigEndian>()?,
                                        catch_type: rdr.read_u16::<BigEndian>()?
                                    });
                                }
                                vec
                            },
                            attributes_count: {
                                attr_count = rdr.read_u16::<BigEndian>()?;
                                attr_count
                            },
                            attributes: {
                                let mut vec: Vec<Attribute> = Vec::new();
                                for _ in 0..attr_count {
                                    vec.push(Attribute::from_bytes(rdr, constant_pool)?);
                                }
                                vec
                            }
                        }
                    }),
                    _ => {
                        rdr.seek(SeekFrom::Current(length as i64));
                        AttributeItem::Unimplemented
                    }
                }
            };

            // if rdr.stream_position() > max_pos {
            //     println!("Start @ {}, length is {}, end is {}, current pos is {}", start_pos, length, max_pos, rdr.position());
            //     panic!("Read too far out of attribute! Lost track of offset");
            // }

            Ok(attr_out)
        }
    }

    #[derive(Debug)]
    pub enum AttributeItem {
        ConstantValue(ConstantValue),
        Code(Code),
        CodeExceptionTable(CodeExceptionTable),
        StackMapFrame(StackMapFrame),
        Exceptions(Exceptions),
        InnerClasses(InnerClasses),
        InnerClassEntry(InnerClassEntry),
        EnclosingMethod(EnclosingMethod),
        Synthetic(Synthetic),
        Signature(Signature),
        SourceFile(SourceFile),
        SourceDebugExtension(SourceDebugExtension),
        LineNumberTable(LineNumberTable),
        LocalVariableTable(LocalVariableTable),
        Deprecated(Deprecated),
        RuntimeVisibleAnnotations(RuntimeVisibleAnnotations),
        Annotation(Annotation),
        Unimplemented
    }

    #[derive(Debug)]
    pub struct ConstantValue {
        constant_value_index: u16
    }

    #[derive(Debug)]
    pub struct Code { //This contains the executable bytecodes of a method
        max_stack: u16,
        max_locals: u16,
        code_length: u32,
        pub code: Vec<u8>, //Stream of bytes
        exception_table_length: u16,
        exception_table: Vec<CodeExceptionTable>,
        attributes_count: u16,
        attributes: Vec<Attribute>
    }

    #[derive(Debug)]
    pub struct CodeExceptionTable {
        start_pc: u16,
        end_pc: u16,
        handler_pc: u16,
        catch_type: u16
    }

    pub mod stackmap { //TODO: complete this? https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.7.4
        // pub struct StackMapTable {
        //     attribute_name_index: u16,
        //     attribute_length: u32,
        //     number_of_entries: u16,
        //     stack_map_frame: Vec<StackmapEntry>
        // }

        #[derive(Debug)]
        pub enum StackMapFrame {
            SameFrame,
            SameLocals1StackItemFrame,
            SameLocals1StackItemFrameExtended,
            ChopFrame,
            SameFrameExtended,
            AppendFrame,
            FullFrame
        }

        #[derive(Debug)]
        pub struct SameFrame {

        }
    }

    #[derive(Debug)]
    pub struct Exceptions {
        number_of_exceptions: u16,
        exception_table_index: Vec<u16>
    }

    #[derive(Debug)]
    pub struct InnerClasses {
        number_of_classes: u16,
        classes: Vec<InnerClassEntry>
    }

    #[derive(Debug)]
    pub struct InnerClassEntry {
        inner_class_info_index: u16,
        outer_class_info_index: u16,
        inner_name_index: u16,
        inner_class_access_flags: u16
    }

    #[derive(Debug)]
    pub struct EnclosingMethod {
        class_index: u16,
        method_index: u16
    }

    #[derive(Debug)]
    pub struct Synthetic {}

    #[derive(Debug)]
    pub struct Signature {
        signature_index: u16
    }

    #[derive(Debug)]
    pub struct SourceFile {
        sourcefile_index: u16
    }

    #[derive(Debug)]
    pub struct SourceDebugExtension {
        debug_extension: Vec<u8>
    }

    #[derive(Debug)]
    pub struct LineNumberTable {
        line_number_table_length: u16,
        line_number_table: Vec<LineNumberTableEntry>
    }

    #[derive(Debug)]
    pub struct LineNumberTableEntry {
        start_pc: u16,
        line_number: u16
    }

    #[derive(Debug)]
    pub struct LocalVariableTable {
        local_variable_table_length: u16,
        local_variable_table: Vec<LocalVariableTableEntry>
    }

    #[derive(Debug)]
    pub struct LocalVariableTableEntry {
        start_pc: u16,
        length: u16,
        name_index: u16,
        descriptor_index: u16,
        index: u16
    }

    // pub struct LocalVariableTypeTable {
    //     local_variable_type_table_length: u16,
    //     local_variable_type_table: Vec<LocalVariableTypeTableEntry>
    // }

    #[derive(Debug)]
    pub struct LocalVariableTypeEntry {
        start_pc: u16,
        length: u16,
        name_index: u16,
        signature_index: u16,
        index: u16
    }

    #[derive(Debug)]
    pub struct Deprecated {}

    #[derive(Debug)]
    pub struct RuntimeVisibleAnnotations {
        num_annotations: u16,
        annotations: Vec<Annotation>
    }

    #[derive(Debug)]
    pub struct Annotation {
        type_index: u16,
        num_element_value_pairs: u16,
        element_value_pairs: Vec<AnnotationElementValuePair>
    }

    #[derive(Debug)]
    pub struct ElementValue {
        tag: u8,
        value: ElementValueUnion
    }

    #[derive(Debug)]
    pub struct ElementValueUnion {
        const_value_index: u16,
    }

    #[derive(Debug)]
    pub struct AnnotationElementValuePair (u16, ElementValue);
}

pub mod constant {
    use std::io::{Cursor, Read, Seek};
    use byteorder::{ReadBytesExt, BigEndian};
    use crate::vm::class::{ConstantPool, ParseError};

    //https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.4
    #[repr(u8)]
    #[derive(Debug)]
    pub enum PoolTag {
        Utf8 = 1,
        Integer = 3,
        Float = 4,
        Long = 5,
        Double = 6,
        Class = 7,
        String = 8,
        FieldRef = 9,
        MethodRef = 10,
        InterfaceMethodRef = 11,
        NameAndType = 12,
        MethodHandle = 15,
        MethodType = 16,
        InvokeDynamic = 18,
    }

    impl PoolTag {
        fn as_int(&self) -> u8 {
            match self {
                PoolTag::Utf8 => 1,
                PoolTag::Integer => 3,
                PoolTag::Float => 4,
                PoolTag::Long => 5,
                PoolTag::Double => 6,
                PoolTag::Class => 7,
                PoolTag::String => 8,
                PoolTag::FieldRef => 9,
                PoolTag::MethodRef => 10,
                PoolTag::InterfaceMethodRef => 11,
                PoolTag::NameAndType => 12,
                PoolTag::MethodHandle => 15,
                PoolTag::MethodType => 16,
                PoolTag::InvokeDynamic => 18
            }
        }
    }

    impl PartialEq for PoolTag {
        fn eq(&self, other: &Self) -> bool {
            self.as_int() == other.as_int()
        }
    }

    impl From<u8> for PoolTag {
        fn from(you_ate: u8) -> Self {
            match you_ate {
                1  => Self::Utf8,
                3  => Self::Integer,
                4  => Self::Float,
                5  => Self::Long,
                6  => Self::Double,
                7  => Self::Class,
                8  => Self::String,
                9  => Self::FieldRef,
                10 => Self::MethodRef,
                11 => Self::InterfaceMethodRef,
                12 => Self::NameAndType,
                15 => Self::MethodHandle,
                16 => Self::MethodType,
                18 => Self::InvokeDynamic,

                tag => panic!("invalid pool tag: {}", tag),
            }
        }
    }

    #[derive(Debug)]
    pub enum Constant {
        ///bytes
        Utf8(String),
        Integer(i32),
        Float(f32),
        Long(i64),
        Double(f64),
        ///name_index
        Class(u16),
        ///string_index
        String(u16),
        ///class_index, name_and_type_index
        FieldRef(u16,u16),
        ///class_index, name_and_type_index
        MethodRef(u16, u16),
        ///class_index, name_and_type_index
        InterfaceMethodRef(u16, u16),
        ///name_index, descriptor_index
        NameAndType(u16,u16),
        ///reference_king, reference_index
        MethodHandle(u8,u16),
        ///descriptor_index
        MethodType(u16),
        ///bootstrap_method_attr_index, name_and_type_index
        InvokeDynamic(u16, u16)
    }

    impl Constant {
        pub fn from_bytes<R: Read + Seek>(rdr: &mut R, constant_pool: &ConstantPool) -> Result<Constant, ParseError> {
            let tag = rdr.read_u8()?;
            let as_pool_tag: PoolTag = tag.into();

            Ok(match as_pool_tag {
                PoolTag::Utf8 => {
                    let length = rdr.read_u16::<BigEndian>()?;
                    let mut buf = Vec::new();
                    for _ in 0..length {
                        buf.push(rdr.read_u8()?);
                    }
                    Constant::Utf8(String::from_utf8(buf).map_err(|_| ParseError::StringError)?)
                },
                PoolTag::Integer => Constant::Integer(rdr.read_i32::<BigEndian>()?),
                PoolTag::Float => Constant::Float(rdr.read_f32::<BigEndian>()?),
                PoolTag::Long => Constant::Long(rdr.read_i64::<BigEndian>()?),
                PoolTag::Double => Constant::Double(
                    rdr.read_f64::<BigEndian>()?
                ),
                PoolTag::Class => Constant::Class(rdr.read_u16::<BigEndian>()?),
                PoolTag::String => Constant::String(rdr.read_u16::<BigEndian>()?),
                PoolTag::FieldRef => Constant::FieldRef(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
                PoolTag::MethodRef => Constant::MethodRef(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
                PoolTag::InterfaceMethodRef => Constant::InterfaceMethodRef(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
                PoolTag::NameAndType => Constant::NameAndType(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?),
                PoolTag::MethodHandle => Constant::MethodHandle(rdr.read_u8()?, rdr.read_u16::<BigEndian>()?),
                PoolTag::MethodType => Constant::MethodType(rdr.read_u16::<BigEndian>()?),
                PoolTag::InvokeDynamic => Constant::InvokeDynamic(rdr.read_u16::<BigEndian>()?, rdr.read_u16::<BigEndian>()?)
            })
        }
    }
}