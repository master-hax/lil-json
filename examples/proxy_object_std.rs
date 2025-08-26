/// proxies a json object from stdin to stdout

use std::{io::{stderr, stdin, stdout}, process::exit};

use embedded_io::{Read, Write};
use embedded_io_adapters::std::FromStd;
use lil_json::{ArrayJsonObject, JsonField, JsonParseFailure, JsonValue};

const MAX_FIELDS: usize = 50;
const READ_BUFFER_SIZE: usize = 16384; // 16 KiB
const ESCAPE_BUFFER_SIZE: usize = 16384; // 16 KiB

fn main() {
    let stdin = FromStd::new(stdin());
    let stderr = FromStd::new(stderr());
    let stdout = FromStd::new(stdout());
    proxy_json_object(stdin, stdout, stderr);
}

fn proxy_json_object<Input: Read, Output: Write, Logs: Write>(mut input: Input, mut output: Output, mut log_output: Logs) {
    let mut read_buffer = [0_u8; READ_BUFFER_SIZE];
    let mut escape_buffer = [0_u8; ESCAPE_BUFFER_SIZE];
    let mut read_buffer_end = 0;
    loop {
        match input.read(read_buffer.as_mut_slice()) {
            // Ok(0) => break,
            Err(e) => {
                // e.error
                eprintln!("failed to read from stdin: {:?}", e);
                exit(1);
            },
            Ok(n) => {
                read_buffer_end += n;
                match ArrayJsonObject::<MAX_FIELDS>::new_parsed(read_buffer.split_at(read_buffer_end).0, &mut escape_buffer) {
                    Err(JsonParseFailure::Incomplete) => continue,
                    Err(e) => {
                        log_output.write_fmt(format_args!("read {} bytes, failed to parse json object: {:?}\n", read_buffer_end, e)).unwrap();
                        log_output.flush().unwrap();
                        exit(1);
                    },
                    Ok((bytes_consumed, json_object)) => {
                        log_output.write_fmt(format_args!("read {} bytes, parsed a json object in {} bytes with {} fields\n", read_buffer_end, bytes_consumed, json_object.len())).unwrap();
                        log_output.flush().unwrap();
                        json_object.serialize(&mut output).unwrap();
                        output.flush().unwrap();
                        exit(0)
                    },
                }
            },

        }
    }
}