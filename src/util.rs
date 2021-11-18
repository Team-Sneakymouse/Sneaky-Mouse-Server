//By Mami
use crate::config;
use crate::event;


// #[derive(Clone, Copy, Debug)]
pub fn connect_to(redis_address : &str) -> Option<redis::Connection> {
	for _ in 1..config::REDIS_RETRY_CON_MAX_ATTEMPTS {
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
	//NOTE: this can trigger a long thread::sleep() if reconnection fails
	match cmd.query(&mut server_state.redis_con) {
		Ok(data) => return Some(data),
		Err(error) => {
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
