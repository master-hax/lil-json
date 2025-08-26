# lil-json

lil `#![no_std]` Rust crate to parse & serialize JavaScript Object Notation (JSON). alloc optional. std optional.

JSON can be serialized into any type that implements [`embedded_io::Write`](https://docs.rs/embedded-io/latest/embedded_io/trait.Write.html) or a `String` (with `alloc` feature enabled)

Here is a minimal example of serializing a JSON object to stdout with a one-liner, making use of the `lil-json::FieldBuffer` auto trait, `core::convert::From for JsonValue`, & `core::convert::Into for JsonField`
```rust
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
```

Here is an example of parsing a JSON object
```rust
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

```

Here is an example of parsing a JSON object with the alloc feature enabled - no need to pre-allocate space for the fields or escaped strings:
```rust
use lil_json::{JsonField, JsonObject, JsonValue, InfiniteEscapeBuffer};

fn main() {
    const SERIALIZED_DATA: &[u8] = br#"{"some_string_key":"some_string_value"}"#;
    let mut json_object = JsonObject::wrap(Vec::new());
    let mut infinite_escape_buffer = InfiniteEscapeBuffer::new();
    // parse_alloc is enabled by using wrapping a Vec and providing a mutable reference to an InfiniteEscapeBuffer
    let bytes_consumed = json_object.parse_alloc(SERIALIZED_DATA, &mut infinite_escape_buffer).unwrap();
    assert_eq!(SERIALIZED_DATA.len(), bytes_consumed);
    let parsed_fields = json_object.fields();
    assert_eq!(1, parsed_fields.len());
    assert_eq!(JsonField::new("some_string_key", JsonValue::String("some_string_value")), parsed_fields[0]);
}
```

Check out the examples for more. Still a work in progress. Expect bugs & breaking API changes. Check out the examples to get started.

the following types are currently supported:
* objects (currently limited to non-nested types)
* string (currently limited to ascii-)
* boolean
* null
* number (currently limited to integers)

the following types are not currently supported:
* arrays (currently limited to non-nested types)

TODO:
- [x] support null type
- [x] alloc features
- [x] expose serialization methods for terminal types
- [x] support arrays
- [ ] support parsing arbitrary types
- [ ] support unicode strings
- [ ] support buffered serialization
- [ ] support parsing from stream
- [ ] support parsing streaming objects/arrays
- [ ] support embedded-io-async?
