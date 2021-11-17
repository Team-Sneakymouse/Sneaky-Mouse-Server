// #![feature(proc_macro_hygiene, decl_macro)]

// #[macro_use]
extern crate redis;

const REDIS_PRIMARY_IN_STREAM : &str = "sneaky-mouse-in";
const REDIS_STREAM_TIMEOUT_MS : i32 = 5000;

fn main() {
	let redis_address: String;
	match std::env::var("REDIS_ADDRESS") {
		Ok(val) => redis_address = val,
		Err(_e) => redis_address = String::from("redis://127.0.0.1/"),
	}
	let client = redis::Client::open(redis_address).expect("Could not connect");
	let mut con = client.get_connection().expect("Could not connect");

	let _ : redis::Value = redis::cmd("SET").arg("my_key").arg(42i32).query(&mut con).expect("Could not SET my_key to redis database");
	let val : String = redis::cmd("GET").arg("my_key").query(&mut con).expect("Could not GET my_key from redis database");
	print!("{}\n", val);


	let _ : redis::Value = redis::cmd("XADD").arg(REDIS_PRIMARY_IN_STREAM).arg("*").arg("my_key").arg("my_val").query(&mut con).expect("XADD failed");
	//NOTE: XRANGE does not "consume" messages from the stream, so every time the above code is run query will get longer, because my_stream will have all the unconsumed messages from the previous runs

	let mut last_id = String::from("$");
	loop {
		let response : Result<redis::Value, redis::RedisError> = redis::cmd("XREAD").arg("COUNT").arg(1).arg("BLOCK").arg(REDIS_STREAM_TIMEOUT_MS).arg("STREAMS").arg(REDIS_PRIMARY_IN_STREAM).arg(&last_id).query(&mut con);

		match response {
			Ok(message) => {
				print!("message = {:?}\n", message);
			}
			Err(error) => {
				print!("error = {:?}\n", error);
			}
		}
	}

	// let query_as_str : String = redis::from_redis_value(&query).expect("Could not interpret redis stream query as a rust string");//This will cause the written error if run, because the returned query is not compatible with the string type! query, roughly speaking, is more of a multidimensional list than a string
	// print!("{}\n", query_as_str);
}
