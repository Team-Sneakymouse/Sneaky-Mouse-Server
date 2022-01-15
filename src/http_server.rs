
use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::time::{Duration};

use crate::util::*;
use crate::config::*;

struct Request<'a> {
	// http_version: &'a str,
	method: &'a str,
	path: &'a str,
}
struct Response {
	// content: Option<&'a [u8]>,
	// header: &'a str,
	status: &'static str,
}

pub fn poll(http_server: &mut HTTPServer, trans_mem: &mut Vec<u8>) -> HTTPServerOutput {
	//NOTE: the following code is insecure for a variety of reasons (no authentication, slow loris, no https, etc), do not open the port to the internet
	let mut output = HTTPServerOutput{
		shutdown: false,
	};
	while !http_server.disabled {
		trans_mem.clear();
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

			match tcpstream.set_read_timeout(Some(Duration::from_millis((http::CONNECTION_TIMEOUT*1000.0) as u64))) {
				Ok(n) => n,
				Err(e) => {
					print!("network error: failed to set timeout on tcp stream, got error '{}'\n", e);
					break;
				}
			}
			match tcpstream.set_write_timeout(Some(Duration::from_millis((http::CONNECTION_TIMEOUT*1000.0) as u64))) {
				Ok(n) => n,
				Err(e) => {
					print!("network error: failed to set timeout on tcp stream, got error '{}'\n", e);
					break;
				}
			}
			match tcpstream.set_nonblocking(false) {
				Ok(n) => n,
				Err(e) => {
					print!("network error: failed to set_nonblocking on tcp stream, got error '{}'\n", e);
					break;
				}
			}

			let raw_request = {//read up to first line
				let start = trans_mem.len();

				let mut got_newline = false;
				let mut mem = [0; 1028];
				let mut total_read = 0;
				while !got_newline && total_read < http::REQUEST_SIZE_MAX {
					let n = match tcpstream.read(&mut mem) {
						Ok(n) => n,
						Err(e) => match e.kind() {
							io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
								print!("network warning: tcp stream from '{}' timed out, got error '{}'\n", addr, e);
								continue;
							},
							_ => {
								print!("network error: failed to read tcp stream, got error '{}'\n", e);
								break;
							},
						}
					};
					for i in 0..n {
						total_read += 1;
						trans_mem.push(mem[i]);
						if mem[i] == ASCII_NEWLINE {
							got_newline = true;
							break;
						}
					}
				}
				let end = trans_mem.len();
				let ptr = trans_mem.as_ptr();
				if got_newline {
					unsafe {
						std::slice::from_raw_parts(ptr.add(start), end - start)
					}
				} else {
					print!("network warning: '{}' sent http request longer than cap, contents:'{}'\n", addr, String::from_utf8_lossy(&trans_mem[start..end]));
					continue;
				}
			};

			// print!("http: client '{}' sent request:\n {}", addr, String::from_utf8_lossy(raw_request));

			match parse_request(raw_request) {
				Ok(request) => {
					if request.method == "POST" {//POST /shutdown
						if request.path == "/shutdown" {
							output.shutdown = true;

							write_response(&mut tcpstream, trans_mem, Response{
								// content: None,
								status: http::status::OK,
							});

							print!("network: received a POST /shutdown request from '{}', shutting down\n", addr);
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
					print!("network error: failed to bind to '{}', got error '{}', disabling http server please contact admin", http_server.addr, v);
					http_server.disabled = true;
					break;
				},
				Ok(l) => {
					match l.set_nonblocking(true) {
						Err(e) => {
							print!("network error: failed to bind to '{}', got error '{}', disabling http server please contact admin", http_server.addr, e);
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
	// let http_version = match parts.next() {
	// 	Some(v) => v.trim(),
	// 	None => return Err(()),
	// };
	Ok(Request {
		// http_version: http_version,
		method: method,
		path: path,
	})
}

fn encode_response<'a>(stack: &mut Vec<u8>, response: Response) -> &'a [u8] {
	let start = stack.len();
	stack.extend_from_slice(b"HTTP/1.0 ");
	stack.extend_from_slice(response.status.as_bytes());
	stack.extend_from_slice(b"\r\n");
	stack.extend_from_slice(b"\r\n");
	let end = stack.len();
	let ptr = stack.as_ptr();
	return unsafe {
		std::slice::from_raw_parts(ptr.add(start), end - start)
	}
}
fn write_response(tcpstream: &mut TcpStream, trans_mem: &mut Vec<u8>, response: Response) {
	let data = encode_response(trans_mem, response);
	match tcpstream.write_all(data) {
		Ok(()) => {
			match tcpstream.flush() {
				_ => (),
			}
		},
		Err(e) => {

		}
	}
}

