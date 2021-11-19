//By Mami
use crate::config::*;
use crate::event;
use event::SneakyMouseServer;
use redis::FromRedisValue;

pub fn push_u64(mem : &mut Vec<u8>, i : u64) {
	if i >= 10 {
		push_u64(mem, i/10);
	}
	mem.push(((i%10) as u8) + 48);
}

pub fn to_u64(mem : &[u8]) -> Option<u64> {//eats leading 0s
	let mut i : u64 = 0;
	for c in mem {
		if *c >= 48u8 && *c <= 48 + 9 {
			i += (*c - 48) as u64;
			i *= 10;
		} else {
			return None;
		}
	}
	return Some(i);
}
pub fn to_bool(mem : &[u8]) -> Option<bool> {//eats leading 0s
	match mem {
		b"true" => Some(true),
		b"false" => Some(false),
		_ => None,
	}
}

pub fn lookup_user_uuid(server_state : &mut SneakyMouseServer, user_identifier : &[u8]) -> Option<u64> {
	let mut cmd = redis::cmd("HGET");
	cmd.arg(KEY_USERUUID_HM).arg(user_identifier);
	match FromRedisValue::from_redis_value(&auto_retry_cmd(server_state, &mut cmd)?) {
		Ok(uuid) => Some(uuid),
		Err(_) => {//assuming that the user does not exist
			let mut cmd = redis::Cmd::incr(KEY_MAXUUID, 1i32);
			match FromRedisValue::from_redis_value(&auto_retry_cmd(server_state, &mut cmd)?) {
				Ok(uuid) => {
					let mut cmd = redis::cmd("HSET");
					cmd.arg(KEY_USERUUID_HM).arg(user_identifier).arg(uuid);
					auto_retry_cmd(server_state, &mut cmd)?;
					//TODO: set up user profile
					Some(uuid)
				}
				Err(_) => None
			}
		}
	}
}

pub fn find_val<'a>(key : &str, keys : &[&[u8]], vals : &[&'a[u8]]) -> Option<&'a[u8]> {
	for (i, cur_key) in keys.iter().enumerate() {
		if &key.as_bytes() == cur_key {
			return Some(vals[i]);
		}
	}
	return None;
}

pub fn find_u64_field_or_default(key : &'static str, default : u64, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> u64 {
	match find_val(key, keys, vals) {
		Some(raw) => match to_u64(raw) {
			Some(i) => i,
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				default
			}
		}
		None => default
	}
}

// #[derive(Clone, Copy, Debug)]
pub fn connect_to(redis_address : &str) -> Option<redis::Connection> {
	for _ in 1..=REDIS_RETRY_CON_MAX_ATTEMPTS {
		match redis::Client::open(redis_address) {
			Ok(client) => match client.get_connection() {
				Ok(con) => {
					print!("successfully connected to server\n");
					return Some(con);
				}
				Err(error) => {
					print!("failed to connect to '{}': {}\n", redis_address, error);
				}
			}
			Err(error) => panic!("could not parse redis url \'{}\': {}\n", redis_address, error)
		}
		std::thread::sleep(std::time::Duration::from_secs(REDIS_TIME_BETWEEN_RETRY_CON));
	}

	print!("connection attempts to exceeded {}, shutting down: contact an admin to restart the redis server\n", REDIS_RETRY_CON_MAX_ATTEMPTS);
	return None;
}

// #[derive(Clone, Copy, Debug)]
pub fn auto_retry_cmd<T : redis::FromRedisValue>(server_state : &mut event::SneakyMouseServer, cmd : &redis::Cmd) -> Option<T> {
	//Only returns None if a connection cannot be established to the server, only course of action is to shut down until an admin intervenes
	//NOTE: this can trigger a long thread::sleep() if reconnection fails
	match cmd.query(&mut server_state.redis_con) {
		Ok(data) => return Some(data),
		Err(error) => match error.kind() {
			redis::ErrorKind::InvalidClientConfig => {
				panic!("fatal error: the redis command was invalid {}\n", error);
			}
			redis::ErrorKind::TypeError => {
				panic!("fatal error: TypeError thrown by redis {}\n", error);
			}
			_ => {
				print!("lost connection to the server: {}\n", error);
				print!("attempting to reconnect\n");

				let con = connect_to(&server_state.redis_address[..])?;
				server_state.redis_con = con;
				match cmd.query(&mut server_state.redis_con) {
					Ok(data) => return Some(data),
					Err(error) => {
						print!("connection immediately failed on retry, shutting down: {}\n", error);
						return None
					}
				}
			}
		}
	}
}

pub fn send_error(server_state : &mut SneakyMouseServer, error : &String) {
	//Unlike all of our other functions, this one will only attempt to send the error to redis once and then move on if it fails
	let mut cmd = redis::cmd("XADD");
	cmd.arg(EVENT_DEBUG_ERROR).arg("*");
	cmd.arg(FIELD_MESSAGE).arg(error);

	match cmd.query::<redis::Value>(&mut server_state.redis_con) {
		Ok(_) => (),
		Err(error) => match error.kind() {
			redis::ErrorKind::InvalidClientConfig => {
				panic!("fatal error: the redis command was invalid {}\n", error);
			}
			redis::ErrorKind::TypeError => {
				panic!("fatal error: TypeError thrown by redis {}\n", error);
			}
			_ => {
				print!("lost connection to the server: {}\n", error);
				print!("we will not attempt to reconnect\n");
			}
		}
	}
}

pub fn push_kvs(error : &mut String, keys : &[&[u8]], vals : &[&[u8]]) {
	error.push_str("{{\n");
	for (i, key) in keys.iter().enumerate() {
		error.push_str(&String::from_utf8_lossy(key).into_owned());//TODO: remove this allocation
		error.push_str(":");
		error.push_str(&String::from_utf8_lossy(vals[i]).into_owned());
		if i + 1 == keys.len() {
			error.push_str("\n}}");
		} else {
			error.push_str(",\n");
		}
	}
}

pub fn invalid_value(server_state : &mut event::SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]], field : &'static str) {
	let mut error = format!("invalid event error: field '{}' had an incorrect value, the event will still be executed with default values, name:{} id:{} contents:", field, String::from_utf8_lossy(event_name), String::from_utf8_lossy(event_uid));
	push_kvs(&mut error, keys, vals);

	print!("{}\n", error);
	send_error(server_state, &error);
}

pub fn missing_field(server_state : &mut event::SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]], field : &'static str) {
	let mut error = format!("invalid event error: missing critical field '{}', name:{} id:{} contents:", field, String::from_utf8_lossy(event_name), String::from_utf8_lossy(event_uid));
	push_kvs(&mut error, keys, vals);

	print!("{}\n", error);
	send_error(server_state, &error);
}


pub fn mismatch_spec(server_state : &mut SneakyMouseServer, file : &'static str, line : u32) {
	let error = format!("fatal error {} line {}: redis response does not match expected specification, server will shutdown now", file, line);

	print!("{}\n", error);
	send_error(server_state, &error);
	panic!("shutting down due to fatal error\n");
}
