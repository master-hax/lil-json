use lil_json::{JsonField, JsonObject, JsonValue, AllocEscapeBuffer};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value"}"#;
    let mut json_object = JsonObject::wrap(Vec::new());
    let mut infinite_escape_buffer = AllocEscapeBuffer::new();
    // parse_alloc is enabled by using wrapping a Vec and providing a mutable reference to an InfiniteEscapeBuffer
    let bytes_consumed = json_object.parse_alloc(SERIALIZED_DATA, &mut infinite_escape_buffer).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
