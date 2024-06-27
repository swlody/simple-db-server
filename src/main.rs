use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::Context;

use std::collections::HashMap;

fn handle_get(
    stream: &mut TcpStream,
    kvp: &str,
    map: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let (key, value) = kvp.split_once("=").context("invalid kvp")?;
    if key != "key" {
        anyhow::bail!("Invalid key");
    }

    let value = map.get(value).context("invalid key")?;
    let length = value.len();

    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{value}");
    stream.write(response.as_bytes())?;

    Ok(())
}

fn handle_set(
    stream: &mut TcpStream,
    kvp: &str,
    map: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    let (key, value) = kvp.split_once("=").context("invalid kvp")?;
    map.insert(key.to_string(), value.to_string());
    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write(response.as_bytes())?;
    Ok(())
}

fn handle_connection(
    mut stream: &mut TcpStream,
    mut map: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    // Assume header is within the first 1024 bytes.
    let mut buf = [0; 1024];
    let _bytes_read = stream.read(&mut buf)?;

    if let Some((header, _body)) = String::from_utf8_lossy(&buf[..]).split_once("\r\n\r\n") {
        println!("{header}");
        let (_, get_req) = header.split_once("GET /").context("invalid_header")?;
        let (get_req, _) = get_req.split_once(" HTTP/1.1").context("invalid_header")?;
        let (method, kvp) = get_req.split_once("?").context("invalid header")?;
        println!("method: {method}, kvp: {kvp}");

        match method {
            "get" => handle_get(&mut stream, kvp, &mut map)?,
            "set" => handle_set(&mut stream, kvp, &mut map)?,
            _ => {
                anyhow::bail!("Invalid method");
            }
        }

        Ok(())
    } else {
        anyhow::bail!("Invalid header");
    }
}

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4000")?;
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
