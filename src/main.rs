// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;

fn vec_drop_and_reuse<'a, 'b, T : ?Sized>(mut vec : Vec<&'a T>) -> Vec<&'b T> {
	//this function is a superior version of .clear()
	//the borrow checker does not acknowledge that .clear() drops all borrowed references, so we have to force it too
	vec.clear();
	unsafe {std::mem::transmute(vec)}
}


const REDIS_STREAM_TIMEOUT_MS : i32 = 2000;
const REDIS_STREAM_READ_COUNT : i32 = 55;
const REDIS_TIME_BETWEEN_RETRY_CON : u64 = 5;
const REDIS_RETRY_CON_MAX_ATTEMPTS : i32 = 5;
const REDIS_PRIMARY_IN_STREAM : &str = "sneaky_mouse_in";
const REDIS_INIT_STREAM_ID_KEY : &str = "sneaky_mouse_in-last_used_id";

struct SneakyMouseServer<'a> {
	redis_con : redis::Connection,
	redis_address : &'a str,
}

fn redis_connect_to(redis_address : &str) -> Option<redis::Connection> {
	for _ in 1..REDIS_RETRY_CON_MAX_ATTEMPTS {
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

fn redis_auto_retry_cmd<T : redis::FromRedisValue>(server_state : &mut SneakyMouseServer, cmd : &redis::Cmd) -> Option<T> {
	//NOTE: this can trigger a long thread::sleep() if reconnection fails
	match cmd.query(&mut server_state.redis_con) {
		Ok(data) => return Some(data),
		Err(error) => {
			print!("Lost connection to the server: {}\n", error);
			print!("Attempting to reconnect\n");

			let con = redis_connect_to(&server_state.redis_address[..])?;
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

fn sneaky_mouse_in_message_received(_server_state : &mut SneakyMouseServer, _id : &str, _keys : &[&[u8]], _vals : &[&[u8]]) -> Option<bool> {
	Some(true)
}


fn server_main() -> Option<bool> {
	let redis_address: String;
	match std::env::var("REDIS_ADDRESS") {
		Ok(val) => redis_address = val,
		Err(_e) => redis_address = String::from("redis://127.0.0.1/"),
	}
	let con = redis_connect_to(&redis_address[..])?;

	let mut server_state = SneakyMouseServer{redis_con : con, redis_address : &redis_address[..]};

	// let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("Could not SET my_key to redis database");
	// let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("Could not GET my_key from redis database");
	// print!("{}\n", val);


	let mut last_id : String;
	// let query = ;
	let id_data = redis_auto_retry_cmd(&mut server_state, redis::cmd("GET").arg(REDIS_INIT_STREAM_ID_KEY))?;
	match id_data {
		redis::Value::Data(id_raw) => last_id = String::from_utf8_lossy(&id_raw).into_owned(),//TODO: improve this
		_ => last_id = String::from("0-0"),
	}


	let mut message_keys_mem = Vec::<&[u8]>::new();
	let mut message_vals_mem = Vec::<&[u8]>::new();
	loop {
		// let _ : redis::Value = redis::cmd("XADD").arg(REDIS_PRIMARY_IN_STREAM).arg("*").arg("my_key").arg("my_val").query(&mut con).expect("XADD failed");
		// let query = redis::cmd("XREAD").arg("COUNT").arg(REDIS_STREAM_READ_COUNT).arg("BLOCK").arg(REDIS_STREAM_TIMEOUT_MS).arg("STREAMS").arg(REDIS_PRIMARY_IN_STREAM).arg(&last_id);
		let mut cmd = redis::cmd("XREAD");
		cmd.arg("COUNT").arg(REDIS_STREAM_READ_COUNT).arg("BLOCK").arg(REDIS_STREAM_TIMEOUT_MS).arg("STREAMS").arg(REDIS_PRIMARY_IN_STREAM).arg(&last_id);
		let response : redis::Value = redis_auto_retry_cmd(&mut server_state, &mut cmd)?;

		//NOTE(mami): this code was built upon the principle of non-pesimization; as such there are shorter ways to do this, but most of them are not fast nor robust
		if let redis::Value::Bulk(stream_responses) = response {
			if let redis::Value::Bulk(stream_response) = &stream_responses[0] {
				if let redis::Value::Bulk(stream_messages) = &stream_response[1] {
					for message_data in stream_messages {
						if let redis::Value::Bulk(message) = message_data {
							if let redis::Value::Data(message_id_raw) = &message[0] {
								if let redis::Value::Bulk(message_body) = &message[1] {
									let mut message_keys = message_keys_mem;
									let mut message_vals = message_vals_mem;
									for i in 0..message_body.len()/2 {
										if let redis::Value::Data(message_key_raw) = &message_body[i] {
											if let redis::Value::Data(message_val_raw) = &message_body[i + 1] {
												message_keys.push(&message_key_raw[..]);
												message_vals.push(&message_val_raw[..]);
											} else {
												panic!("critical error: redis response does not match expected specification\n");
											}
										} else {
											panic!("critical error: redis response does not match expected specification\n");
										}
									}
									let message_id_str : &str = std::str::from_utf8(message_id_raw).expect("critical error: redis returned a non-utf8 message id; did we misunderstand the specification?");


									sneaky_mouse_in_message_received(&mut server_state, message_id_str, &message_keys[..], &message_vals[..])?;


									last_id.clear();
									last_id.push_str(message_id_str);//this avoids allocating
									let result_set : Result<redis::Value, redis::RedisError> = redis::cmd("SET").arg(REDIS_INIT_STREAM_ID_KEY).arg(&last_id).query(&mut server_state.redis_con);
									if let Err(error) = result_set {
										print!("redis error, the last consumed message was not saved!: {}\n", error);
										//TODO: reattempt to save last_id?
									}
									message_keys_mem = vec_drop_and_reuse(message_keys);
									message_vals_mem = vec_drop_and_reuse(message_vals);
								} else {
									panic!("critical error: redis response does not match expected specification\n");
								}
							} else {
								panic!("critical error: redis response does not match expected specification\n");
							}
						} else {
							panic!("critical error: redis response does not match expected specification\n");
						}
					}
				} else {
					panic!("critical error: redis response does not match expected specification\n");
				}
			} else {
				panic!("critical error: redis response does not match expected specification\n");
			}
		} else if let redis::Value::Nil = response {
			print!("no messages received before timeout, last id was {}; trying again...\n", last_id);
		}
	}
}

fn main() {
	server_main();
}
