#![no_std]

use core::fmt::{Debug, Display, Formatter, Write as CoreFmtWrite};
use embedded_io::{ErrorType, Write};
use numtoa::base10;

#[cfg(feature = "alloc")]
extern crate elsa;
#[cfg(feature = "alloc")]
use elsa::FrozenVec;

/// a buffer for an growable string escape buffer. enabled with `alloc` feature.
#[cfg(feature = "alloc")]
pub type AllocEscapeBuffer = FrozenVec<String>;

/// trait for types that JSON can be serialized into. mainly meant for internal usage.
pub trait StringWrite {
    type StringWriteFailure: Debug;
    fn write_char(&mut self, data: char, bytes_to_skip: usize) -> Result<usize,(usize,Self::StringWriteFailure)>;
}

impl<T: Write + ErrorType> StringWrite for T {
    type StringWriteFailure = T::Error;
    fn write_char(&mut self, data: char, resume_from: usize) -> Result<usize,(usize,Self::StringWriteFailure)> {
        debug_assert!(resume_from <= 4);
        let mut str_buffer = [0_u8; 4];
        let encoded_string = data.encode_utf8(str_buffer.as_mut_slice()).as_bytes();
        let to_skip = core::cmp::min(encoded_string.len(), resume_from);
        let target = encoded_string.split_at(to_skip).1;
        if target.is_empty() {
            return Ok(0);
        }
        match self.write_all(target) {
            Ok(()) => Ok(target.len() + to_skip),
            Err(e) => Err((0,e))
        }
    }
}

struct FormatWrapper<T: ?Sized> {
    inner: T,
}

impl<T> FormatWrapper<T> {
    fn new(inner: T) -> Self {
        FormatWrapper { inner }
    }
}

impl<'a> StringWrite for FormatWrapper<&mut Formatter<'a>> {
    type StringWriteFailure = core::fmt::Error;
    fn write_char(&mut self, data: char, bytes_to_skip: usize) -> Result<usize,(usize,Self::StringWriteFailure)> {
        assert!(bytes_to_skip == 0);
        let mut encoding_buffer = [0_u8; 4];
        let n = data.encode_utf8(encoding_buffer.as_mut_slice()).len();
        match self.inner.write_char(data) {
            Ok(()) => Ok(n),
            Err(e) => Err((0,e))
        }
    }
}

/// trait for an optionally mutable collection of JSON array values
pub trait ValueBuffer<'a>: AsRef<[JsonValue<'a>]> {

    /// convenience one-liner to call JsonArray::wrap_init on this Sized type, consuming it
    fn into_json_array(self) -> JsonArray<Self> where Self: Sized {
        JsonArray::wrap_init(self)
    }
    
    /// convenience one-liner to call JsonArray::wrap_init on an immutable reference to this type
    fn as_json_array(&self) -> JsonArray<&Self> {
        JsonArray::wrap_init(self)
    }

}

/// ValueBuffer is automatically implemented for all types that implement AsRef<[JsonField<'data,'data>]>
impl <'a,T: AsRef<[JsonValue<'a>]>> ValueBuffer<'a> for T {}


/// trait for a mutable collection of JSON array values
pub trait ValueBufferMut<'a>: ValueBuffer<'a> +  AsMut<[JsonField<'a,'a>]> {

    /// convenience one-liner to call JsonObject::wrap_init on a mutable reference to this type
    fn as_json_array_mut(&mut self) -> JsonArray<&mut Self> {
        JsonArray::wrap_init(self)
    }
}

/// ValueBufferMut is automatically implemented for all types that implement FieldBuffer + AsMut<[JsonField<'data,'data>]>
impl <'a,T: ValueBuffer<'a> + AsMut<[JsonField<'a,'a>]>> ValueBufferMut<'a> for T {}


/// trait for all optionally mutable collection of JSON object fields
pub trait FieldBuffer<'data>: AsRef<[JsonField<'data,'data>]> {

    /// convenience one-liner to call JsonObject::wrap_init on this Sized type, moving it
    fn into_json_object(self) -> JsonObject<Self> where Self: Sized {
        JsonObject::wrap_init(self)
    }
    
    /// convenience one-liner to call JsonObject::wrap_init on an immutable reference to this type
    fn as_json_object(&self) -> JsonObject<&Self> {
        JsonObject::wrap_init(self)
    }

}

/// FieldBuffer is automatically implemented for all types that implement AsRef<[JsonField<'data,'data>]>
impl <'a,T: AsRef<[JsonField<'a,'a>]>> FieldBuffer<'a> for T {}

/// trait for a mutable collection of JSON object fields
pub trait FieldBufferMut<'a>: FieldBuffer<'a> +  AsMut<[JsonField<'a,'a>]> {

    /// convenience one-liner to call JsonObject::wrap_init on a mutable reference to this type
    fn as_json_object_mut(&mut self) -> JsonObject<&mut Self> {
        JsonObject::wrap_init(self)
    }

}

/// FieldBufferMut is automatically implemented for all types that implement FieldBuffer + AsMut<[JsonField<'data,'data>]>
impl <'a,T: FieldBuffer<'a> + AsMut<[JsonField<'a,'a>]>> FieldBufferMut<'a> for T {}

/// the various reasons parsing JSON can fail
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum JsonParseFailure {
    /// there was no error, but the data slice is incomplete
    Incomplete,
    /// there was no error, but there were more fields than the provided field buffer could hold
    FieldBufferTooSmall,
    /// there was no error, but there were more fields than the provided string escape buffer could hold
    EscapeBufferTooSmall,
    /// there was an error in the JSON structure of the data
    InvalidStructure,
    /// an invalid JSON string was encountered
    InvalidStringField,
    /// an invalid JSON number was encountered
    InvalidNumericField,
    /// a valid JSON number was encountered but we failed to interpret it
    NumberParseError,
    /// an invalid JSON boolean was encountered
    InvalidBooleanField,
    /// an invalid JSON null was encountered
    InvalidNullField,
}

/// terminal (non-nested) JSON types
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum JsonValue<'a> {
    /// a JSON string - it will be automatically escaped
    String(&'a str),
    /// a JSON boolean
    Boolean(bool),
    /// a JSON number
    Number(i64),
    /// a JSON null value
    Null,
}

