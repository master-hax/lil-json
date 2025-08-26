use lil_json::{ArrayJsonArray, InfiniteEscapeBuffer, JsonField, JsonObject, JsonValue, EMPTY_FIELD};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value"}"#;
    
    let mut string_escape_buffer = InfiniteEscapeBuffer::new();
    let mut json_object = JsonObject::wrap([EMPTY_FIELD; 1]);
    // parse_alloc_escape uses an unlimited string escape buffer
    let bytes_consumed = json_object.parse_alloc_escape(
        SERIALIZED_DATA,
        &mut string_escape_buffer,
    ).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
