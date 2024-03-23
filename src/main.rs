use std::{net::{TcpListener, TcpStream}, io::{Write, Read, self}};
use std::io::BufRead;

fn resp(mut stream: &TcpStream, code: u8, status: &str) -> std::io::Result<()> {
    // stream.write(b"HTTP/1.1 200 OK\r\n\r\n")
    write!(&mut stream, "HTTP/1.1 {} {}\r\n\r\n", code, status)
}

#[derive(Debug)]
enum Verb {
    GET
}

#[derive(Debug)]
struct Request<'a> {
    verb: Verb,
    path: &'a str,
    version: &'a str
}

#[derive(Debug)]
enum RequestParseError {
    NoInput,
    InvalidVerb,
    InvalidStructure
}

fn parse_start_line(req: &str) -> Result<Request, RequestParseError> {
    let lines : Vec<_> = req.lines().collect();

    if lines.len() == 0 {
        Err(RequestParseError::NoInput)
    } else {
        let start_line = lines[0];
        let mut split_iter = start_line.split(' ');

        let verb_str = split_iter.next().ok_or(RequestParseError::InvalidStructure)?;
        let path_str = split_iter.next().ok_or(RequestParseError::InvalidStructure)?;
        let vers_str = split_iter.next().ok_or(RequestParseError::InvalidStructure)?;

        let verb = match verb_str {
            "GET" => Ok(Verb::GET),
            _ => Err(RequestParseError::InvalidVerb),
        }?;

        Ok(Request{verb, path: path_str, version: vers_str})
    }
}

fn handle_request(request: &Request, stream: &mut TcpStream) -> std::io::Result<()> {
    todo!();
}

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let reader = io::BufReader::new(stream);
                let line_iter = reader.lines();

                match parse_start_line(&buffer) {
                    Ok(req) => match handle_request(&req, &mut stream) {
                        Ok(_) => println!("request handled"),
                        Err(_) => println!("problem handling request!"),
                    },
                    Err(err) => println!("problem understanding request: {:#?}", err),
                }

                let _ = resp(&mut stream, 200, "OK");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