impl <'a> JsonValue<'a> {
    pub fn parse(data: &'a [u8], escape_buffer_slice: &'a mut [u8]) -> Result<(usize,Self),JsonParseFailure> {
        let mut escape_buffer = StringBuffer::Finite(0, escape_buffer_slice);
        let mut current_data_index = 0_usize;
        skip_whitespace(&mut current_data_index, data)?;
        // let first_character = data[current_data_index];
        let value = if data[current_data_index] == b'"' {
                let unescaped_string_value = unescape_json_string(&mut current_data_index, data, &mut escape_buffer)?;
                JsonValue::String(unescaped_string_value)
            } else if data[current_data_index] == b'n' {
                skip_literal(&mut current_data_index, data, "null", JsonParseFailure::InvalidBooleanField)?;
                JsonValue::Null
            } else if data[current_data_index] == b't' || data[current_data_index] == b'f' {
                let expect_true = data[current_data_index] == b't';
                skip_literal(&mut current_data_index, data, if expect_true { "true" } else { "false"}, JsonParseFailure::InvalidBooleanField)?;
                JsonValue::Boolean(expect_true)
            } else if data[current_data_index] == b'-' {
                // negative number
                let minus_sign_numeric_start_index = current_data_index;
                current_data_index += 1;
                skip_numeric(&mut current_data_index, data)?;
                let minus_sign_numeric_end = current_data_index;
                if minus_sign_numeric_end - minus_sign_numeric_start_index == 1 {
                    // no digits found
                    return Err(JsonParseFailure::InvalidNumericField);
                }
                let numeric_string = core::str::from_utf8(&data[minus_sign_numeric_start_index..minus_sign_numeric_end]).expect("skipped negative number digit(s)");
                let numeric_value: i64 = match numeric_string.parse() {
                    Ok(i) => i,
                    Err(_parse_int_error) => return Err(JsonParseFailure::NumberParseError),
                };
                JsonValue::Number(numeric_value)
            } else if data[current_data_index] >= b'0' && data[current_data_index] < b'9' {
                // positive number
                let numeric_start_index = current_data_index;
                current_data_index += 1;
                skip_numeric(&mut current_data_index, data)?;
                let numeric_after_index = current_data_index;
                let numeric_string = core::str::from_utf8(&data[numeric_start_index..numeric_after_index]).expect("skipped positive number digit(s)");
                let numeric_value: i64 = match numeric_string.parse() {
                    Ok(i) => i,
                    Err(_parse_int_error) => return Err(JsonParseFailure::NumberParseError),
                };
                JsonValue::Number(numeric_value)
            } else {
                return Err(JsonParseFailure::InvalidStructure);
            };
            Ok((current_data_index,value))
    }
}

impl<'a> Default for JsonValue<'a> {
    fn default() -> Self { JsonValue::Null }
}

impl From<i64> for JsonValue<'static> {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for JsonValue<'static> {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<()> for JsonValue<'static> {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl<'a> From<&'a str> for JsonValue<'a> {
    fn from(s: &'a str) -> Self {
        Self::String(s)
    }
}

/// a default JSON value with static lifetime. equivalent to `JsonValue::Null`.
pub const EMPTY_VALUE: JsonValue<'static> = JsonValue::Null;

/// a field within a JSON object
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub struct JsonField<'a,'b> {
    pub key: &'a str,
    pub value: JsonValue<'b>,
}

impl <'a,'b> JsonField<'a,'b> {
    /// create a new JSON object field with the given key & value
    pub const fn new(key: &'a str, value: JsonValue<'b>) -> Self {
        JsonField { key, value }
    }

    /// convenience helper to get the json field as a (key,value) tuple
    pub const fn from_tuple(tuple: (&'a str, JsonValue<'b>)) -> Self {
        Self::new(tuple.0, tuple.1)
    }

    /// convenience helper to get the json field as a (key,value) tuple
    pub const fn as_tuple(&self) -> (&'a str, JsonValue<'b>) {
        (self.key, self.value)
    }

    /// convenience helper to create a new JSON object string field
    pub const fn new_string(key: &'a str, value: &'b str) -> Self {
        Self::new(key, JsonValue::String(value))
    }
    /// convenience helper to create a new JSON object number field
    pub const fn new_number(key: &'a str, value: i64) -> Self {
        Self::new(key, JsonValue::Number(value))
    }
    /// convenience helper to create a new JSON object boolean field
    pub const fn new_boolean(key: &'a str, value: bool) -> Self {
        Self::new(key, JsonValue::Boolean(value))
    }
}

/// two JsonObjects are equal if their initialized fields are identical (in the same order)
impl<'a,T: FieldBuffer<'a>> PartialEq for JsonObject<T> {
    fn eq(&self, other: &JsonObject<T>) -> bool {
        self.num_fields == other.num_fields && PartialEq::eq(self.fields.as_ref(), other.fields.as_ref())
    }
}

/// PartialEq for JsonObject is reflexive
impl<'a,T: FieldBuffer<'a>> Eq for JsonObject<T> {}

/// a default JSON field with static lifetime. equivalent to `JsonField::new("", JsonValue::Null)`
pub const EMPTY_FIELD: JsonField<'static,'static> = JsonField{ key: "", value: JsonValue::Null};

impl <'a,'b,V: Into<JsonValue<'b>>> From<(&'a str, V)> for JsonField<'a,'b> {
    fn from(tuple: (&'a str, V)) -> Self {
        Self::new(tuple.0, tuple.1.into())
    }
}

impl <'a,'b> Default for JsonField<'a,'b> {
    fn default() -> Self {
        EMPTY_FIELD
    }
}

/// JsonObject represents an RFC 8259 JSON Array. It wraps a mutable or immutable buffer of JSON values.  The easiest way to use it is through the ArrayJsonArray type alias, however you can use JsonArray directly to wrap your own buffer like a heap allocated Vec.
#[derive(Debug,Clone,Copy)]
pub struct JsonArray<Values> {
    values: Values,
    num_values: usize,
}

impl<T> JsonArray<T> {
    /// consume this JsonObject to return (field buffer, num fields considered initialized)
    pub fn into_inner(self) -> (T,usize) {
        (self.values,self.num_values)
    }
}

impl<'a,T: FieldBuffer<'a> + Default + ?Sized> Default for JsonArray<T> {
    fn default() -> Self {
        JsonArray { values: T::default(), num_values: 0 }
    }
}

impl <'a,T: ValueBuffer<'a>> JsonArray<T> {

    /// wrap a collection of values into a JsonArray and considers none of the values to be initialized
    pub const fn wrap(values: T) -> Self {
        JsonArray { values, num_values: 0 }
    }

    /// wrap a collection of fields into a JsonObject and considers all of the fields to be initialized
    pub fn wrap_init(values: T) -> Self {
        let num_values = values.as_ref().len();
        JsonArray { values, num_values }
    }

    /// get the number of initialized values in this JsonArray. Same as self.values().len()
    pub const fn len(&self) -> usize {
        self.num_values
    }

    /// get the max number of values this JsonArray can store
    pub fn capacity(&self) -> usize {
        self.values.as_ref().len()
    }

    /// get an immutable reference to the initialized values of this JsonArray
    pub fn values(&self) -> &[JsonValue<'a>] {
        self.values.as_ref().split_at(self.num_values).0
    }

    /// attempt to serialize this JsonArray into the provided output & returns the number of bytes written on success
    pub fn serialize<Output: Write>(&self, mut output: Output) -> Result<usize,Output::Error> {
        match serialize_json_array(&mut output, self.values().as_ref(), 0) {
            Ok(n) => Ok(n),
            Err((_written,e)) => Err(e),
        }
    }

    /// attempt to serialize this JsonArray into the provided output starting from `resume_from` & returns the number of bytes written on both success & failure
    pub fn serialize_resume<Output: Write>(&self, mut output: Output, resume_from: usize) -> Result<usize,(usize,Output::Error)> {
        serialize_json_array(&mut output, self.values().as_ref(), resume_from)
    }

}

impl <'a,T: ValueBuffer<'a>> Display for JsonArray<T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        match serialize_json_array(
            &mut FormatWrapper::new(fmt),
            self.values.as_ref(),
            0,
        ) {
            Ok(_) => Ok(()),
            Err((_written,e)) => Err(e),
        }
    }
}

