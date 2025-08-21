#![no_std]

use embedded_io::Write;
use numtoa::base10;

/// terminal (non-nested) JSON types
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum JsonValue<'a> {
    /// a JSON string - it will be automatically escaped
    String(&'a str),
    /// a JSON boolean
    Boolean(bool),
    /// a JSON number
    Number(i64),
}

/// a field within a JSON object
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub struct JsonField<'a,'b> {
    pub key: &'a str,
    pub value: JsonValue<'b>,
}

impl <'a,'b> JsonField<'a,'b> {
    /// create a new JSON object field with the given key & value
    pub fn new(key: &'a str, value: JsonValue<'b>) -> Self {
        JsonField { key, value }
    }
    /// convenience helper to create a new JSON object string field
    pub fn new_string(key: &'a str, value: &'b str) -> Self {
        Self::new(key, JsonValue::String(value))
    }
    /// convenience helper to create a new JSON object number field
    pub fn new_number(key: &'a str, value: i64) -> Self {
        Self::new(key, JsonValue::Number(value))
    }
    /// convenience helper to create a new JSON object boolean field
    pub fn new_boolean(key: &'a str, value: bool) -> Self {
        Self::new(key, JsonValue::Boolean(value))
    }
}

/// a JSON Object (rfc8259) that wraps a mutable or immutable buffer of object fields. The easiest way to use it is through the ArrayJsonObject type alias, however you can use JsonObject directly to wrap your own buffer (maybe a Vec)
#[derive(Debug)]
pub struct JsonObject<Fields> {
    fields: Fields,
    num_fields: usize,
}

/// the various reasons parsing JSON can fail
#[derive(Debug)]
pub enum JsonParseFailure {
    /// there was no error, but the data slice is incomplete
    Incomplete,
    /// there was no error, but there were more fields than the provided field buffer could hold
    TooManyFields,
    /// there was an error in the JSON structure of the data
    InvalidStructure,
    /// an invalid JSON string was encountered
    InvalidStringField,
    /// an invalid JSON number was encountered
    InvalidNumericField,
    /// an invalid JSON boolean was encountered
    InvalidBooleanField,
}

/// a default JSON field with static lifetime
pub const EMPTY_FIELD: JsonField<'static,'static> = JsonField{ key: "", value: JsonValue::Number(0)};

impl <'a,'b> Default for JsonField<'a,'b> {
    fn default() -> Self {
        EMPTY_FIELD
    }
}

impl<'a,T: FieldBuffer<'a> + Default> Default for JsonObject<T> {
    fn default() -> Self {
        JsonObject { fields: T::default(), num_fields: 0 }
    }
}

/// this trait is automatically implemented for all types that implement AsRef<[JsonField<'data,'data>]>
pub trait FieldBuffer<'data>: AsRef<[JsonField<'data,'data>]> {}
/// this trait is automatically implemented for all types that implement AsMut<[JsonField<'data,'data>]>
pub trait FieldBufferMut<'a>: FieldBuffer<'a> +  AsMut<[JsonField<'a,'a>]> {}

impl <'a,T: AsRef<[JsonField<'a,'a>]>> FieldBuffer<'a> for T {}
impl <'a,T: FieldBuffer<'a> + AsMut<[JsonField<'a,'a>]>> FieldBufferMut<'a> for T {}

impl <'a,T: FieldBuffer<'a>> JsonObject<T> {
    pub const fn wrap(fields: T, num_fields: usize) -> Self {
        JsonObject { fields, num_fields }
    }

    pub fn fields(&self) -> &[JsonField<'a,'a>] {
        self.fields.as_ref().split_at(self.num_fields).0
    }

    pub fn serialize<Output: Write>(&self, output: Output) -> Result<usize,Output::Error> {
        serialize_json_object(output, self.fields())
    }
}

impl<'a,T: FieldBuffer<'a>> From<T> for JsonObject<T> {
    fn from(field_buffer: T) -> Self {
        let num_headers = field_buffer.as_ref().len();
        JsonObject::wrap(field_buffer, num_headers)
    }
}

impl <'a,T: FieldBufferMut<'a>> JsonObject<T> {

    /// get a mutable reference to the initialized fields
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

