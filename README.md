# lil-json

lil `#![no_std]` Rust crate to parse & serialize JavaScript Object Notation (JSON). Alloc optional.

JSON can be serialized into any type that implements [`embedded_io::Write`](https://docs.rs/embedded-io/latest/embedded_io/trait.Write.html). Serialize a JSON object to stdout with a one-liner!
```rust
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
- [ ] alloc features
- [ ] expose serialization methods for terminal types
- [ ] support arrays
- [ ] support parsing arbitrary types
- [ ] support unicode strings
- [ ] support buffered serialization
- [ ] support parsing from stream
- [ ] support parsing streaming objects/arrays
- [ ] support embedded-io-async?
