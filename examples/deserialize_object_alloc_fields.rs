use lil_json::{JsonField, JsonObject, JsonValue};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value"}"#;
    let mut string_escape_buffer = [0_u8; 100];
    let mut json_object = JsonObject::wrap(Vec::new());
    // parse_alloc_fields is enabled by using wrapping a Vec. It can support unlimited object fields.
    let bytes_consumed = json_object.parse_alloc_fields(
        SERIALIZED_DATA,
        string_escape_buffer.as_mut_slice()
    ).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
