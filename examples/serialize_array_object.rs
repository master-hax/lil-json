use std::io::stdout;
use lil_json::{ArrayJsonObject, FieldBuffer, JsonField, JsonObject, JsonValue};

fn main() {

    let number_field = JsonField::new("some_number", JsonValue::Number(12345));
    let string_field = JsonField::new("some_string", JsonValue::String("hello world!"));
    let boolean_field = JsonField::new("some_boolean", JsonValue::Boolean(true));

    let mut json_object = ArrayJsonObject::<3>::new();
    json_object.push(number_field).unwrap();
    json_object.push(string_field).unwrap();
    json_object.push(boolean_field).unwrap();

    json_object.serialize_std(stdout()).unwrap();
}

// output: {"some_number":12345,"some_string":"hello world!","some_boolean":true}