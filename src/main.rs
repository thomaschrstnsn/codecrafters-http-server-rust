use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::thread;
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

struct Content {
    mime_type: &'static str,
    content: Vec<u8>,
}

struct StatusCode {
    code: u16,
    status: &'static str,
}

mod status_codes {
    use super::StatusCode;

    const fn status_code(code: u16, status: &'static str) -> StatusCode {
        StatusCode { code, status }
    }

    pub const OK: StatusCode = status_code(200, "OK");
    pub const CREATED: StatusCode = status_code(201, "Created");

    pub const NOT_FOUND: StatusCode = status_code(404, "Not Found");

    pub const INTERNAL_SERVER_ERROR: StatusCode = status_code(500, "Internal Server Error");
}

struct Response<'a> {
    status_code: &'a StatusCode,
    content: Option<Content>,
}

fn write_newline(mut stream: &TcpStream) -> std::io::Result<()> {
    stream.write_all(b"\r\n")
}

fn write_header(mut stream: &TcpStream, key: &str, value: &str) -> std::io::Result<()> {
    write!(&mut stream, "{}: {}", key, value)?;
    write_newline(stream)
}

impl<'a> Response<'a> {
    fn write_to_stream(&self, mut stream: &TcpStream) -> std::io::Result<()> {
        write!(
            &mut stream,
            "HTTP/1.1 {} {}",
            self.status_code.code, self.status_code.status
        )?;
        write_newline(stream)?;

        if let Some(content) = &self.content {
            write_header(stream, "Content-Type", content.mime_type)?;
            write_header(
                stream,
                "Content-Length",
                &format!("{}", content.content.len()),
            )?;
            write_newline(stream)?;

            stream.write_all(&content.content)?;
        } else {
            write_newline(stream)?;
            write_newline(stream)?;
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
                content: text.as_bytes().to_vec(),
            }),
        }
    }

    fn file_response(path: &PathBuf) -> Self {
        match std::fs::read(path) {
            Ok(file_content) => Self {
                status_code: &status_codes::OK,
                content: Some(Content {
                    mime_type: "application/octet-stream",
                    content: file_content,
                }),
            },
            Err(_) => Self::empty_response(&status_codes::NOT_FOUND),
        }
    }
}

#[derive(Debug)]
enum Verb {
    Get,
    Post,
}

#[derive(Debug)]
struct Request {
    verb: Verb,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

#[derive(Debug)]
enum RequestParseError {
    InvalidVerb,
    CouldNotReadStartLine,
    InvalidStructure,
    CouldNotReadHeader,
    InvalidHeader,
    InvalidContentLength,
    CouldNotReadBody,
}

fn read_headers(reader: &mut dyn BufRead) -> Result<HashMap<String, String>, RequestParseError> {
    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        let mut header_line = String::new();
        reader
            .read_line(&mut header_line)
            .map_err(|_| RequestParseError::CouldNotReadHeader)?;
        let header_line = header_line.trim_end();
        if header_line.is_empty() {
            break;
        }

        if let Some((key, value)) = header_line.split_once(": ") {
            headers.insert(key.to_owned(), value.to_owned());
        } else {
            return Err(RequestParseError::InvalidHeader);
        }
    }
    Ok(headers)
}

fn parse_request(mut stream: &TcpStream) -> Result<Request, RequestParseError> {
    let mut reader = BufReader::new(&mut stream);

    let mut start_line = String::new();
    reader
        .read_line(&mut start_line)
        .map_err(|_| RequestParseError::CouldNotReadStartLine)?;
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
        "GET" => Ok(Verb::Get),
        "POST" => Ok(Verb::Post),
        _ => Err(RequestParseError::InvalidVerb),
    }?;

    let headers = read_headers(&mut reader)?;

    let content = if let Some(length_str) = headers.get("Content-Length") {
        let content_length = length_str
            .parse::<usize>()
            .map_err(|_| RequestParseError::InvalidContentLength)?;
        let mut buffer: Vec<u8> = vec![0; content_length];

        reader
            .read_exact(&mut buffer)
            .map_err(|_| RequestParseError::CouldNotReadBody)?;

        Some(buffer)
    } else {
        None
    };

    Ok(Request {
        verb,
        path: path_str,
        version: vers_str,
        headers,
        body: content,
    })
}

fn handle_request(request: &Request) -> Response {
    dbg!("handling: {:?}", request);
    if let Some(path) = request.path.strip_prefix('/') {
        match path {
            "" => Response::empty_response(&status_codes::OK),
            "user-agent" => Response::text_reponse(
                &status_codes::OK,
                request
                    .headers
                    .get("User-Agent")
                    .expect("must have User-Agent header"),
            ),
            _ => match path.split_once('/') {
                Some(("echo", content)) => Response::text_reponse(&status_codes::OK, content),
                Some(("files", filename)) => {
                    let path = [
                        CONFIGURATION
                            .files_root
                            .as_ref()
                            .expect("files_root should be configured"),
                        &filename.to_owned(),
                    ]
                    .iter()
                    .collect();

                    match request.verb {
                        Verb::Get => Response::file_response(&path),
                        Verb::Post => {
                            let body = request
                                .body
                                .as_ref()
                                .expect("body should be present on request");

                            match std::fs::write(path, body) {
                                Ok(_) => Response::empty_response(&status_codes::CREATED),
                                Err(_) => {
                                    Response::empty_response(&status_codes::INTERNAL_SERVER_ERROR)
                                }
                            }
                        }
                    }
                }
                _ => Response::empty_response(&status_codes::NOT_FOUND),
            },
        }
    } else {
        Response::empty_response(&status_codes::NOT_FOUND)
    }
}

fn handle_connection(stream: &TcpStream) {
    println!("accepted new connection");

    let request = parse_request(stream).expect("request should be parsable");
    let response = handle_request(&request);
    response
        .write_to_stream(stream)
        .expect("response can be sent back");
}

#[derive(Clone)]
struct Configuration {
    files_root: Option<String>,
}

impl Configuration {
    fn from_args(args: &mut std::env::Args) -> Configuration {
        args.next(); // skip first (program)
        let directory = if let Some(dir_arg) = args.next() {
            if dir_arg == "--directory" {
                args.next()
            } else {
                None
            }
        } else {
            None
        };

        Configuration {
            files_root: directory,
        }
    }
}

use lazy_static::lazy_static;

lazy_static! {
    static ref CONFIGURATION: Configuration = Configuration::from_args(&mut std::env::args());
}

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let _ = thread::spawn(move || handle_connection(&stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
