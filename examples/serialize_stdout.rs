use std::io::stdout;
use embedded_io::Write;
use embedded_io_adapters::std::FromStd;
use lil_json::{ArrayJsonObject, JsonValue};


fn main() {
    let mut stdout = FromStd::new(stdout());
    let mut json_object = ArrayJsonObject::<10>::new();
    json_object.push_field("some_number", JsonValue::Number(12345)).unwrap();
    json_object.push_field("some_string", JsonValue::String("hello world!")).unwrap();
    json_object.push_field("some_boolean", JsonValue::Boolean(true)).unwrap();
    json_object.serialize(&mut stdout).unwrap();
    stdout.flush().unwrap();
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}