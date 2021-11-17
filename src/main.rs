// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;

const REDIS_PRIMARY_IN_STREAM : &str = "sneaky_mouse_in";
const REDIS_STREAM_TIMEOUT_MS : i32 = 2000;
const REDIS_STREAM_READ_COUNT : i32 = 55;
const REDIS_INIT_STREAM_ID_KEY : &str = "sneaky_mouse_in-last_used_id";

fn sneaky_mouse_in_message_received(_message_id : &str, _message_raw_keys : &[&[u8]], _message_raw_vals : &[&[u8]]) {

}


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


	let mut last_id : String;
	let result_id : Result<redis::Value, redis::RedisError> = Ok(redis::Value::Nil);
	// let result_id : Result<redis::Value, redis::RedisError> = redis::cmd("GET").arg(REDIS_INIT_STREAM_ID_KEY).query(&mut con);
	match result_id {
		Ok(id_data) => match id_data {
			redis::Value::Data(id_raw) => last_id = String::from_utf8_lossy(&id_raw).into_owned(),
			_ => last_id = String::from("0-0"),
		}
		Err(error) => {
			panic!("redis error, did the server connection die?: {}\n", error);
			//TODO: attempt recovery
			// last_id = String::from("0-0");
		}
	}

	loop {
		let mut message_keys = Vec::<&[u8]>::new();
		let mut message_vals = Vec::<&[u8]>::new();
		// let _ : redis::Value = redis::cmd("XADD").arg(REDIS_PRIMARY_IN_STREAM).arg("*").arg("my_key").arg("my_val").query(&mut con).expect("XADD failed");
		let response : Result<redis::Value, redis::RedisError> = redis::cmd("XREAD").arg("COUNT").arg(REDIS_STREAM_READ_COUNT).arg("BLOCK").arg(REDIS_STREAM_TIMEOUT_MS).arg("STREAMS").arg(REDIS_PRIMARY_IN_STREAM).arg(&last_id).query(&mut con);

		//NOTE(mami): this code was built upon the principle of non-pesimization; as such there are shorter ways to do this, but most of them are either not fast or not robust
		match response {
			Ok(response_data) => {
				// print!("response_data = {:?}\n", response_data);
				//lol I think redis overdoes its data packing, at least its not json..
				if let redis::Value::Bulk(stream_responses) = response_data {
					if let redis::Value::Bulk(stream_response) = &stream_responses[0] {
						if let redis::Value::Bulk(stream_messages) = &stream_response[1] {
							for message_data in stream_messages {
								if let redis::Value::Bulk(message) = message_data {
									if let redis::Value::Data(message_id_raw) = &message[0] {
										if let redis::Value::Bulk(message_body) = &message[1] {
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
											sneaky_mouse_in_message_received(message_id_str, &message_keys[..], &message_vals[..]);

											last_id.clear();
											last_id.push_str(message_id_str);//this avoids allocating
											let result_set : Result<redis::Value, redis::RedisError> = redis::cmd("SET").arg(REDIS_INIT_STREAM_ID_KEY).arg(&last_id).query(&mut con);
											match result_set {
												Ok(_) => (),
												Err(error) => {
													print!("redis error, the last consumed message was not saved!: {}\n", error);
													//TODO: reattempt to save last_id?
												}
											}
											message_keys.clear();
											message_vals.clear();
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
				} else if let redis::Value::Nil = response_data {
					print!("no messages received before timeout, last id was {}; trying again...\n", last_id);
				}

			}
			Err(error) => {
				panic!("redis error, did the server connection die?: {}\n", error);
				//TODO: attempt recovery
			}
		}
	}

	// let query_as_str : String = redis::from_redis_value(&query).expect("Could not interpret redis stream query as a rust string");//This will cause the written error if run, because the returned query is not compatible with the string type! query, roughly speaking, is more of a multidimensional list than a string
	// print!("{}\n", query_as_str);
}
