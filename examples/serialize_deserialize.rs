#![no_std]

use lil_json::{ArrayJsonObject, JsonField, JsonObject};

fn main() {
    let number_field = JsonField::new_number("some_number", 12345);
    let string_field = JsonField::new_string("some_string", "hello world!");
    let boolean_field = JsonField::new_boolean("some_boolean", true);

    // create a JSON object
    let mut original_object = ArrayJsonObject::<10>::new();
    original_object.push(number_field).unwrap();
    original_object.push(string_field).unwrap();
    original_object.push(boolean_field).unwrap();

    // create a buffer & serialize the object into it
    let mut buffer = [0_u8; 128];
    let serialized_end = original_object.serialize(buffer.as_mut_slice()).unwrap();
    let serialized = buffer.split_at(serialized_end).0;
    assert_eq!(b"{\"some_number\":12345,\"some_string\":\"hello world!\",\"some_boolean\":true}", serialized);

    // deserialize an identical JSON object from the serialized data
    let (data_end,deserialized_object) = ArrayJsonObject::<3>::new_parsed(serialized).unwrap();
    assert_eq!(serialized_end,data_end);
    let deserialized_fields = deserialized_object.fields();
    assert_eq!(3,deserialized_fields.len());
    assert_eq!(number_field, deserialized_fields[0]);
    assert_eq!(string_field, deserialized_fields[1]);
    assert_eq!(boolean_field, deserialized_fields[2]);
}