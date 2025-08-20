# lil-json

`#![no_std]` Rust crate to parse & serialize JSON

example object serialization:
```
use lil_json::{JsonObject, JsonValue};

fn main() {
    let mut buffer = [0_u8; 256];
    let mut json_object = JsonObject::<10>::new();
    json_object.push_field("some_number", JsonValue::Number(12345)).unwrap();
    json_object.push_field("some_string", JsonValue::String("hello world!")).unwrap();
    json_object.push_field("some_boolean", JsonValue::Boolean(true)).unwrap();
    let n = json_object.serialize_blocking(buffer.as_mut_slice()).unwrap();
    assert_eq!(
        b"{\"some_number\":12345,\"some_string\":\"hello world!\",\"some_boolean\":true}",
        buffer.split_at(n).0,
    )
}
```

the following types are currently supported:
* objects (currently limited to non-nested objects)
* string (currently limited to ascii)
* boolean
* number

the following types are not currently supported:
* arrays
* null
* nested types

TODO:
- [ ] support null type
- [ ] expose serialization methods for terminal types
- [ ] support arrays
- [ ] support parsing arbitrary types
- [ ] support unicode strings
