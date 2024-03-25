use std::collections::HashMap;
use std::io::BufRead;
use std::{
    io::{self, Write},
    net::{TcpListener, TcpStream},
};

struct Content<'a> {
    mime_type: &'a str,
    content: &'a [u8],
}

struct StatusCode {
    code: u16,
    status: &'static str,
}

mod status_codes {
    use super::StatusCode;

    pub const OK: StatusCode = StatusCode {
        code: 200,
        status: "OK",
    };

    pub const NOT_FOUND: StatusCode = StatusCode {
        code: 404,
        status: "Not Found",
    };
}

struct Response<'a> {
    status_code: &'a StatusCode,
    content: Option<Content<'a>>,
}

fn write_newline(mut stream: &TcpStream) -> std::io::Result<()> {
    stream.write(b"\r\n")?;
    Ok(())
}

fn write_header(mut stream: &TcpStream, key: &str, value: &str) -> std::io::Result<()> {
    write!(&mut stream, "{}: {}", key, value)?;
    write_newline(&mut stream)
}

impl<'a> Response<'a> {
    fn write_to_stream(&self, mut stream: &TcpStream) -> std::io::Result<()> {
        write!(
            &mut stream,
            "HTTP/1.1 {} {}",
            self.status_code.code, self.status_code.status
        )?;
        write_newline(&mut stream)?;

        if let Some(content) = &self.content {
            write_header(&mut stream, "Content-Type", content.mime_type)?;
            write_header(
                &mut stream,
                "Content-Length",
                &format!("{}", content.content.len()),
            )?;
            write_newline(&mut stream)?;

            stream.write(content.content)?;
        } else {
            write_newline(&mut stream)?;
            write_newline(&mut stream)?;
        }


        Ok(())
    }

    fn empty_response(status_code: &'a StatusCode) -> Self {
        Self {
            status_code,
            content: None,
        }
    }

    fn text_reponse(status_code: &'a StatusCode, text: &'a str) -> Self {
        Self {
            status_code,
            content: Some(Content {
                mime_type: "text/plain",
                content: text.as_bytes(),
            }),
        }
    }
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
    headers: HashMap<String, String>,
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

        let mut headers : HashMap<String, String> = HashMap::new();
        for header_line in request_lines.iter().skip(1) {
            if let Some((key, value)) = header_line.split_once(": ") {
                headers.insert(key.to_owned(), value.to_owned());
            }
        }

        Ok(Request {
            verb,
            path: path_str,
            version: vers_str,
            headers,
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

fn handle_request(request: &Request) -> Response {
    dbg!("handling: {:?}", request);
    if let Some(path) = request.path.strip_prefix('/') {
        if path.is_empty() {
            return Response::empty_response(&status_codes::OK);
        }
        if path == "user-agent" {
            return Response::text_reponse(&status_codes::OK, request.headers.get("User-Agent").expect("must have User-Agent header"));
        }
        match path.split_once('/') {
            Some(("echo", content)) => {
                Response::text_reponse(&status_codes::OK, content)
            }
            _ => Response::empty_response(&status_codes::NOT_FOUND),
        }
    } else {
        Response::empty_response(&status_codes::NOT_FOUND)
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
                let response = handle_request(&request);
                response.write_to_stream(&mut stream).expect("response can be sent back");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
