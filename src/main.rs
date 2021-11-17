// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;

const REDIS_PRIMARY_IN_STREAM : &str = "sneaky_mouse_in";
const REDIS_STREAM_TIMEOUT_MS : i32 = 5000;
const REDIS_INIT_STREAM_ID_KEY : &str = "sneaky_mouse_in-last_used_id";

fn main() {
	let redis_address: String;
	match std::env::var("REDIS_ADDRESS") {
		Ok(val) => redis_address = val,
		Err(_e) => redis_address = String::from("redis://127.0.0.1/"),
	}
	let client = redis::Client::open(redis_address).expect("Could not connect");
	let mut con = client.get_connection().expect("Could not connect");

	// let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("Could not SET my_key to redis database");
	// let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("Could not GET my_key from redis database");
	// print!("{}\n", val);


	let mut last_id;
	let result_id : Result<redis::Value, redis::RedisError> = redis::cmd("GET").arg(REDIS_INIT_STREAM_ID_KEY).query(&mut con);
	match result_id {
		Ok(id_data) => match id_data {
			redis::Value::Data(id_raw) => last_id = String::from_utf8_lossy(&id_raw).into_owned(),
			_ => last_id = String::from("0-0"),
		}
		Err(error) => {
			panic!("redis error, did the server connection die?: {:?}\n", error);
			//TODO: attempt recovery
			// last_id = String::from("0-0");
		}
	}

	loop {
		// let _ : redis::Value = redis::cmd("XADD").arg(REDIS_PRIMARY_IN_STREAM).arg("*").arg("my_key").arg("my_val").query(&mut con).expect("XADD failed");
		let response : Result<redis::Value, redis::RedisError> = redis::cmd("XREAD").arg("COUNT").arg(1).arg("BLOCK").arg(REDIS_STREAM_TIMEOUT_MS).arg("STREAMS").arg(REDIS_PRIMARY_IN_STREAM).arg(&last_id).query(&mut con);

		match response {
			Ok(response_data) => {
				// print!("response_data = {:?}\n", response_data);
				let mut valid_response = false;
				if let redis::Value::Bulk(stream_responses) = response_data {
					if let redis::Value::Bulk(stream_response) = &stream_responses[0] {
						if let redis::Value::Bulk(stream_messages) = &stream_response[1] {
							if let redis::Value::Bulk(message) = &stream_messages[0] {
								if let redis::Value::Data(message_id_raw) = &message[0] {
									if let redis::Value::Bulk(message_body) = &message[1] {
										if let redis::Value::Data(message_key_raw) = &message_body[0] {
											if let redis::Value::Data(message_val_raw) = &message_body[1] {//lol I think redis overdoes its data packing, at least its not json..
												valid_response = true;
												let message_id_str = String::from_utf8_lossy(message_id_raw);
												let message_key = String::from_utf8_lossy(message_key_raw);
												let message_val = String::from_utf8_lossy(message_val_raw);
												print!("message id {:?} with key-val {:?}-{:?}\n", message_id_str, message_key, message_val);
												last_id = message_id_str.into_owned();
												let result_set : Result<redis::Value, redis::RedisError> = redis::cmd("SET").arg(REDIS_INIT_STREAM_ID_KEY).arg(&last_id).query(&mut con);
												match result_set {
													Ok(_) => {},
													Err(error) => {
														print!("redis error, the last consumed message was not saved!: {:?}\n", error);
														//TODO: reattempt to save last_id
													}
												}
											}
										}
									}
								}
							}
						}
					}
				} else if let redis::Value::Nil = response_data {
					valid_response = true;
					print!("no messages received before timeout, trying again...\n");
				}
				if !valid_response {
					panic!("critical error: redis response does not match expected specification\n");
				}
			}
			Err(error) => {
				panic!("redis error, did the server connection die?: {:?}\n", error);
				//TODO: attempt recovery
			}
		}
	}

	// let query_as_str : String = redis::from_redis_value(&query).expect("Could not interpret redis stream query as a rust string");//This will cause the written error if run, because the returned query is not compatible with the string type! query, roughly speaking, is more of a multidimensional list than a string
	// print!("{}\n", query_as_str);
}
