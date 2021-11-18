//By Mami
use crate::config;
use crate::util;


pub struct SneakyMouseServer<'a> {
	pub redis_con : redis::Connection,
	pub redis_address : &'a str,
}

pub fn get_event_list() -> Vec<&'static [u8]> {
	//Normally I wouldn't have a function return an allocation, but this is only called once by main to configure itself
	vec![//this list is considered unordered
		b"debug",
		b"sm-cheese:request",
		b"sm-cheese:collected",
		b"sm-cheese:spawn",
		b"sm-cheese:promoted",
	]
}

pub fn server_event_received(server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], kvs : &[(&[u8], &[u8])]) -> Option<bool> {
	match event_name {
		b"debug" => {
			print!("debug event: {} <", String::from_utf8_lossy(event_uid));
			for (i, (key, val)) in kvs.iter().enumerate() {
				print!("{}:{}", String::from_utf8_lossy(key), String::from_utf8_lossy(val));
				if i+1 == kvs.len() {
					print!("> {}\n", server_state.redis_address);
				} else {
					print!(", ");
				}
			}

			// let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("Could not SET my_key to redis database");
			// let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("Could not GET my_key from redis database");
			// print!("{}\n", val);
		}
		b"sm-cheese:request" => {

		}
		b"sm-cheese:collected" => {

		}
		b"sm-cheese:spawn" => {

		}
		b"sm-cheese:promoted" => {

		}
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, {}, please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Some(true)
}