/// ArrayJsonObject is a type alias for a JsonObject that wraps an array. It has extra functionality when compared to any other type of JsonObject.
pub type ArrayJsonArray<'a,const N: usize> = JsonArray<[JsonValue<'a>; N]>;

impl<'a,const N: usize> ArrayJsonArray<'a,N> {
    
    /// convenience method to initialize a new array & call JsonObject::wrap on it
    pub const fn new() -> Self {
        JsonArray::wrap([JsonValue::Null; N])
    }

    /// convenience method to automatically create an ArrayJsonObject if object parsing is successful
    // pub fn new_parsed(data: &'a [u8], escape_buffer: &'a mut [u8]) -> Result<(usize,Self),JsonParseFailure> {
    //     let mut ret = Self::new();
    //     let data_end = ret.parse(data, escape_buffer)?;
    //     Ok((data_end,ret))
    // }

    /// similar to JsonObject::push but supports const contexts & only returns a reference
    pub const fn push_const(&mut self, value: JsonValue<'a>) -> Result<(),()> {
        if self.num_values == N {
            return Err(());
        }
        self.values[self.num_values] = value;
        self.num_values += 1;
        Ok(())
    }

    /// similar to JsonObject::pop but supports const contexts
    pub const fn pop_const(&mut self) -> Option<&JsonValue<'a>> {
        match self.values_const().split_last() {
            None => return None,
            Some((split,_remaining)) => return Some(split),
        }
    }

    /// same as JsonObject::fields but supports const contexts
    pub const fn values_const(&self) -> &[JsonValue<'a>] {
        self.values.split_at(self.num_values).0
    }

    /// same as JsonObject::fields_mut but supports const contexts
    pub const fn values_mut_const(&mut self) -> &mut [JsonValue<'a>] {
        self.values.split_at_mut(self.num_values).0
    }
}


/// JsonObject represents an RFC 8259 JSON Object. It wraps a mutable or immutable buffer of object fields. The easiest way to use it is through the ArrayJsonObject type alias, however you can use JsonObject directly to wrap your own buffer like a heap allocated Vec
#[derive(Debug,Clone,Copy)]
pub struct JsonObject<Fields> {
    fields: Fields,
    num_fields: usize,
}

impl<T> JsonObject<T> {
    /// consume this JsonObject to return (field buffer, num fields considered initialized)
    pub fn into_inner(self) -> (T,usize) {
        (self.fields,self.num_fields)
    }
}

impl<'a,T: FieldBuffer<'a> + Default + ?Sized> Default for JsonObject<T> {
    fn default() -> Self {
        JsonObject::wrap(T::default())
    }
}

impl <'a,T: FieldBuffer<'a>> JsonObject<T> {

    /// wrap a collection of fields into a JsonObject and considers none of the fields to be initialized
    pub const fn wrap(fields: T) -> Self {
        JsonObject { fields, num_fields: 0 }
    }

    /// wrap a collection of fields into a JsonObject and considers all of the fields to be initialized
    pub fn wrap_init(fields: T) -> Self {
        let num_fields = fields.as_ref().len();
        JsonObject { fields, num_fields }
    }

    /// get the number of initialized fields in this JsonObject. Same as self.fields().len().
    pub const fn len(&self) -> usize {
        self.num_fields
    }

    /// get the max number of fields this JsonObject can store.
    pub fn capacity(&self) -> usize {
        self.fields.as_ref().len()
    }

    /// get an immutable reference to the initialized fields of this JsonObject
    pub fn fields(&self) -> &[JsonField<'a,'a>] {
        self.fields.as_ref().split_at(self.num_fields).0
    }

    /// attempt to serialize this JsonObject into the provided output & returns the number of bytes written on success
    pub fn serialize<Output: Write>(&self, mut output: Output) -> Result<usize,Output::Error> {
        match serialize_json_object(&mut output, self.fields().as_ref(), 0) {
            Ok(n) => Ok(n),
            Err((_written,e)) => Err(e),
        }
    }

    /// attempt to serialize this JsonObject into the provided output starting from `resume_from` & returns the number of bytes written on both success & failure
    pub fn serialize_resume<Output: Write>(&self, mut output: Output, resume_from: usize) -> Result<usize,(usize,Output::Error)> {
        serialize_json_object(&mut output, self.fields().as_ref(), resume_from)
    }
}

impl <'a,T: FieldBuffer<'a>> Display for JsonObject<T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        match serialize_json_object(
            &mut FormatWrapper::new(fmt),
            self.fields.as_ref().split_at(self.num_fields).0,
            0
        ) {
            Ok(_) => Ok(()),
            Err((_written,e)) => Err(e),
        }
    }
}

impl <'a,T: FieldBuffer<'a>> From<T> for JsonObject<T> {
    fn from(t: T) -> Self {
        Self::wrap_init(t)
    }
}

impl <'a,T: FieldBufferMut<'a>> JsonObject<T> {

    /// get a mutable reference to the initialized fields of this JsonObject
    pub fn fields_mut(&mut self) -> &mut [JsonField<'a,'a>] {
        self.fields.as_mut().split_at_mut(self.num_fields).0
    }

    /// attempt to push a new field - returns the field if there is not enough space
    pub fn push<'x: 'a,'y: 'a>(&mut self, field: JsonField<'x,'y>) -> Result<(),JsonField<'x,'y>> {
        if self.num_fields == self.fields.as_ref().len(){
            return Err(field);
        }
        self.fields.as_mut()[self.num_fields] = field;
        self.num_fields += 1;
        Ok(())
    }

    /// attempt to pop an existing field - returns None if there are no initialized fields
    pub fn pop(&mut self) -> Option<JsonField<'a,'a>> {
        if self.num_fields == 0 {
            return None;
        }
        self.num_fields -= 1;
        Some(core::mem::take(&mut self.fields.as_mut()[self.num_fields+1]))
    }

    /// convenience helper to create and push a new field
    pub fn push_field<'x: 'a,'y: 'a>(&mut self, key: &'x str, value: JsonValue<'y>) -> Result<(),()> {
        if self.num_fields == self.fields.as_ref().len(){
            return Err(());
        }
        self.fields.as_mut()[self.num_fields] = JsonField { key, value };
        self.num_fields += 1;
        Ok(())
    }

    /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject - returns a tuple of (num bytes consumed, num fields parsed) on success
    pub fn parse(&mut self, data: &'a [u8], string_escape_buffer: &'a mut [u8]) -> Result<usize,JsonParseFailure> {
        let (data_end, parsed_fields) = parse_json_object(
            data,
            ParseBuffer::Finite(0, self.fields.as_mut()),
            &mut StringBuffer::Finite(0, string_escape_buffer),
        )?;
        let new_num_fields = parsed_fields;
        self.num_fields = new_num_fields;
        Ok(data_end)
    }

