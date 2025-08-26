use lil_json::{JsonField, JsonObject, JsonValue};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value"}"#;
    let mut string_escape_buffer = [0_u8; 100];
    let mut field_buffer_vec = Vec::new();
    let mut alloc_enabled_json_object = JsonObject::wrap(&mut field_buffer_vec);
    let bytes_consumed = alloc_enabled_json_object.parse_alloc_fields(
        SERIALIZED_DATA,
        string_escape_buffer.as_mut_slice()
    ).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = alloc_enabled_json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
