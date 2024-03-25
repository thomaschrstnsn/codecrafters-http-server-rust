use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::thread;
use std::{
    io::{self, Write},
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
    content: Option<Content>,
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

            stream.write(&content.content)?;
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
                })
            },
            Err(_) => Self::empty_response(&status_codes::NOT_FOUND),

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

        let mut headers: HashMap<String, String> = HashMap::new();
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

fn handle_request<'a>(conf: &Configuration, request: &'a Request) -> Response<'a> {
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
                Some(("files", filename)) => Response::file_response(&[conf.files_root.as_ref().expect("files_root should be configured"), &filename.to_owned()].iter().collect()),
                _ => Response::empty_response(&status_codes::NOT_FOUND),
            },
        }
    } else {
        Response::empty_response(&status_codes::NOT_FOUND)
    }
}

fn handle_connection(conf: &Configuration, mut stream: &TcpStream) -> () {
    println!("accepted new connection");

    let request_lines = read_request(&stream);
    let request = parse_request(&request_lines).expect("request can be parsed");
    let response = handle_request(&conf, &request);
    response
        .write_to_stream(&mut stream)
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

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let configuration = Configuration::from_args(&mut std::env::args());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let configuration = configuration.clone();
                let _ = thread::spawn(move || handle_connection(&configuration, &mut stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