    /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing escaped strings
    /// returns num bytes consumed on success
    #[cfg(feature = "alloc")]
    pub fn parse_alloc_escape(&mut self, data: &'a [u8], escape_buffer: &'a FrozenVec<String>) -> Result<usize,JsonParseFailure> {
        let (data_end, parsed_fields) = parse_json_object(
            data,
            ParseBuffer::Finite(0,self.fields.as_mut()),
            &mut crate::StringBuffer::Infinite(String::new(), escape_buffer)
        )?;
        let new_num_fields = parsed_fields;
        self.num_fields = new_num_fields;
        Ok(data_end)
    }

}

impl <'a,T: FieldBufferMut<'a> + Default> JsonObject<T> {

    /// convenience method to automatically create a JsonObject if object parsing is successful
    pub fn default_parsed(data: &'a [u8], escape_buffer: &'a mut [u8]) -> Result<(usize,Self),JsonParseFailure> {
        let mut ret = Self::default();
        let num_bytes = ret.parse(data, escape_buffer)?;
        Ok((num_bytes,ret))
    }

}


/// ArrayJsonObject is a type alias for a JsonObject that wraps an array. It has extra functionality when compared to any other type of JsonObject.
pub type ArrayJsonObject<'a,const N: usize> = JsonObject<[JsonField<'a,'a>; N]>;

impl<'a,const N: usize> ArrayJsonObject<'a,N> {

    /// convenience method to initialize a new array & call JsonObject::wrap on it
    pub const fn new() -> Self {
        JsonObject::wrap([EMPTY_FIELD; N])
    }

    /// convenience method to automatically create an ArrayJsonObject if object parsing is successful
    pub fn new_parsed(data: &'a [u8], escape_buffer: &'a mut [u8]) -> Result<(usize,Self),JsonParseFailure> {
        let mut ret = Self::new();
        let data_end = ret.parse(data, escape_buffer)?;
        Ok((data_end,ret))
    }

    /// similar to JsonObject::push but supports const contexts & only returns a reference
    pub const fn push_const(&mut self, key: &'a str, value: JsonValue<'a>) -> Result<(),()> {
        if self.num_fields == N {
            return Err(());
        }
        self.fields[self.num_fields] = JsonField { key, value: value };
        self.num_fields += 1;
        Ok(())
    }

    /// similar to JsonObject::pop but supports const contexts
    pub const fn pop_const(&mut self) -> Option<&JsonField<'a,'a>> {
        match self.fields_const().split_last() {
            None => return None,
            Some((split,_remaining)) => return Some(split),
        }
    }

    /// same as JsonObject::fields but supports const contexts
    pub const fn fields_const(&self) -> &[JsonField<'a,'a>] {
        self.fields.split_at(self.num_fields).0
    }

    /// same as JsonObject::fields_mut but supports const contexts
    pub const fn fields_mut_const(&mut self) -> &mut [JsonField<'a,'a>] {
        self.fields.split_at_mut(self.num_fields).0
    }

}

#[cfg(feature = "alloc")]
extern crate alloc as alloclib;
#[cfg(feature = "alloc")]
use alloclib::{string::String, vec::Vec};

/// a buffer that any sized type can be written to. `ParseBuffer::Infinite` is only available with the `alloc` feature enabled.
pub enum ParseBuffer<'a,T> {
    /// a finite buffer of T
    Finite(usize, &'a mut [T]),
    /// an infinite buffer of T
    #[cfg(feature = "alloc")]
    Infinite(usize,&'a mut Vec<T>)
}

impl<'a,T> ParseBuffer<'a,T> {

    fn write_thing(&mut self, thing: T) -> Result<(),JsonParseFailure> {
        match self {
            ParseBuffer::Finite(position, slice) => {
                if *position == (*slice).len() {
                    Err(JsonParseFailure::FieldBufferTooSmall)
                } else {
                    slice[*position] = thing;
                    *position += 1;
                    Ok(())
                }
            },
            #[cfg(feature = "alloc")]
            ParseBuffer::Infinite(position,vec) => {
                if *position < vec.len() {
                    vec[*position] = thing;
                    *position += 1;
                    Ok(())
                } else {
                    vec.push(thing);
                    *position += 1;
                    Ok(())
                }
            }
        }
    }

    const fn consume(self) -> usize {
        match self {
            ParseBuffer::Finite(n, _) => n,
            #[cfg(feature = "alloc")]
            ParseBuffer::Infinite(n, _) => n,
        }
    }
}

// pub enum StringOutput<T> {
//     Write(usize,T),

//     String(String),
// }

/// a buffer that string slices can be written to
pub enum StringBuffer<'a> {
    Finite(usize, &'a mut [u8]),
    #[cfg(feature = "alloc")]
    Infinite(String,&'a AllocEscapeBuffer),
}

impl<'a> StringBuffer<'a> {
    fn write_part(&mut self, string: &str) -> Result<(),JsonParseFailure> {
        if string.len() == 0 {
            return Ok(())
        }
        match self {
            StringBuffer::Finite(position, slice) => {
                let needed = string.len();
                let have = slice.len() - *position;
                if needed > have {
                    return Err(JsonParseFailure::EscapeBufferTooSmall);
                }
                let target = slice.split_at_mut(*position).1.split_at_mut(needed).0;
                target.copy_from_slice(string.as_bytes());
                *position += needed;
                Ok(())
            },
            #[cfg(feature = "alloc")]
            StringBuffer::Infinite(current_string, _frozen_vec) => {
                current_string.push_str(string);
                Ok(())
            },
        }
    }
    fn consume_string(&mut self) -> &'a str {
        match self {
            StringBuffer::Finite(position, slice) => {
                let (ret, remaining) = core::mem::take(slice).split_at_mut(*position);
                *slice = remaining;
                *position = 0;
                // safety: this data was written from &str
                unsafe { core::str::from_utf8_unchecked(ret) }
            },
            #[cfg(feature = "alloc")]
            StringBuffer::Infinite(current_string, frozen_vec) => {
                let completed_string = core::mem::replace(current_string, String::new());
                let x = frozen_vec.push_get(completed_string);
                x
            },
        }
    }
}



/// the core function that powers parsing in the JsonObject API. It attempts to parse the fields of a json object from the provided data slice into the provided parse buffer.
/// returns (num bytes consumed,num fields parsed) on success
pub fn parse_json_object<'input_data: 'escaped_data,'escaped_data>(
    data: &'input_data [u8],
    mut field_buffer: ParseBuffer<'_,JsonField<'escaped_data,'escaped_data>>,
    string_escape_buffer: &mut StringBuffer<'escaped_data>,
) -> Result<(usize,usize),JsonParseFailure> {
    let mut current_data_index = 0;
    // let mut current_field_index = 0;
    let mut map_entry_needs_comma = false;
    skip_whitespace(&mut current_data_index, data)?;
    if data[current_data_index] != b'{' {
        return Err(JsonParseFailure::InvalidStructure);
    }
    let _map_start_index = current_data_index;
    current_data_index += 1;
    while current_data_index < data.len()  {
        skip_whitespace(&mut current_data_index, data)?;
        if data[current_data_index] == b'}' {
            return Ok((current_data_index+1,field_buffer.consume()))
        } else if map_entry_needs_comma  {
            if data[current_data_index] != b',' {
                return Err(JsonParseFailure::InvalidStructure);
            }
            current_data_index += 1;
            map_entry_needs_comma = false;
        } else {
            map_entry_needs_comma = true;
            // let key_start_quote_index = current_data_index;
            // current_data_index += 1; // include the quote for json string

            let string_key = unescape_json_string(&mut current_data_index, data, string_escape_buffer)?;

            // skip_json_string(&mut current_data_index, data)?;
            // let key_end_quote_index = current_data_index;
            // let string_key = core::str::from_utf8(&data[key_start_quote_index+1..key_end_quote_index]).expect("skipped json object key string");
            // current_data_index += 1;
            skip_whitespace(&mut current_data_index, data)?;
            if data[current_data_index] != b':' {
                return Err(JsonParseFailure::InvalidStructure);
            }
            current_data_index += 1;
            skip_whitespace(&mut current_data_index, data)?;

            if data[current_data_index] == b'"' {
                let unescaped_string_value = unescape_json_string(&mut current_data_index, data, string_escape_buffer)?;
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::String(unescaped_string_value)))?;
            } else if data[current_data_index] == b'n' {
                skip_literal(&mut current_data_index, data, "null", JsonParseFailure::InvalidBooleanField)?;
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::Null))?;
            } else if data[current_data_index] == b't' || data[current_data_index] == b'f' {
                let expect_true = data[current_data_index] == b't';
                skip_literal(&mut current_data_index, data, if expect_true { "true" } else { "false"}, JsonParseFailure::InvalidBooleanField)?;
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::Boolean(expect_true)))?;
            } else if data[current_data_index] == b'-' {
                // negative number
                let minus_sign_numeric_start_index = current_data_index;
                current_data_index += 1;
                skip_numeric(&mut current_data_index, data)?;
                let minus_sign_numeric_end = current_data_index;
                if minus_sign_numeric_end - minus_sign_numeric_start_index == 1 {
                    // no digits found
                    return Err(JsonParseFailure::InvalidNumericField);
                }
                let numeric_string = core::str::from_utf8(&data[minus_sign_numeric_start_index..minus_sign_numeric_end]).expect("skipped negative number digit(s)");
                let numeric_value: i64 = match numeric_string.parse() {
                    Ok(i) => i,
                    Err(_parse_int_error) => return Err(JsonParseFailure::NumberParseError),
                };
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::Number(numeric_value)))?;
            } else if data[current_data_index] >= b'0' && data[current_data_index] < b'9' {
                // positive number
                let numeric_start_index = current_data_index;
                current_data_index += 1;
                skip_numeric(&mut current_data_index, data)?;
                let numeric_after_index = current_data_index;
                let numeric_string = core::str::from_utf8(&data[numeric_start_index..numeric_after_index]).expect("skipped positive number digit(s)");
                let numeric_value: i64 = match numeric_string.parse() {
                    Ok(i) => i,
                    Err(_parse_int_error) => return Err(JsonParseFailure::NumberParseError),
                };
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::Number(numeric_value)))?;
            } else {
                return Err(JsonParseFailure::InvalidStructure);
            }
        }
    }
    Err(JsonParseFailure::Incomplete)
}

