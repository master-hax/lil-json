use std::io::stdout;
use embedded_io_adapters::std::FromStd;
use lil_json::{FieldBuffer, JsonField, JsonValue};

fn main() {
    [
        JsonField::new("some_number", JsonValue::Number(12345)),
        JsonField::new("some_string", JsonValue::String("hello world!")),
        JsonField::new("some_boolean", JsonValue::Boolean(true)),
    ]
    .into_json_object()
    .serialize(FromStd::new(stdout()))
    .unwrap();
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}