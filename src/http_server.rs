
use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;


struct HTTPServerOutput {
	do_shutdown: bool,
}

struct Request<'a> {
	http_version: &'a str,
	method: &'a str,
	path: &'a str,
}

fn http_server_loop(output: &mut HTTPServerOutput) -> io::Result<()> {
	//NOTE: this code is extremely lazy with error handling,
	// println!("Starting server...");
	let mut _trans_mem = String::new();
	let trans_mem = &mut _trans_mem;

	let listener = TcpListener::bind("127.0.0.1:8001")?;
	// println!("Server started!");
	for stream in listener.incoming() {
		match stream {
			Ok(_stream) => {
				trans_mem.clear();
				let mut stream = _stream;

				match stream.read_to_string(trans_mem) {
					Err(e) => eprintln!("Error handling client: {}", e),
					_ => (),
				}

				match parse_request(&trans_mem) {
					Ok(request) => {
						if request.method == "POST" && request.path == "/shutdown" {//POST /shutdown
							output.do_shutdown = true;
						}
					},
					Err(()) => {
						eprintln!("Bad request: {}", &trans_mem);
					},
				}
			},
			Err(e) => eprintln!("Connection failed: {}", e),
		}
	}
	Ok(())
}

pub fn handle_client(_stream: TcpStream, trans_mem: &mut String) -> io::Result<()> {
	let mut stream = _stream;

	stream.read_to_string(trans_mem)?;

	match parse_request(&trans_mem) {
		Ok(request) => {
			if request.method == "POST" && request.path == "shutdown" {

			}
		},
		Err(()) => {
			eprintln!("Bad request: {}", &trans_mem);
		},
	}
	Ok(())
}

fn parse_request(request: &String) -> Result<Request, ()> {
	let mut parts = request.split(" ");
	let method = match parts.next() {
		Some(method) => method.trim(),
		None => return Err(()),
	};
	let path = match parts.next() {
		Some(path) => path.trim(),
		None => return Err(()),
	};
	let http_version = match parts.next() {
		Some(version) => version.trim(),
		None => return Err(()),
	};
	// let time = Local::now();
	Ok(Request {
		http_version: http_version,
		method: method,
		path: path,
	})
}