const fn get_required_escape_sequence(c: char) -> Option<&'static str> {
    Some(match c {
        '"' => r#"\""#, // quotation mark
        '\\' => r#"\\"#, // reverse solidus
        '/' => r#"\/"#, // solidus
        '\u{0008}' =>  r#"\b"#, // backspace
        '\u{000C}' =>  r#"\f"#, // form feed
        '\n' =>  r#"\n"#, // line feed
        '\r' => r#"\r"#, // carriage return
        '\t' => r#"\t"#, // character tabulation
        _ => return None,
    })
}

const fn get_required_unescaped_char(c: char) -> Option<char> {
    Some(match c {
        '"' => '"', // quotation mark
        '\\' => '\\', // reverse solidus
        '/' => '/', // solidus
        'b' => '\u{0008}', // backspace
        'f' => '\u{000C}', // form feed
        'n' => '\n', // line feed
        'r' => '\r', // carriage return
        't' => '\t', // character tabulation
        _ => return None,
    })
}

fn unescape_json_string<'data,'escaped>(index: &mut usize, data: &[u8], escaped: &mut StringBuffer<'escaped>) -> Result<&'escaped str,JsonParseFailure> {
    if data[*index] != b'\"' {
        return Err(JsonParseFailure::InvalidStringField);
    }
    *index += 1;
    let mut current_char_escaped = false;
    let mut encoding_buffer = [0_u8; 4];
    while *index < data.len() {
        let current_char = data[*index];
        if !current_char.is_ascii() {
            return Err(JsonParseFailure::InvalidStringField);
        } else if current_char_escaped {
            if let Some(unescaped_char) = get_required_unescaped_char(current_char as char) {
                let encoded = unescaped_char.encode_utf8(&mut encoding_buffer);
                escaped.write_part(&encoded)?;
                *index += 1;
                current_char_escaped = false;
            } else {
                return Err(JsonParseFailure::InvalidStringField);
            }
        } else if current_char == '\\' as u8 {
            current_char_escaped = true;
            *index += 1;
        } else if current_char == '"' as u8 {
            *index += 1;
            return Ok(escaped.consume_string());
        } else {
            let encoded = (current_char as char).encode_utf8(&mut encoding_buffer);
            escaped.write_part(&encoded)?;
            *index += 1;
        }
        // else if '\\' as u8 == current_char {
        //     if current_char_escaped {
        //         escaped.write_part("\\")?;
        //         current_char_escaped = false;
        //     } else {
        //         current_char_escaped = true;
        //     }
        // } else if '"' as u8 == current_char {
        //     if current_char_escaped {
        //         escaped.write_part(r#"""#)?;
        //         current_char_escaped = false;
        //     } else {
        //         *index += 1;
        //         return Ok(escaped.consume_string());
        //     }
        // } else if let Some(escape_sequence) = escape_char(current_char as char) {
        //     if !current_char_escaped {
        //         return Err(JsonParseFailure::InvalidStringField);
        //     }
        //     let mut char_buffer = [0_u8; 4];
        //     let char_as_str = (current_char as char).encode_utf8(&mut char_buffer);
        //     escaped.write_part(char_as_str)?;
        //     *index += char_as_str.len();
        //     current_char_escaped = false;
        // } else {
        //     let mut char_buffer = [0_u8; 4];
        //     let char_as_str = (current_char as char).encode_utf8(&mut char_buffer);
        //     escaped.write_part(char_as_str)?;
        //     *index += char_as_str.len();
        //     current_char_escaped = false;
        // }
    }
    Err(JsonParseFailure::Incomplete)
}

const fn skip_numeric(index: &mut usize, data: &[u8]) -> Result<(),JsonParseFailure> {
    while *index < data.len() && data[*index] <= b'9' && data[*index] >= b'0' {
        *index += 1;
    }
    if *index == data.len() {
        Err(JsonParseFailure::Incomplete)
    } else if data[*index].is_ascii_whitespace() || data[*index] == b',' || data[*index] == b'}' {
        Ok(())
    } else {
        Err(JsonParseFailure::InvalidNumericField)
    }
}

fn skip_literal(index: &mut usize, data: &[u8], target: &str, field_error_type: JsonParseFailure) -> Result<(),JsonParseFailure> {
    let start = *index;
    while (*index - start) < target.len() {
        if *index >= data.len() {
            return Err(JsonParseFailure::Incomplete)
        }
        if data[*index] != target.as_bytes()[*index-start] {
            return Err(field_error_type);
        }
        *index += 1;
    }
    Ok(())
}

fn skip_whitespace(index: &mut usize, data: &[u8]) -> Result<(),JsonParseFailure> {
    while *index < data.len() && data[*index].is_ascii_whitespace() {
        *index += 1;
    }
    if *index == data.len() {
        Err(JsonParseFailure::Incomplete)
    } else {
        Ok(())
    }
}

/// the core function that powers serialization in the JsonArray API. It attempts to serialize the provided values as a JSON array into the provided output & returns the number of bytes written on success.
pub fn serialize_json_array<'data, Output: StringWrite>(
    output: &mut Output,
    fields: &[JsonValue<'data>],
    resume_from: usize,
) -> Result<usize, (usize,Output::StringWriteFailure)> {
    let mut ret = 0;
    tracked_write(output,&mut ret , &resume_from, LEFT_SQUARE_BRACKET)?;
    let mut value_needs_comma = false;
    for value in fields.as_ref().iter() {
        if value_needs_comma {
            tracked_write(output,&mut ret , &resume_from, ",")?;
        } else {
            value_needs_comma = true;
        }
        match *value {
            JsonValue::Boolean(b) => if b {
                tracked_write(output,&mut ret , &resume_from, "true")?;
            } else {
                tracked_write(output,&mut ret , &resume_from, "false")?;
            },
            JsonValue::Null => {
                tracked_write(output,&mut ret , &resume_from, "null")?;
            },
            JsonValue::Number(n) => {
                tracked_write(output,&mut ret , &resume_from, base10::i64(n).as_str())?;
            },
            JsonValue::String(s) => {
                write_escaped_json_string(output, &mut ret , &resume_from, s)?;
            },
        }
    }
    tracked_write(output, &mut ret , &resume_from, RIGHT_SQUARE_BRACKET)?;
    Ok(ret.saturating_sub(resume_from))
}

// const LEFT_SQUARE_BRACKET_CHAR: char = '{';
const LEFT_SQUARE_BRACKET: &str = "[";
const LEFT_CURLY_BRACKET: &str = "{";
const RIGHT_SQUARE_BRACKET: &str = "]";
const RIGHT_CURLY_BRACKET: &str = "}";
const COLON: &str = ":";
const COMMA: &str = ",";

/// the core function that powers serialization in the JsonObject API. It attempts to serialize the provided fields as a JSON object into the provided output, & returns the number of bytes written on success.
pub fn serialize_json_object<'data, Output: StringWrite>(
    output: &mut Output,
    fields: &[JsonField<'data,'data>],
    resume_from: usize,
) -> Result<usize, (usize,Output::StringWriteFailure)> {
    let mut ret = 0;
    tracked_write(output,&mut ret , &resume_from, LEFT_CURLY_BRACKET)?;
    let mut field_needs_comma = false;
    for field in fields.as_ref().iter() {
        if field_needs_comma {
            tracked_write(output,&mut ret , &resume_from, COMMA)?;
        } else {
            field_needs_comma = true;
        }
        write_escaped_json_string(output, &mut ret , &resume_from, field.key)?;
        tracked_write(output, &mut ret, &resume_from, COLON)?;
        match field.value {
            JsonValue::Boolean(b) => if b {
                tracked_write(output,&mut ret , &resume_from, "true")?;
            } else {
                tracked_write(output,&mut ret , &resume_from, "false")?;
            },
            JsonValue::Null => {
                tracked_write(output,&mut ret , &resume_from, "null")?;
            },
            JsonValue::Number(n) => {
                tracked_write(output,&mut ret , &resume_from, base10::i64(n).as_str())?;
            },
            JsonValue::String(s) => {
                write_escaped_json_string(output, &mut ret , &resume_from, s)?;
            },
        }
    }
    tracked_write(output, &mut ret, &resume_from, RIGHT_CURLY_BRACKET)?;
    Ok(ret.saturating_sub(resume_from))
}

