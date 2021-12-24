
use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::time::{Duration};

use crate::util::*;
use crate::config::*;

struct Request<'a> {
	http_version: &'a str,
	method: &'a str,
	path: &'a str,
}
struct Response {
	// content: Option<&'a [u8]>,
	// header: &'a str,
	status: &'static str,
}

pub fn poll(http_server: &mut HTTPServer, trans_mem: &mut Vec<u8>) -> HTTPServerOutput {
	//NOTE: the following code does not correctly timeout a slow connection, enabling slow loris attacks if this server were exposed to the internet; the fix would be something like manually implementing tcp packet buffering with a timer, or dig into the specifics of rust tcp to see if such a packet buffering system is already available
	//multithreading this code would aleviate this vulnerability but not get rid of it (an attacker could spin up infinite threads)
	let mut output = HTTPServerOutput{
		shutdown: false,
	};
	while !http_server.disabled {
		if let Some(listener) = &http_server.listener {
			let (mut tcpstream, addr) = match listener.accept() {
				Ok(s) => s,
				Err(e) => match e.kind() {
					io::ErrorKind::WouldBlock => break,
					_ => {
						print!("network error: failed to accept on network listener, got error '{}'\n", e);

						http_server.listener = None;
						break;
					},
				},
			};

			// match tcpstream.set_read_timeout(Some(Duration::from_millis((http::CONNECTION_TIMEOUT*1000.0) as u64))) {
			// 	_ => (),
			// }
			// match tcpstream.set_write_timeout(Some(Duration::from_millis((http::CONNECTION_TIMEOUT*1000.0) as u64))) {
			// 	_ => (),
			// }

			let raw_request = match push_stream(trans_mem, &mut tcpstream) {
				Ok(v) => v,
				Err(e) => match e.kind() {
					io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
						print!("network warning: connection timeout from '{}', got error '{}'\n", addr, e);
						break;
					}
					_ => {
						print!("network warning: could not read tcp connection from '{}', got error '{}'\n", addr, e);
						break;
					},
				},
			};

			match parse_request(raw_request) {
				Ok(request) => {
					if request.method == "POST" {//POST /shutdown
						if request.path == "/shutdown" {
							output.shutdown = true;

							write_response(&mut tcpstream, trans_mem, Response{
								// content: None,
								status: http::status::OK,
							});
							break;
						} else {
							print!("network warning: received an unrecognized http request from '{}', contents:'{}'\n", addr, String::from_utf8_lossy(raw_request));

							write_response(&mut tcpstream, trans_mem, Response{
								// content: None,
								status: http::status::NOT_FOUND,
							});
						}
					} else {
						print!("network warning: received an unrecognized http request from '{}', contents:'{}'\n", addr, String::from_utf8_lossy(raw_request));

						write_response(&mut tcpstream, trans_mem, Response{
							// content: None,
							status: http::status::METHOD_NOT_ALLOWED,
						});
					}
				},
				Err(()) => {
					print!("network warning: received a tcp request that was not valid http from '{}', contents:'{}'\n", addr, String::from_utf8_lossy(raw_request));
				},
			}
		} else {
			match TcpListener::bind(http_server.addr) {
				Err(v) => {
					print!("network error: failed to bind to '{}', got error '{}'", http_server.addr, v);
					http_server.disabled = true;
					break;
				},
				Ok(l) => {
					match l.set_nonblocking(true) {
						Err(e) => {
							print!("network error: failed to bind to '{}', got error '{}'", http_server.addr, e);
							http_server.disabled = true;
							break;
						},
						Ok(()) => {
							http_server.listener = Some(l);
						},
					}
				},
			}
		};
	}
	return output;
}

fn parse_request(raw_request: &[u8]) -> Result<Request, ()> {
	let request = match std::str::from_utf8(raw_request) {
		Ok(v) => v,
		Err(e) => return Err(()),
	};
	let mut parts = request.split(' ');
	let method = match parts.next() {
		Some(v) => v.trim(),
		None => return Err(()),
	};
	let path = match parts.next() {
		Some(v) => v.trim(),
		None => return Err(()),
	};
	let http_version = match parts.next() {
		Some(v) => v.trim(),
		None => return Err(()),
	};
	Ok(Request {
		http_version: http_version,
		method: method,
		path: path,
	})
}

fn encode_response<'a>(stack: &mut Vec<u8>, response: Response) -> &'a [u8] {
	let start = stack.len();
	stack.extend_from_slice(b"HTTP/1.0 ");
	stack.extend_from_slice(response.status.as_bytes());
	stack.extend_from_slice(b"\r\n");
	let end = stack.len();
	let ptr = stack.as_ptr();
	return unsafe {
		std::slice::from_raw_parts(ptr.add(start), end)
	}
}
fn write_response(tcpstream: &mut TcpStream, trans_mem: &mut Vec<u8>, response: Response) {
	match tcpstream.write_all(encode_response(trans_mem, response)) {
		Ok(()) => (),
		Err(e) => {

		}
	}
}

