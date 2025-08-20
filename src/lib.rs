#![no_std]

use embedded_io::Write;
use numtoa::base10;

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum JsonValue<'a> {
    String(&'a str),
    Boolean(bool),
    Number(i64),
}

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub struct JsonField<'a,'b> {
    pub key: &'a str,
    pub value: JsonValue<'b>,
}

impl <'a,'b> JsonField<'a,'b> {
    pub fn new(key: &'a str, value: JsonValue<'b>) -> Self {
        JsonField { key, value }
    }

    pub fn new_string(key: &'a str, value: &'b str) -> Self {
        Self::new(key, JsonValue::String(value))
    }

    pub fn new_number(key: &'a str, value: i64) -> Self {
        Self::new(key, JsonValue::Number(value))
    }

    pub fn new_boolean(key: &'a str, value: bool) -> Self {
        Self::new(key, JsonValue::Boolean(value))
    }
}

#[derive(Debug)]
pub struct JsonObject<'a,const MAX_FIELDS: usize> {
    fields: [JsonField<'a,'a>; MAX_FIELDS],
    num_fields: usize,
}

#[derive(Debug)]
pub enum JsonParseFailure {
    Incomplete,
    TooManyFields,
    InvalidStructure,
    InvalidStringField,
    InvalidNumericField,
    InvalidBooleanField,
}

pub const EMPTY_FIELD: JsonField<'static,'static> = JsonField{ key: "", value: JsonValue::String("")};

impl <'a,'b> Default for JsonField<'a,'b> {
    fn default() -> Self {
        EMPTY_FIELD
    }
}

impl<'a,const MAX_FIELDS: usize> Default for JsonObject<'a,MAX_FIELDS> {
    fn default() -> Self {
        Self::new()
    }
}

impl <'a,const MAX_FIELDS: usize> JsonObject<'a,MAX_FIELDS> {

    pub const fn new() -> Self {
        JsonObject { fields: [EMPTY_FIELD; MAX_FIELDS], num_fields: 0 }
    }

    pub const fn as_slice(&self) -> &[JsonField<'a,'a>] {
        self.fields.split_at(self.num_fields).0
    }

    pub const fn as_mut_slice(&mut self) -> &mut [JsonField<'a,'a>] {
        self.fields.split_at_mut(self.num_fields).0
    }

    pub const fn push<'x: 'a,'y: 'a>(&mut self, field: JsonField<'x,'y>) -> Result<(),JsonField<'x,'y>> {
        if self.num_fields == MAX_FIELDS {
            return Err(field);
        }
        self.fields[self.num_fields] = field;
        self.num_fields += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<JsonField<'a,'a>> {
        if self.num_fields == 0 {
            return None;
        }
        let field =  core::mem::take(&mut self.fields[self.num_fields]);
        self.num_fields -= 1;
        Some(field)
    }

    pub const fn push_field(&mut self, key: &'a str, value: JsonValue<'a>) -> Result<(),()> {
        if self.num_fields == MAX_FIELDS {
            return Err(());
        }
        self.fields[self.num_fields] = JsonField { key, value: value };
        self.num_fields += 1;
        Ok(())
    }

    pub fn parse_from(data: &'a [u8]) -> Result<(usize,Self),JsonParseFailure> {
        let mut fields = [EMPTY_FIELD; MAX_FIELDS];
        let (n, num_fields) = parse_json_object(data, fields.as_mut_slice())?;
        let ret = Self {
            fields,
            num_fields
        };
        Ok((n,ret))
    }

    pub fn serialize_blocking<T: Write>(&self, output: T) -> Result<usize,T::Error> {
        write_json_map(output, self.as_slice())
    }

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

/// parse a json object into the provided field buffer & return (data bytes consumed,num fields parsed) on success
pub fn parse_json_object<'a>(data: &'a [u8], field_buffer: &mut [JsonField<'a,'a>]) -> Result<(usize,usize),JsonParseFailure> {
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
            return Ok((current_data_index+1,current_field_index))
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

pub(crate) fn write_json_map<T: Write>(mut output: T, fields: &[JsonField]) -> Result<usize, T::Error> {
    let mut ret = 0;
    tracked_write(&mut output,&mut ret , "{")?;
    let mut field_needs_comma = false;
    for field in fields.iter() {
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_object_empty() {
        let mut buffer = [0_u8; 1000];
        let test_map = JsonObject::<0>::default();
        let n = test_map.serialize_blocking(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{}", buffer.split_at(n).0)
    }

    #[test]
    fn test_serialize_object_simple() {
        let mut buffer = [0_u8; 1000];
        let mut test_map = JsonObject::<50>::default();
        test_map.push_field("sub", JsonValue::String("1234567890")).unwrap();
        test_map.push_field("name", JsonValue::String("John Doe")).unwrap();
        test_map.push_field("iat", JsonValue::Number(1516239022)).unwrap();
        test_map.push_field("something", JsonValue::Boolean(false)).unwrap();
        let n = test_map.serialize_blocking(buffer.as_mut_slice()).unwrap();
        assert_eq!(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false}", buffer.split_at(n).0)
    }

    #[test]
    fn test_parse_object_success_empty() {
        let data = b"{}";
        let (n,test_map) = JsonObject::<0>::parse_from(data).unwrap();
        assert_eq!(data.len(),n);
        assert!(test_map.as_slice().is_empty());
    }

    #[test]
    fn test_parse_object_success_simple() {
        let data = b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false}";
        let (n,test_map) = JsonObject::<50>::parse_from(data).unwrap();
        assert_eq!(data.len(),n);
        let test_fields = test_map.as_slice();
        assert_eq!(4, test_fields.len());
        assert_eq!(JsonField { key: "sub", value: JsonValue::String("1234567890")}, test_fields[0]);
        assert_eq!(JsonField { key: "name", value: JsonValue::String("John Doe")}, test_fields[1]);
        assert_eq!(JsonField { key: "iat", value: JsonValue::Number(1516239022)}, test_fields[2]);
        assert_eq!(JsonField { key: "something", value: JsonValue::Boolean(false)}, test_fields[3]);
    }

    #[test]
    fn test_parse_object_failure_incomplete_simple() {
        match JsonObject::<50>::parse_from(b"{") {
            Err(JsonParseFailure::Incomplete) => {},
            other => panic!("{:?}", other)
        }
    }

    #[test]
    fn test_parse_object_failure_incomplete_brace() {
        match JsonObject::<50>::parse_from(b"{\"sub\":\"1234567890\",\"name\":\"John Doe\",\"iat\":1516239022,\"something\":false") {
            Err(JsonParseFailure::Incomplete) => {},
            other => panic!("{:?}", other)
        }
    }

        #[test]
    fn test_parse_object_failure_too_many_fields() {
        match JsonObject::<0>::parse_from(b"{\"some\":\"thing\"}") {
            Err(JsonParseFailure::TooManyFields) => {},
            other => panic!("{:?}", other)
        }
    }

}
