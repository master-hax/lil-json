#![no_std]

use lil_json::{ArrayJsonObject, FieldBuffer, JsonField};

fn main() {
    let number_field = JsonField::new_number("some_number", 12345);
    let string_field = JsonField::new_string("some_string", "hello world!");
    let boolean_field = JsonField::new_boolean("some_boolean", true);

    // create a JSON object
    let json_object = [
        number_field,
        string_field,
        boolean_field,
    ].into_json_object();

    // create a buffer & serialize the object into it
    let mut serialize_buffer = [0_u8; 128];
    let serialized_end = json_object.serialize(serialize_buffer.as_mut_slice()).unwrap();
    let serialized = serialize_buffer.split_at(serialized_end).0;
    assert_eq!(br#"{"some_number":12345,"some_string":"hello world!","some_boolean":true}"#, serialized);

    // JSON strings need to be escaped; this space is used to store the escaped strings
    let mut escape_buffer = [0_u8; 128];

    // parse a JSON object from the serialized data
    let (data_end,deserialized_object) = ArrayJsonObject::<3>::new_parsed(serialized,&mut escape_buffer).unwrap();

    // verify that the parsed JSON object is identical to the original one
    assert_eq!(data_end, serialized_end); // they both took up the same amount of data
    assert_eq!(json_object.fields(),deserialized_object.fields()); // they both have the same fields in the same order
}