    /// attempt to parse some data into this JsonObject
    /// returns a tuple of (object end position, num fields found) on success
    pub fn parse(
        &mut self,
        data: &'a [u8],
        ) -> Result<(usize,usize),JsonParseFailure> {
        let (data_end, parsed_fields) = parse_json_object(data, self.fields.as_mut())?;
        let new_num_fields = parsed_fields.len();
        self.num_fields = new_num_fields;
        Ok((data_end,new_num_fields))
    }

}


/// ArrayJsonObject is a type alias for a JsonObject that wraps an array. It is has some additional functionality compared to a normal JsonObject.
pub type ArrayJsonObject<'a,const N: usize> = JsonObject<[JsonField<'a,'a>; N]>;

impl<'a,const N: usize> ArrayJsonObject<'a,N> {

    /// convenience method to wrap a JsonObject over an array
    pub const fn new() -> Self {
        JsonObject::wrap([EMPTY_FIELD; N], 0)
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

    /// convenience method to automatically create an ArrayJsonObject if object parsing is successful
    pub fn new_parsed(data: &'a [u8]) -> Result<(usize,Self),JsonParseFailure> {
        let mut ret = Self::new();
        let (data_end,num_headers) = ret.parse(data)?;
        Ok((data_end,ret))
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

/// the core function that powers parsing in the JsonObject API. It attempts to parse the fields of a json object from the provided data slice into the provided field buffer, then return (data bytes consumed, parsed field slice) on success.
pub fn parse_json_object<'data,'field_buffer>(data: &'data [u8], field_buffer: &'field_buffer mut [JsonField<'data,'data>]) -> Result<(usize,&'field_buffer[JsonField<'data,'data>]),JsonParseFailure> {
    let mut current_data_index = 0;
    let mut current_field_index = 0;
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
            return Ok((current_data_index+1,field_buffer.split_at(current_field_index).0))
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
                let string_value = core::str::from_utf8(&data[value_start_quote_index+1..value_end_quote_index]).expect("skipped json object value string");
                if current_field_index >= field_buffer.len() {
                    return Err(JsonParseFailure::TooManyFields);
                }
                field_buffer[current_field_index] = JsonField::new(string_key, JsonValue::String(string_value));
                current_field_index += 1;
            } else if data[current_data_index] == b't' || data[current_data_index] == b'f' {
                let expect_true = data[current_data_index] == b't';
                skip_json_boolean(&mut current_data_index, data, expect_true)?;
                if current_field_index >= field_buffer.len() {
                    return Err(JsonParseFailure::TooManyFields);
                }
                field_buffer[current_field_index] = JsonField::new(string_key, JsonValue::Boolean(expect_true));
                current_field_index += 1;
            } else if (data[current_data_index] >= b'0' && data[current_data_index] < b'9') || data[current_data_index] == b'-' {
                let numeric_start_index = current_data_index;
                current_data_index += 1;
                skip_json_numeric(&mut current_data_index, data)?;
                let numeric_after_index = current_data_index;
                let numeric_string = core::str::from_utf8(&data[numeric_start_index..numeric_after_index]).expect("skipped numeric digits");
                let numeric_value: i64 = match numeric_string.parse() {
                    Ok(i) => i,
                    Err(_) => return Err(JsonParseFailure::InvalidNumericField),
                };
                if current_field_index >= field_buffer.len() {
                    return Err(JsonParseFailure::TooManyFields);
                }
                field_buffer[current_field_index] = JsonField::new(string_key, JsonValue::Number(numeric_value));
                current_field_index += 1;
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

fn skip_json_numeric(index: &mut usize, data: &[u8]) -> Result<(),JsonParseFailure> {
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

fn skip_json_boolean(index: &mut usize, data: &[u8], value: bool) -> Result<(),JsonParseFailure> {
    let start = *index;
    let target = if value { "true" } else { "false" };
    while (*index - start) < target.len() {
        if *index >= data.len() {
            return Err(JsonParseFailure::Incomplete)
        }
        if data[*index] != target.as_bytes()[*index-start] {
            return Err(JsonParseFailure::InvalidBooleanField);
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
pub fn serialize_json_object<'data,Output: Write, Fields: FieldBuffer<'data>>(mut output: Output, fields: Fields) -> Result<usize, Output::Error> {
    let mut ret = 0;
    tracked_write(&mut output,&mut ret , "{")?;
    let mut field_needs_comma = false;
    for field in fields.as_ref().iter() {
        if field_needs_comma {
            tracked_write(&mut output,&mut ret , ",")?;
        } else {
            field_needs_comma = true;
        }
        write_escaped_json_string(&mut output, &mut ret , field.key)?;
        tracked_write(&mut output, &mut ret , ":")?;
        match field.value {
            JsonValue::String(s) => {
                write_escaped_json_string(&mut output, &mut ret , s)?;
            },
            JsonValue::Boolean(false) => {
                tracked_write(&mut output,&mut ret , "false")?;
            },
            JsonValue::Boolean(true) => {
                tracked_write(&mut output,&mut ret , "true")?;
            },
            JsonValue::Number(n) => {
                tracked_write(&mut output,&mut ret , &base10::i64(n))?;
            },
        }
    }
    tracked_write(&mut output, &mut ret , "}")?;
    Ok(ret)
}


fn tracked_write<T: Write>(mut output: T, counter: &mut usize, data: &str) -> Result<(), T::Error> {
    output.write_all(data.as_bytes())?;
    *counter += data.len();
    Ok(())
}

fn write_escaped_json_string<T: Write>(mut output: T, counter: &mut usize, data: &str) -> Result<(), T::Error> {
    tracked_write(&mut output, &mut *counter, "\"")?;
    for field_character in data.chars() {
        if field_character == '"' {
            tracked_write(&mut output, &mut *counter, unsafe { core::str::from_utf8_unchecked(&[b'\\', field_character as u8]) })?;
        } else {
            tracked_write(&mut output, &mut *counter, unsafe { core::str::from_utf8_unchecked(&[field_character as u8]) })?;
        }
    }
    tracked_write(&mut output, &mut *counter, "\"")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::default;

    use super::*;

    #[test]
    fn test_serialize_object_empty() {
        let mut buffer = [0_u8; 1000];
        let test_map = ArrayJsonObject::<50>::new();
        let n = test_map.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{}", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_object_simple() {
        let mut buffer = [0_u8; 1000];
        let mut test_map = ArrayJsonObject::<50>::new();
        test_map.push_field("sub", JsonValue::String("1234567890")).unwrap();
        test_map.push_field("name", JsonValue::String("John Doe")).unwrap();
        test_map.push_field("iat", JsonValue::Number(1516239022)).unwrap();
        test_map.push_field("something", JsonValue::Boolean(false)).unwrap();
        let n = test_map.serialize(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false}", buffer.split_at(n).0)
    }

    #[test]
    fn test_parse_object_success_empty() {
        let (data_end,json_object) = ArrayJsonObject::<0>::new_parsed(b"{}").unwrap();
        assert!(json_object.fields().is_empty());
    }

    #[test]
    fn test_parse_object_success_simple() {
        let data = b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false}";
        let (data_end,json_object) = ArrayJsonObject::<50>::new_parsed(data).unwrap();
        let test_fields = json_object.fields();
        assert_eq!(4, test_fields.len());
        assert_eq!(JsonField { key: "sub", value: JsonValue::String("1234567890")}, test_fields[0]);
        assert_eq!(JsonField { key: "name", value: JsonValue::String("John Doe")}, test_fields[1]);
        assert_eq!(JsonField { key: "iat", value: JsonValue::Number(1516239022)}, test_fields[2]);
        assert_eq!(JsonField { key: "something", value: JsonValue::Boolean(false)}, test_fields[3]);
    }

    #[test]
    fn test_parse_object_failure_incomplete_simple() {
        match ArrayJsonObject::<50>::new_parsed(b"{") {
            Err(JsonParseFailure::Incomplete) => {},
            _ => panic!("incomplete json")
        }
    }

    #[test]
    fn test_parse_object_failure_incomplete_brace() {
        match ArrayJsonObject::<50>::new_parsed(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false") {
            Err(JsonParseFailure::Incomplete) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_too_many_fields() {
        match ArrayJsonObject::<0>::new_parsed(b"{\"some\":\"thing\"}") {
            Err(JsonParseFailure::TooManyFields) => {},
            other => panic!("{:?}", other)
        }
    }

}
