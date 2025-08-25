use lil_json::{ArrayJsonObject, JsonField, JsonValue};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value}"#;
    let mut escape_buffer = [0_u8; 100];
    let (bytes_consumed,json_object) = ArrayJsonObject::<1>::new_parsed(
        SERIALIZED_DATA,
        escape_buffer.as_mut_slice()
    ).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
