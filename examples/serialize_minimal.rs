use lil_json::FieldBuffer;

fn main() {
    println!(
        "{}",
        [
            ("some_number", 12345).into(),
            ("some_string", "hello world!").into(),
            ("some_boolean", true).into()
        ].as_json_object()
    );
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}