fn tracked_write<T: StringWrite>(output: &mut T, counter: &mut usize, resume_from: &usize, the_string: &str) -> Result<(), (usize,T::StringWriteFailure)> {
    let mut encoding_buffer = [0_u8; 4];
    for char in the_string.chars() {
        let encoded_char = char.encode_utf8(encoding_buffer.as_mut_slice());
        let to_skip = if resume_from <= counter {
            0
        } else {
            let to_skip = *resume_from - *counter;
            if to_skip >= encoded_char.len() {
                *counter += encoded_char.len();
                continue;
            } else {
                to_skip
            }
        };
        match output.write_char(char, to_skip) {
            Ok(n_success) => *counter += n_success,
            Err((n_failed, e)) => {
                *counter += n_failed;
                return Err((counter.saturating_sub(*resume_from), e));
            },
        };
    }
    Ok(())
}

fn write_escaped_json_string<T: StringWrite>(output: &mut T, counter: &mut usize, resume_from: &usize, data: &str) -> Result<(), (usize,T::StringWriteFailure)> {
    tracked_write(output, counter, resume_from, "\"")?;
    for field_character in data.chars() {
        if !field_character.is_ascii() {
            continue;
        } else if let Some(escape_sequence) = get_required_escape_sequence(field_character) {
            tracked_write(output, counter, resume_from, escape_sequence)?;
        } else {
            tracked_write(output, counter, resume_from, field_character.encode_utf8(&mut [0_u8; 4]))?;
        }
    }
    tracked_write(output, counter, resume_from, "\"")?;
    Ok(())
}

#[cfg(feature = "alloc")]
mod alloc {

    extern crate alloc as alloclib;
    

    use alloclib::string::String;
    use alloclib::vec::Vec;

    pub use elsa::FrozenVec;

    use crate::{parse_json_object, JsonField, JsonObject, JsonParseFailure, ParseBuffer, StringBuffer};

    impl <'a, T: AsMut<Vec<JsonField<'a,'a>>>> JsonObject<T> {

        /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing parsed fields
        /// returns num bytes consumed on success
        pub fn parse_alloc_fields(&mut self, data: &'a [u8], escape_buffer: &'a mut [u8]) -> Result<usize,JsonParseFailure> {
            let (data_end, parsed_fields) = parse_json_object(
                data,
                ParseBuffer::Infinite(0, self.fields.as_mut()),
                &mut StringBuffer::Finite(0, escape_buffer),
            )?;
            let new_num_fields = parsed_fields;
            self.num_fields = new_num_fields;
            Ok(data_end)
        }

