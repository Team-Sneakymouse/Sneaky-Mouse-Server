//By Mami
// use crate::config;
use crate::util;


pub struct SneakyMouseServer<'a> {
	pub redis_con : redis::Connection,
	pub redis_address : &'a str,
}

pub fn get_event_list() -> Vec<&'static [u8]> {
	//Normally I wouldn't have a function return an allocation, but this is only called once by main to configure itself
	vec![//this list is considered unordered
		b"debug:console",
		b"sm-cheese:request",
		b"sm-cheese:collected",
		b"sm-cheese:spawn",
		b"sm-cheese:promoted",
	]
}

pub fn server_event_received(server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<bool> {
	match event_name {
		b"debug:console" => {
			print!("debug event: {} <", String::from_utf8_lossy(event_uid));
			for (i, key) in keys.iter().enumerate() {
				print!("{}:{}", String::from_utf8_lossy(key), String::from_utf8_lossy(vals[i]));
				if i+1 == keys.len() {
					print!("> {}\n", server_state.redis_address);
				} else {
					print!(", ");
				}
			}
		}
		b"sm-cheese:request" => {
			if let Some(userid) = util::find_val("user-id", keys, vals) {
				if let Some(username) = util::find_val("user-name", keys, vals) {
					if let Some(stream) = util::find_val("stream", keys, vals) {
						let uuid = util::lookup_user_uuid(server_state, userid);

					}
				}
			}
		}
		b"sm-cheese:collected" => {

		}
		b"sm-cheese:spawn" => {

		}
		b"sm-cheese:promoted" => {

		}
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, '{}', please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Some(true)
}

