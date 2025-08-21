use std::io::stdout;
use lil_json::FieldBuffer;

fn main() {
    [
        ("some_number", 12345).into(),
        ("some_string", "hello world!").into(),
        ("some_boolean", true).into()
    ]
    .as_json_object()
    .serialize_std(stdout())
    .unwrap();
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}