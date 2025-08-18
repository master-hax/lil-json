use lil_json::{JsonField, JsonObject};

fn main() {
    let mut original_object = JsonObject::<10>::new();
    let number_field = JsonField::new_number("some_number", 12345);
    let string_field = JsonField::new_string("some_string", "hello world!");
    let boolean_field = JsonField::new_boolean("some_boolean", true);
    original_object.push(number_field).unwrap();
    original_object.push(string_field).unwrap();
    original_object.push(boolean_field).unwrap();

    let mut buffer = [0_u8; 128];
    let serialized_end = original_object.serialize_blocking(buffer.as_mut_slice()).unwrap();
    let serialized = buffer.split_at(serialized_end).0;
    assert_eq!(b"{\"some_number\":12345,\"some_string\":\"hello world!\",\"some_boolean\":true}", serialized);

    let (deserialized_end,deserialized_object) = JsonObject::<3>::parse_from(serialized).unwrap();
    assert_eq!(serialized_end,deserialized_end);
    let deserialized_fields = deserialized_object.as_slice();
    assert_eq!(3,deserialized_fields.len());
    assert_eq!(number_field, deserialized_fields[0]);
    assert_eq!(string_field, deserialized_fields[0]);
    assert_eq!(boolean_field, deserialized_fields[0]);
}