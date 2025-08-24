#![no_std]
#![forbid(unsafe_code)]

use core::fmt::{Debug, Display, Formatter};
use embedded_io::{Write};
use numtoa::base10;
#[cfg(feature = "alloc")]
use elsa::FrozenVec;

trait StringWrite {
    type StringWriteFailure: Debug;
    fn write_string(&mut self, data: &str) -> Result<(),(usize,Self::StringWriteFailure)>;
}

impl<X: Write> StringWrite for X {
    type StringWriteFailure = X::Error;
    fn write_string(&mut self, mut data: &str) -> Result<(),(usize,Self::StringWriteFailure)> {
        let mut written = 0_usize;
        loop {
            if data.is_empty() {
                return Ok(());
            }
            let n = match self.write(data.as_bytes()) {
                Ok(0) => panic!("zero write"),
                Err(e) => return Err((written,e)),
                Ok(n) => n,
            };
            written += n;
            data = data.split_at(n).1
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
        fn write_string(&mut self, data: &str) -> Result<(),(usize,Self::StringWriteFailure)> {
            match self.inner.write_str(data) {
                Ok(()) => {
                    Ok(())
                },
                Err(e) => {
                    Err((0,e))
                },
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


/// trait for an optionally mutable collection of JSON object fields
pub trait FieldBuffer<'data>: AsRef<[JsonField<'data,'data>]> {

    /// convenience one-liner to call JsonObject::wrap_init on this Sized type, consuming it
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

/// a default JSON field with static lifetime
pub const EMPTY_FIELD: JsonField<'static,'static> = JsonField{ key: "", value: JsonValue::Number(0)};

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

/// JsonObject represents an RFC 8259 JSON Array. Tt wraps a mutable or immutable buffer of JSON values.
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

    pub fn serialize<Output: Write>(&self, _output: Output) -> Result<usize,Output::Error> {
        todo!()
        // serialize_json_object(output, self.values.as_ref())
    }

}

/// JsonObject represents an RFC 8259 JSON Object. Tt wraps a mutable or immutable buffer of object fields. The easiest way to use it is through the ArrayJsonObject type alias, however you can use JsonObject directly to wrap your own buffer like a heap allocated Vec
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

    /// attempt to serialize this JsonObject into the provided output, returns the number of bytes written on success
    pub fn serialize<Output: Write>(&self, mut output: Output) -> Result<usize,Output::Error> {
        serialize_json_object_internal(&mut output, self.fields().as_ref())
    }
}

impl <'a,T: FieldBuffer<'a>> Display for JsonObject<T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
        match serialize_json_object_internal(
            &mut FormatWrapper::new(fmt),
            self.fields.as_ref(),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(e)
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
            ResultBuffer::Finite(0, self.fields.as_mut()),
            StringBuffer::Finite(0, string_escape_buffer),
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

/// a buffer that T can be written to
pub enum ResultBuffer<'a,T> {
    Finite(usize, &'a mut [T]),
    #[cfg(feature = "alloc")]
    Growable(usize,&'a mut Vec<T>)
}

impl<'a,T> ResultBuffer<'a,T> {

    fn write_thing(&mut self, thing: T) -> Result<(),JsonParseFailure> {
        match self {
            ResultBuffer::Finite(position, slice) => {
                if *position == (*slice).len() {
                    Err(JsonParseFailure::FieldBufferTooSmall)
                } else {
                    slice[*position] = thing;
                    *position += 1;
                    Ok(())
                }
            },
            #[cfg(feature = "alloc")]
            ResultBuffer::Growable(position,vec) => {
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
            ResultBuffer::Finite(n, _) => n,
            #[cfg(feature = "alloc")]
            ResultBuffer::Growable(n, _) => n,
        }
    }
}

/// a buffer that string slices can be written to
pub enum StringBuffer<'a> {
    Finite(usize, &'a mut [u8]),
    #[cfg(feature = "alloc")]
    Infinite(String,&'a FrozenVec<String>),
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
            StringBuffer::Infinite(current_string, all_strings) => {
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
                core::str::from_utf8(ret).unwrap()
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
    mut field_buffer: ResultBuffer<'_,JsonField<'escaped_data,'escaped_data>>,
    mut string_escape_buffer: StringBuffer<'escaped_data>,
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
            let key_start_quote_index = current_data_index;
            current_data_index += 1;
            skip_json_string(&mut current_data_index, data)?;
            let key_end_quote_index = current_data_index;
            let string_key = core::str::from_utf8(&data[key_start_quote_index+1..key_end_quote_index]).expect("skipped json object key string");
            current_data_index += 1;
            skip_whitespace(&mut current_data_index, data)?;
            if data[current_data_index] != b':' {
                return Err(JsonParseFailure::InvalidStructure);
            }
            current_data_index += 1;
            skip_whitespace(&mut current_data_index, data)?;

            if data[current_data_index] == b'"' {
                let value_start_quote_index = current_data_index;
                current_data_index += 1;
                skip_json_string(&mut current_data_index, data)?;
                let value_end_quote_index = current_data_index;
                current_data_index += 1;
                let unescaped_string_value = core::str::from_utf8(&data[value_start_quote_index+1..value_end_quote_index]).expect("skipped json object value string");
                // TODO: escape
                string_escape_buffer.write_part(unescaped_string_value)?;
                let escaped_string = string_escape_buffer.consume_string();
                field_buffer.write_thing(JsonField::new(string_key, JsonValue::String(escaped_string)))?;
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

fn skip_json_string(index: &mut usize, data: &[u8]) -> Result<(),JsonParseFailure> {
    let mut last_char_escape = false;
    while *index < data.len() {
        if data[*index] == b'\\' && !last_char_escape {
            last_char_escape = true;
        } else if data[*index] == b'"' && !last_char_escape {
            return Ok(());
        } else if !data[*index].is_ascii() {
            return Err(JsonParseFailure::InvalidStringField);
        } else {
            last_char_escape = false
        }
        *index += 1;
    }
    Err(JsonParseFailure::Incomplete)
}

fn skip_numeric(index: &mut usize, data: &[u8]) -> Result<(),JsonParseFailure> {
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

/// the core function that powers serialization in the JsonObject API. It attempts to serialize the provided fields as a JSON object into the provided output, & returns the number of bytes written on success.
pub fn serialize_json_object<'data, Output: Write>(
    output: &mut Output,
    fields: &[JsonField<'data,'data>],
) -> Result<usize, Output::Error> {
    serialize_json_object_internal(output, fields)
}

fn serialize_json_object_internal<'data, Output: StringWrite>(
    output: &mut Output,
    fields: &[JsonField<'data,'data>],
) -> Result<usize, Output::StringWriteFailure> {
    let mut ret = 0;
    tracked_write(output,&mut ret , "{")?;
    let mut field_needs_comma = false;
    for field in fields.as_ref().iter() {
        if field_needs_comma {
            tracked_write(output,&mut ret , ",")?;
        } else {
            field_needs_comma = true;
        }
        write_escaped_json_string(output, &mut ret , field.key)?;
        tracked_write(output, &mut ret , ":")?;
        match field.value {
            JsonValue::Boolean(b) => if b {
                tracked_write(output,&mut ret , "true")?;
            } else {
                tracked_write(output,&mut ret , "false")?;
            },
            JsonValue::Null => {
                tracked_write(output,&mut ret , "null")?;
            },
            JsonValue::Number(n) => {
                tracked_write(output,&mut ret , base10::i64(n).as_str())?;
            },
            JsonValue::String(s) => {
                write_escaped_json_string(output, &mut ret , s)?;
            },
        }
    }
    tracked_write(output, &mut ret , "}")?;
    Ok(ret)
}

fn tracked_write<T: StringWrite>(output: &mut T, counter: &mut usize, data: &str) -> Result<(), T::StringWriteFailure> {
    match output.write_string(data) {
        Ok(()) => {
            *counter += data.len();
            Ok(())
        },
        Err((partial, e)) => {
            *counter += partial;
            Err(e)
        },
    }
}

fn write_escaped_json_string<T: StringWrite>(output: &mut T, counter: &mut usize, data: &str) -> Result<(), T::StringWriteFailure> {
    tracked_write(output, counter, "\"")?;
    for field_character in data.chars() {
        if field_character == '"' || field_character == '\\' {
            tracked_write(output, counter, "\\")?;
        }
        tracked_write(output, counter, field_character.encode_utf8(&mut [0_u8; 4]))?;
    }
    tracked_write(output, counter, "\"")?;
    Ok(())
}

#[cfg(feature = "alloc")]
mod alloc {

    extern crate alloc as alloclib;
    use core::{convert::Infallible, fmt::{Arguments, Formatter}, marker::PhantomData};

    use alloclib::string::String;
    use alloclib::vec::Vec;

    pub use elsa::FrozenVec;

    use crate::{parse_json_object, FieldBufferMut, JsonField, JsonObject, JsonParseFailure, ResultBuffer, StringWrite};

    // struct StringWrapper {
    //     inner: String,
    // }


    // impl StringWrite for StringWrapper {
    //     type StringWriteFailure = core::convert::Infallible;
    //     fn write_string(&mut self, data: &str) -> Result<(),(usize,Self::StringWriteFailure)> {
    //         let n = data.len();
    //         self.inner.push_str(data);
    //         Ok(())
    //     }
    // }

    impl <'a, T: FieldBufferMut<'a>> JsonObject<T> {
        /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing escaped strings
        /// returns num bytes consumed on success
        pub fn parse_alloc_buffer(&mut self, data: &'a [u8], escape_buffer: &'a FrozenVec<String>) -> Result<usize,JsonParseFailure> {
            let (data_end, parsed_fields) = parse_json_object(
                data,
                ResultBuffer::Finite(0,self.fields.as_mut()),
                crate::StringBuffer::Infinite(String::new(), escape_buffer)
            )?;
            let new_num_fields = parsed_fields;
            self.num_fields = new_num_fields;
            Ok(data_end)
        }
    }

    impl <'a, T: AsMut<Vec<JsonField<'a,'a>>>> JsonObject<T> {

        /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing parsed fields
        /// returns num bytes consumed on success
        pub fn parse_alloc_fields(&mut self, data: &'a [u8], escape_buffer: &'a mut [u8]) -> Result<usize,JsonParseFailure> {
            let (data_end, parsed_fields) = parse_json_object(
                data,
                ResultBuffer::Growable(0, self.fields.as_mut()),
                crate::StringBuffer::Finite(0, escape_buffer),
            )?;
            let new_num_fields = parsed_fields;
            self.num_fields = new_num_fields;
            Ok(data_end)
        }

        // /// attempt to parse a JSON object from the provided data slice and write its fields into this JsonObject while allocating space as needed for storing parsed fields & escaped strings
        // /// returns num bytes consumed on success
        // pub fn parse_alloc(&mut self, data: &'a [u8]) -> Result<usize,JsonParseFailure> {
        //     let (data_end, parsed_fields) = parse_json_object(data, ResultBuffer::Growable(0, self.fields.as_mut()))?;
        //     let new_num_fields = parsed_fields;
        //     self.num_fields = new_num_fields;
        //     Ok(data_end)
        // }
    }

    // impl<V> JsonBuffer<V> for Vec<V> {}
    // impl<V> JsonBuffer<V> for &Vec<V> {}
    // impl<V> JsonBuffer<V> for &mut Vec<V> {}
    // impl<V> JsonBufferMut<V> for Vec<V> {
    //     fn set_or_push_value(&mut self, n: usize, value: V) -> Result<(), JsonParseFailure> {
    //         assert!(n <= self.len());
    //         if n == self.len() {
    //             self.push(value);
    //         } else {
    //             self[n] = value;
    //         }
    //         Ok(())
    //     }
    // }
    // impl<V> JsonBufferMut<V> for &mut Vec<V> {
    //     fn set_or_push_value(&mut self, n: usize, value: V) -> Result<(), JsonParseFailure> {
    //         assert!(n <= self.len());
    //         if n == self.len() {
    //             self.push(value);
    //         } else {
    //             self[n] = value;
    //         }
    //         Ok(())
    //     }
    // }


}

#[cfg(feature = "std")]
mod std {
    extern crate std as stdlib;
    use embedded_io_adapters::std::FromStd as StdIoAdapter;
    use stdlib::io::Error as StandardLibIoError;
    use stdlib::io::Write as StandardLibIoWrite;

    use crate::FieldBuffer;
    use crate::JsonObject;

    impl <'a,T: FieldBuffer<'a>> JsonObject<T> {
        /// convenience method to serialize after wrapping std::io::Write with embedded_io_adapters::std::FromStd
        pub fn serialize_std<Output: StandardLibIoWrite>(&self, output: Output) -> Result<usize,StandardLibIoError> {
            self.serialize(StdIoAdapter::new(output))
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
            ResultBuffer::Finite(0,&mut Vec::new()),
            StringBuffer::Finite(0, &mut [0_u8; 256]),
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
            ResultBuffer::Growable(0,&mut fields),
            StringBuffer::Finite(0, &mut [0_u8; 256])
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
            ResultBuffer::Growable(0,&mut Vec::new()),
            StringBuffer::Finite(0, &mut [0_u8; 256])
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

    use super::*;

    #[test]
    fn test_parse_object_empty_core() {
        let mut escape_buffer = [0_u8; 256];
        let (bytes_consumed,num_fields) = parse_json_object(
            b"{}",
            ResultBuffer::Finite(0,&mut []),
            StringBuffer::Finite(0, &mut escape_buffer),
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
    fn test_parse_object_ignore_trailing_whitespace() {
        let data = br#"{}    "#; // add 4 spaces to the end
        let (data_end,_) = ArrayJsonObject::<0>::new_parsed(data,&mut []).unwrap();
        assert_eq!(data_end, data.len() - 4);
    }

    #[test]
    fn test_parse_object_failure_too_many_fields() {
        let mut escape_buffer = [0_u8; 256];
        match ArrayJsonObject::<0>::new_parsed(br#"{"some":"thing"}"#,&mut escape_buffer) {
            Err(JsonParseFailure::FieldBufferTooSmall) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_invalid_number_minus() {
        match ArrayJsonObject::<1>::new_parsed(br#"{"": -}"#,&mut []) {
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
    fn test_serialize_object_empty() {
        let mut buffer = [0_u8; 1000];
        let test_map = ArrayJsonObject::<50>::new();
        let n = test_map.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{}", buffer.split_at(n).0)
    }

    #[test]
    fn test_display_object_empty() {
        let mut buffer = [0_u8; 1000];
        buffer.as_mut_slice().write_fmt(format_args!("{}", ArrayJsonObject::<0>::new())).unwrap();
        assert_eq!(b"{}", buffer.split_at(2).0)
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

}
