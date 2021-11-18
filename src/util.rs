//By Mami
use crate::config;
use crate::event;
use event::SneakyMouseServer;
use redis::FromRedisValue;

pub fn lookup_user_uuid(server_state : &mut SneakyMouseServer, user_identifier : &[u8]) -> Option<u64> {
	let mut cmd = redis::cmd("HMGET");
	cmd.arg(config::KEY_USERUUID_HM).arg(user_identifier);
	match FromRedisValue::from_redis_value(&auto_retry_cmd(server_state, &mut cmd)?) {
		Ok(uuid) => Some(uuid),
		Err(_) => {//assuming that the user does not exist
			let mut cmd = redis::Cmd::incr(config::KEY_MAXUUID, 1i32);
			match FromRedisValue::from_redis_value(&auto_retry_cmd(server_state, &mut cmd)?) {
				Ok(uuid) => {
					let mut cmd = redis::cmd("HMSET");
					cmd.arg(config::KEY_USERUUID_HM).arg(user_identifier).arg(uuid);
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

// #[derive(Clone, Copy, Debug)]
pub fn connect_to(redis_address : &str) -> Option<redis::Connection> {
	for _ in 1..=config::REDIS_RETRY_CON_MAX_ATTEMPTS {
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
		std::thread::sleep(std::time::Duration::from_secs(config::REDIS_TIME_BETWEEN_RETRY_CON));
	}

	print!("connection attempts to exceeded {}, shutting down: contact an admin to restart the redis server\n", config::REDIS_RETRY_CON_MAX_ATTEMPTS);
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
