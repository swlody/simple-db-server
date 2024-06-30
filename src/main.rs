use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Context;

use std::collections::HashMap;

fn handle_get(
    stream: &mut TcpStream,
    kvp: &str,
    map: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let (key, keyname) = kvp.split_once("=").context("invalid kvp")?;
    if key != "key" {
        anyhow::bail!("invalid key in get");
    }

    if let Some(value) = map.get(keyname) {
        println!("getting: {keyname}={value}");

        let length = value.len();
        let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{value}");
        stream.write(response.as_bytes())?;
    } else {
        println!("key not found");

        let response = "HTTP/1.1 404 Not Found\r\n\r\n";
        stream.write(response.as_bytes())?;
    }

    Ok(())
}

fn handle_set(
    stream: &mut TcpStream,
    kvp: &str,
    map: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    let (key, value) = kvp.split_once("=").context("invalid kvp")?;

    println!("setting: {key}={value}");

    map.insert(key.to_string(), value.to_string());
    // TODO append write to file
    // TODO for performance, only update the file every N writes with diff since last update
    // TODO for performance, async io

    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write(response.as_bytes())?;

    Ok(())
}

fn handle_connection(
    mut stream: &mut TcpStream,
    mut map: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    // According to the HTTP standard, the request must begin with a header
    // and the header must begin with the HTTP method. Assume the GET line will be less than 1024 bytes.
    let mut buf = [0; 1024];
    let bytes_read = stream.read(&mut buf)?;
    let req = std::str::from_utf8(&buf[..bytes_read])?;
    println!("{:?}", req);

    // Grab only the first line of the request, i.e. the method header
    let (first_header, _) = req.split_once("\r\n").context("invalid header")?;

    // Assume: "GET /`path` HTTP/1.1"
    // anything not of this format will get a "Bad Request"
    // because we're not being super precise about our error codes yet!
    match first_header.split_once(" ").context("invalid request")? {
        ("GET", path_and_stuff) => match path_and_stuff
            .split_once(" HTTP/1.1")
            .context("invalid request")?
            .0
            .split_at(5)
        {
            ("/set?", kvp) => handle_set(&mut stream, kvp, &mut map)?,
            ("/get?", kvp) => handle_get(&mut stream, kvp, &map)?,
            _ => anyhow::bail!("invalid verb"),
        },
        ("HEAD", _) => {
            // standard says we must respond to head.
            let response = "HTTP/1.1 200 OK\r\n\r\n";
            stream.write(response.as_bytes())?;
        }
        _ => {
            let response = "HTTP/1.1 501 Not Implemented\r\n\r\n";
            stream.write(response.as_bytes())?;
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4000")?;

    // TODO read map from file
    // format:
    // key=value\n
    // need to disallow `=` and `\n` in key/value
    let mut map = HashMap::new();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => match handle_connection(&mut stream, &mut map) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("failed: {}", e);
                    let response = "HTTP/1.1 400 Bad Request\r\n\r\n";
                    stream.write(response.as_bytes())?;
                }
            },
            Err(e) => eprintln!("failed: {}", e),
        }
    }

    Ok(())
}