        /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing parsed fields & escaped strings
        /// returns num bytes consumed on success
        pub fn parse_alloc(&mut self, data: &'a [u8], escape_buffer: &'a FrozenVec<String>) -> Result<usize,JsonParseFailure> {
            let (data_end, parsed_fields) = parse_json_object(
                data,
                ParseBuffer::Infinite(0, self.fields.as_mut()),
                &mut crate::StringBuffer::Infinite(String::new(), escape_buffer),
            )?;
            let new_num_fields = parsed_fields;
            self.num_fields = new_num_fields;
            Ok(data_end)
        }
    }

}


#[cfg(feature = "std")]
mod stdlib {
    extern crate std;
    use embedded_io_adapters::std::FromStd;
    use crate::FieldBuffer;
    use crate::JsonObject;

    impl <'a,T: FieldBuffer<'a>> JsonObject<T> {
        /// convenience method to serialize to types implementing std::io::Write by wrapping it with embedded_io_adapters::std::FromStd
        pub fn serialize_std<Output: std::io::Write>(&self, output: Output) -> Result<usize,std::io::Error> {
            self.serialize(FromStd::new(output))
        }
    }
}

#[cfg(all(test,feature = "alloc"))]
mod test_alloc {
    use super::*;

    extern crate alloc;
    use alloc::vec::Vec;
    use alloclib::string::ToString;

