use std::io::BufRead;
use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
};

fn resp(mut stream: &TcpStream, code: u16, status: &str) -> std::io::Result<()> {
    write!(&mut stream, "HTTP/1.1 {} {}\r\n\r\n", code, status)
}

#[derive(Debug)]
enum Verb {
    GET,
}

#[derive(Debug)]
struct Request {
    verb: Verb,
    path: String,
    version: String,
}

#[derive(Debug)]
enum RequestParseError {
    NoInput,
    InvalidVerb,
    InvalidStructure,
}

fn parse_request(request_lines: &Vec<String>) -> Result<Request, RequestParseError> {
    if request_lines.len() == 0 {
        Err(RequestParseError::NoInput)
    } else {
        let start_line = &request_lines[0];
        let mut split_iter = start_line.split(' ');

        let verb_str = split_iter
            .next()
            .ok_or(RequestParseError::InvalidStructure)?;
        let path_str = split_iter
            .next()
            .ok_or(RequestParseError::InvalidStructure)?
            .to_owned();
        let vers_str = split_iter
            .next()
            .ok_or(RequestParseError::InvalidStructure)?
            .to_owned();

        let verb = match verb_str {
            "GET" => Ok(Verb::GET),
            _ => Err(RequestParseError::InvalidVerb),
        }?;

        Ok(Request {
            verb,
            path: path_str,
            version: vers_str,
        })
    }
}

fn read_request(stream: &TcpStream) -> Vec<String> {
    let reader = io::BufReader::new(stream);
    let request_lines: Vec<_> = reader
        .lines()
        .map(|result| result.expect("valid utf8"))
        .take_while(|line| !line.is_empty())
        .collect();

    request_lines
}

fn handle_request(request: &Request, mut stream: &TcpStream) -> std::io::Result<()> {
    if request.path == "/" {
        resp(&mut stream, 200, "OK")
    } else {
        resp(&mut stream, 404, "Not Found")
    }
}

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let request_lines = read_request(&stream);
                let request = parse_request(&request_lines).expect("request can be parsed");
                handle_request(&request, &mut stream).expect("request can be handled");

                // match parse_request(&buffer) {
                //     Ok(req) => match handle_request(&req, &mut stream) {
                //         Ok(_) => println!("request handled"),
                //         Err(_) => println!("problem handling request!"),
                //     },
                //     Err(err) => println!("problem understanding request: {:#?}", err),
                // }
                //
                // let _ = resp(&mut stream, 200, "OK");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