    #[test]
    fn test_parse_core_vec_no_alloc_too_many_fields() {
        match parse_json_object(
            br#"{"a":0}"#,
            ParseBuffer::Finite(0,&mut Vec::new()),
            &mut StringBuffer::Finite(0, &mut [0_u8; 256]),
        ) {
            Err(JsonParseFailure::FieldBufferTooSmall) => {},
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_parse_core_vec_with_alloc_simple() {
        let mut fields = Vec::new();
        match parse_json_object(
            br#"{"a":0}"#,
            ParseBuffer::Infinite(0,&mut fields),
            &mut StringBuffer::Finite(0, &mut [0_u8; 256])
        ) {
            Ok((num_bytes, num_fields)) => {
                assert_eq!(7, num_bytes);
                assert_eq!(1, num_fields);
                assert_eq!(1, fields.len());
                assert_eq!(JsonField::new("a", JsonValue::Number(0)), fields[0])
            },
            other => panic!("{:?}", other),
        }

    }

    #[test]
    fn test_parse_core_vec_success_empty() {
        let (bytes_consumed,num_fields_parsed) = parse_json_object(
            b"{}",
            ParseBuffer::Infinite(0,&mut Vec::new()),
            &mut StringBuffer::Finite(0, &mut [0_u8; 256])
        ).unwrap();
        assert_eq!(2,bytes_consumed);
        assert_eq!(0,num_fields_parsed);
    }

    #[test]
    fn test_parse_object_vec_success_empty() {
        let mut escape_buffer = [0_u8; 256];
        let mut parser = JsonObject::wrap(Vec::new());
        let bytes_consumed =  parser.parse(b"{}", &mut escape_buffer).unwrap();
        assert_eq!(0,parser.fields().len());
        assert_eq!(bytes_consumed, 2);
    }

    #[test]
    fn test_serialize_empty_to_string() {
        let string: String = ArrayJsonObject::<0>::new().to_string();
        assert_eq!("{}", string);
    }


}

#[cfg(test)]
mod test_core {

    use embedded_io::SliceWriteError;

    use super::*;

    #[test]
    fn test_parse_value_string() {
        let data = br#""this is a string""#;
        match JsonValue::parse(data, &mut [0_u8; 16]) {
            Ok((value_end,value)) => {
                assert_eq!(data.len(),value_end);
                match value {
                    JsonValue::String(s) => {
                        assert_eq!("this is a string", s);
                    },
                    other => panic!("{:?}", other),
                }
            },
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_parse_value_integer() {
        let data = br#"12345 "#;
        match JsonValue::parse(data, &mut [0_u8; 16]) {
            Ok((value_end,value)) => {
                assert_eq!(data.len(),value_end+1); // need non-numeric to recognize end
                match value {
                    JsonValue::Number(n) => {
                        assert_eq!(12345, n);
                    },
                    other => panic!("{:?}", other),
                }
            },
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_parse_value_null() {
        let data = br#"null"#;
        match JsonValue::parse(data, &mut [0_u8; 16]) {
            Ok((value_end,value)) => {
                assert_eq!(data.len(),value_end);
                match value {
                    JsonValue::Null => {},
                    other => panic!("{:?}", other),
                }
            },
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn test_parse_object_empty_core() {
        let mut escape_buffer = [0_u8; 256];
        let (bytes_consumed,num_fields) = parse_json_object(
            b"{}",
            ParseBuffer::Finite(0,&mut []),
            &mut StringBuffer::Finite(0, &mut escape_buffer),
        ).unwrap();
        assert_eq!(bytes_consumed, 2);
        assert_eq!(num_fields, 0);
    }

    #[test]
    fn test_parse_object_empty_trait_array() {
        let mut parser = JsonObject::wrap([]);
        let bytes_consumed = parser.parse(b"{}", &mut []).unwrap();
        assert_eq!(bytes_consumed, 2);
        assert_eq!(parser.len(), 0);
    }

    #[test]
    fn test_parse_object_empty_trait_slice() {
        let mut parser = JsonObject::wrap(&mut []);
        let bytes_consumed = parser.parse(b"{}", &mut []).unwrap();
        assert_eq!(bytes_consumed, 2);
        assert_eq!(parser.len(), 0);
    }

    #[test]
    fn test_parse_object_empty_arrayhelper() {
        let mut parser = ArrayJsonObject::<0>::new();
        let bytes_consumed = parser.parse(b"{}", &mut []).unwrap();
        assert_eq!(bytes_consumed, 2);
        assert_eq!(parser.len(), 0);
    }

    #[test]
    fn test_parse_object_simple() {
        let data = br#"{"sub":"1234567890","name":"John Doe","iat":1516239022,"something":false,"null_thing":null}"#;
        let mut escape_buffer = [0_u8; 256];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(5, test_fields.len());
        assert_eq!(JsonField { key: "sub", value: JsonValue::String("1234567890")}, test_fields[0]);
        assert_eq!(JsonField { key: "name", value: JsonValue::String("John Doe")}, test_fields[1]);
        assert_eq!(JsonField { key: "iat", value: JsonValue::Number(1516239022)}, test_fields[2]);
        assert_eq!(JsonField { key: "something", value: JsonValue::Boolean(false)}, test_fields[3]);
        assert_eq!(JsonField { key: "null_thing", value: JsonValue::Null}, test_fields[4]);
    }

    #[test]
    fn test_parse_object_empty_strings() {
        let data = br#"{"":""}"#;
        let mut escape_buffer = [0_u8; 0];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(1, test_fields.len());
        assert_eq!(JsonField { key: "", value: JsonValue::String("")}, test_fields[0]);
    }

    #[test]
    fn test_parse_object_escape_backspace() {
        let data = br#"{"\b":null}"#;
        let mut escape_buffer = [0_u8; 1];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(1, test_fields.len());
        assert_eq!(JsonField { key: "\u{0008}", value: JsonValue::Null}, test_fields[0]);
    }

    #[test]
    fn test_parse_object_escape_newline() {
        let data = br#"{"\n":null}"#;
        let mut escape_buffer = [0_u8; 1];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(1, test_fields.len());
        assert_eq!(JsonField { key: "\n", value: JsonValue::Null}, test_fields[0]);
    }

    #[test]
    fn test_parse_object_escape_carriage_return() {
        let data = br#"{"\r":null}"#;
        let mut escape_buffer = [0_u8; 1];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(1, test_fields.len());
        assert_eq!(JsonField { key: "\r", value: JsonValue::Null}, test_fields[0]);
    }

    #[test]
    fn test_parse_object_escape_quote() {
        let data = br#"{"\"":null}"#;
        let mut escape_buffer = [0_u8; 1];
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data, &mut escape_buffer).unwrap();
        assert_eq!(data_end, data.len());
        let test_fields = json_object.fields();
        assert_eq!(1, test_fields.len());
        assert_eq!(JsonField { key: "\"", value: JsonValue::Null}, test_fields[0]);
    }

    #[test]
    fn test_parse_object_ignore_trailing_whitespace() {
        let data = br#"{}    "#; // add 4 spaces to the end
        let (data_end,_) = ArrayJsonObject::<0>::new_parsed(data,&mut []).unwrap();
        assert_eq!(data_end, data.len() - 4);
    }

    #[test]
    fn test_parse_object_failure_too_many_fields() {
        match ArrayJsonObject::<0>::new_parsed(br#"{"some":"thing"}"#, &mut [0_u8; 256]) {
            Err(JsonParseFailure::FieldBufferTooSmall) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_invalid_number_minus() {
        match ArrayJsonObject::<1>::new_parsed(br#"{"": -}"#, &mut []) {
            Err(JsonParseFailure::InvalidNumericField) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_incomplete_a() {
        match ArrayJsonObject::<0>::new_parsed(b"{",&mut []) {
            Err(JsonParseFailure::Incomplete) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_incomplete_b() {
        let mut escape_buffer = [0_u8; 256];
        match ArrayJsonObject::<50>::new_parsed(
            br#"{"sub":"1234567890","name":"John Doe","iat":1516239022,"something":false"#,
            &mut escape_buffer,
        ) {
            Err(JsonParseFailure::Incomplete) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_serialize_array_empty() {
        let mut buffer = [0_u8; 2];
        let test_array = ArrayJsonArray::<0>::new();
        let n = test_array.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"[]", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_array_empty() {
        let mut buffer = [0_u8; 2];
        let test_array = ArrayJsonArray::<0>::new();
        let n = test_array.serialize_resume(buffer.as_mut_slice(),1).unwrap();
        assert_eq!(b"]", buffer.split_at(n).0)
    }

    #[test]
    fn test_display_array_empty() {
        let mut buffer = [0_u8; 2];
        buffer.as_mut_slice().write_fmt(format_args!("{}", ArrayJsonArray::<0>::new())).unwrap();
        assert_eq!(b"[]", buffer.as_slice())
    }

    #[test]
    fn test_serialize_object_empty() {
        let mut buffer = [0_u8; 2];
        let test_object = ArrayJsonObject::<0>::new();
        let n = test_object.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{}", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_object_empty() {
        let mut buffer = [0_u8; 2];
        let test_object = ArrayJsonObject::<0>::new();
        let n = test_object.serialize_resume(buffer.as_mut_slice(), 1).unwrap();
        assert_eq!(b"}", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_skip_object_empty() {
        let mut buffer = [0_u8; 2];
        let test_object = ArrayJsonObject::<0>::new();
        let n = test_object.serialize_resume(buffer.as_mut_slice(), 2).unwrap();
        assert_eq!(b"", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_too_many_object_empty() {
        let mut buffer = [0_u8; 2];
        let test_object = ArrayJsonObject::<0>::new();
        let n = test_object.serialize_resume(buffer.as_mut_slice(), 3).unwrap();
        assert_eq!(b"", buffer.split_at(n).0)
    }

    #[test]
    fn test_display_object_empty() {
        let mut buffer = [0_u8; 2];
        buffer.as_mut_slice().write_fmt(format_args!("{}", ArrayJsonObject::<0>::new())).unwrap();
        assert_eq!(b"{}", buffer.as_slice())
    }

    #[test]
    fn test_serialize_object_simple() {
        let mut buffer = [0_u8; 1000];
        let mut test_map = ArrayJsonObject::<50>::new();
        test_map.push_field("sub", JsonValue::String("1234567890")).unwrap();
        test_map.push_field("name", JsonValue::String("John Doe")).unwrap();
        test_map.push_field("iat", JsonValue::Number(1516239022)).unwrap();
        test_map.push_field("something", JsonValue::Boolean(false)).unwrap();
        test_map.push_field("null_thing", JsonValue::Null).unwrap();
        let n = test_map.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(br#"{"sub":"1234567890","name":"John Doe","iat":1516239022,"something":false,"null_thing":null}"#, buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_object_simple() {
        const SKIP: usize = 10;
        const EXPECTED: &[u8] = br#"{"sub":"1234567890","name":"John Doe","iat":1516239022,"something":false,"null_thing":null}"#.split_at(SKIP).1;

        let mut buffer = [0_u8; 1000];
        let mut test_map = ArrayJsonObject::<50>::new();
        test_map.push_field("sub", JsonValue::String("1234567890")).unwrap();
        test_map.push_field("name", JsonValue::String("John Doe")).unwrap();
        test_map.push_field("iat", JsonValue::Number(1516239022)).unwrap();
        test_map.push_field("something", JsonValue::Boolean(false)).unwrap();
        test_map.push_field("null_thing", JsonValue::Null).unwrap();
        let n = test_map.serialize_resume(buffer.as_mut_slice(), 10).unwrap();
        assert_eq!(EXPECTED, buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_resume_object_single_byte() {
        const EXPECTED: &[u8] = br#"{"sub":"1234567890","name":"John Doe","iat":1516239022,"something":false,"null_thing":null}"#;

        let mut buffer = [0_u8; 1];
        let mut test_map = ArrayJsonObject::<50>::new();
        test_map.push_field("sub", JsonValue::String("1234567890")).unwrap();
        test_map.push_field("name", JsonValue::String("John Doe")).unwrap();
        test_map.push_field("iat", JsonValue::Number(1516239022)).unwrap();
        test_map.push_field("something", JsonValue::Boolean(false)).unwrap();
        test_map.push_field("null_thing", JsonValue::Null).unwrap();

        // attempt to resume from every each byte
        for (index,expected_byte) in EXPECTED.iter().enumerate() {
            match test_map.serialize_resume(buffer.as_mut_slice(), index) {
                Err((1,SliceWriteError::Full)) => {
                    assert_eq!(*expected_byte as char, buffer[0] as char)
                },
                Ok(0) => assert_eq!(EXPECTED.len(),index),
                Ok(1) => assert_eq!(EXPECTED.len()-1,index),
                unexpected => panic!("{:?}", unexpected),
            };
        }
    }

}
