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
	if mem.len() > 20 {return Some(u64::MAX);}
	let mut i : u64 = 0;
	for c in mem {
		if *c >= 48u8 && *c <= 48 + 9 {
			i = u64::saturating_add(i, (*c - 48) as u64);
			i = u64::saturating_mul(i, 10);
		} else {
			return None;
		}
	}
	return Some(i);
}
pub fn to_u32(mem : &[u8]) -> Option<u32> {//eats leading 0s
	if mem.len() > 10 {return Some(u32::MAX);}
	let mut i : u32 = 0;
	for c in mem {
		if *c >= 48u8 && *c <= 48 + 9 {
			i = u32::saturating_add(i, (*c - 48) as u32);
			i = u32::saturating_mul(i, 10);
		} else {
			return None;
		}
	}
	return Some(i);
}
pub fn to_i64(mem : &[u8]) -> Option<i64> {//eats leading 0s
	let mut i : i64 = 0;
	let s = if mem[0] == 45 {1} else {0};
	for c in &mem[s..] {
		if *c >= 48u8 && *c <= 48 + 9 {
			i = i64::saturating_add(i, (*c - 48) as i64);
			i = i64::saturating_mul(i, 10);
		} else {
			return None;
		}
	}
	if s == 1 {
		return Some(-i);
	} else {
		return Some(i);
	}
}
pub fn to_i32(mem : &[u8]) -> Option<i32> {//eats leading 0s
	let mut i : i32 = 0;
	let s = if mem[0] == 45 {1} else {0};
	for c in &mem[s..] {
		if *c >= 48u8 && *c <= 48 + 9 {
			i = i32::saturating_add(i, (*c - 48) as i32);
			i = i32::saturating_mul(i, 10);
		} else {
			return None;
		}
	}
	if s == 1 {
		return Some(-i);
	} else {
		return Some(i);
	}
}
pub fn to_f32(mem : &[u8]) -> Option<f32> {//eats leading 0s
	if let Ok(s) = std::str::from_utf8(mem) {
	if let Ok(v) = s.parse::<f32>() {
	return Some(v);
	}
	}
	return None;
}
pub fn to_bool(mem : &[u8]) -> Option<bool> {//eats leading 0s
	match mem {
		b"true" => Some(true),
		b"false" => Some(false),
		b"1" => Some(true),
		b"0" => Some(false),
		_ => None,
	}
}
pub fn is_str_null(mem : &[u8]) -> bool {
	return mem == b"" || mem == b"null"
}


pub fn get_u64_from_val_or_panic(server_state : &mut SneakyMouseServer, val : &redis::Value) -> u64 {
	match FromRedisValue::from_redis_value(val) {
		Ok(uuid) => uuid,
		Err(_) => {
			mismatch_spec(server_state, file!(), line!());
			0//unreachable
		}
	}
}
pub fn lookup_user_uuid(server_state : &mut SneakyMouseServer, user_identifier : &[u8]) -> Option<u64> {
	let mut cmd = redis::cmd("HGET");
	cmd.arg(KEY_USERUUID_HM).arg(user_identifier);
	match FromRedisValue::from_redis_value(&auto_retry_cmd(server_state, &mut cmd)?) {
		Ok(uuid) => Some(uuid),
		Err(_) => {//assuming that the user does not exist
			let mut cmd = redis::Cmd::incr(KEY_MAXUUID, 1i32);
			let val = &auto_retry_cmd(server_state, &mut cmd)?;
			let uuid = get_u64_from_val_or_panic(server_state, val);
			let mut cmd = redis::cmd("HSET");
			cmd.arg(KEY_USERUUID_HM).arg(user_identifier).arg(uuid);
			auto_retry_cmd(server_state, &mut cmd)?;
			//TODO: set up user profile
			Some(uuid)
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

pub fn find_data_field(key : &'static str, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<Vec<u8>> {
	match find_val(key, keys, vals) {
		Some(raw) => Some(Vec::from(raw)),
		None => None,
	}
}
pub fn find_bool_field(key : &'static str, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<bool> {
	match find_val(key, keys, vals) {
		Some(raw) => match to_bool(raw) {
			Some(i) => Some(i),
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				None
			}
		}
		None => None
	}
}
pub fn find_u32_field(key : &'static str, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<u32> {
	match find_val(key, keys, vals) {
		Some(raw) => match to_u32(raw) {
			Some(i) => Some(i),
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				None
			}
		}
		None => None
	}
}
pub fn find_i32_field(key : &'static str, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<i32> {
	match find_val(key, keys, vals) {
		Some(raw) => match to_i32(raw) {
			Some(i) => Some(i),
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				None
			}
		}
		None => None
	}
}
pub fn find_f32_field(key : &'static str, server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<f32> {
	match find_val(key, keys, vals) {
		Some(raw) => match to_f32(raw) {
			Some(i) => Some(i),
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				None
			}
		}
		None => None
	}
}

pub fn save_cheese(server_state : &mut SneakyMouseServer, cmd : &mut redis::Cmd, cheese : &CheeseData) {
	cmd.arg(FIELD_IMAGE).arg(&cheese.image);
	if let Some(s) = &cheese.radicalizes {
		cmd.arg(FIELD_RADICALIZES).arg(s);
	}
	cmd.arg(FIELD_TIME_MIN).arg(cheese.time_min);
	cmd.arg(FIELD_TIME_MAX).arg(cheese.time_max);
	cmd.arg(FIELD_SIZE).arg(cheese.size);
	cmd.arg(FIELD_ORIGINAL_SIZE).arg(cheese.original_size);
	cmd.arg(FIELD_SQUIRREL_MULT).arg(cheese.squirrel_mult);
	cmd.arg(FIELD_SILENT).arg(cheese.silent);
	cmd.arg(FIELD_EXCLUSIVE).arg(cheese.exclusive);
}



fn _get_cheese_from_val(server_state : &mut SneakyMouseServer, cheese_val : redis::Value, set_original_size : bool, trans_mem : &mut Vec<u8>) -> CheeseData {
	let mut cheese = generate_default_cheese();
	if let redis::Value::Bulk(vals) = cheese_val {
	
	if vals.len()%2 == 1 {mismatch_spec(server_state, file!(), line!());}
	let mut has_set_time = false;
	for i in 0..vals.len()/2 {
		if let redis::Value::Data(field) = &vals[i] {
		if let redis::Value::Data(val) = &vals[i + 1] {
		
		if field == FIELD_IMAGE.as_bytes() {
			cheese.image.clear();
			cheese.image.extend(val);
		} else if field == FIELD_RADICALIZES.as_bytes() {
			if is_str_null(&val[..]) {
				cheese.radicalizes = None;
			} else {
				let mut rad = Vec::new();
				rad.extend(val);
				cheese.radicalizes = Some(rad);
			}
		} else if field == FIELD_TIME_MIN.as_bytes() {
			if let Some(time) = to_u32(&val) {
				cheese.time_min = time;
				if !has_set_time {
					has_set_time = true;
					cheese.time_max = time;
				}
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_TIME_MIN, &val)}
		} else if field == FIELD_TIME_MAX.as_bytes() {
			if let Some(time) = to_u32(&val) {
				cheese.time_max = time;
				if !has_set_time {
					has_set_time = true;
					cheese.time_min = time;
				}
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_TIME_MAX, &val)}
		} else if field == FIELD_SIZE.as_bytes() {
			if let Some(size) = to_i32(&val) {
				cheese.size = size;
				if set_original_size {
					cheese.original_size = size;
				}
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_SIZE, &val)}
		} else if field == FIELD_SQUIRREL_MULT.as_bytes() {
			if let Some(mult) = to_f32(&val) {
				cheese.squirrel_mult = mult;
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_SIZE, &val)}
		} else if field == FIELD_SILENT.as_bytes() {
			if let Some(is) = to_bool(&val) {
				cheese.silent = is;
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_SIZE, &val)}
		} else if field == FIELD_EXCLUSIVE.as_bytes() {
			if let Some(is) = to_bool(&val) {
				cheese.exclusive = is;
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_SIZE, &val)}
		} else if !set_original_size && field == FIELD_ORIGINAL_SIZE.as_bytes() {
			if let Some(size) = to_i32(&val) {
				cheese.original_size = size;
			} else {invalid_db_entry(server_state, &trans_mem[..], FIELD_ORIGINAL_SIZE, &val)}
		}
		} else {mismatch_spec(server_state, file!(), line!());}
		} else {mismatch_spec(server_state, file!(), line!());}
	}
	} else {mismatch_spec(server_state, file!(), line!());}
	trans_mem.clear();
	return cheese;
}

pub fn get_cheese_from_id(server_state : &mut SneakyMouseServer, cheese_id : &[u8], trans_mem : &mut Vec<u8>) -> Option<CheeseData> {
	trans_mem.extend(KEY_CHEESE_PREFIX.as_bytes());
	trans_mem.extend(cheese_id);
	let mut cmdgetdata = redis::cmd("HGETALL");
	cmdgetdata.arg(&trans_mem[..]);

	let val = auto_retry_cmd(server_state, &mut cmdgetdata)?;
	return Some(_get_cheese_from_val(server_state, val, true, trans_mem));
}
pub fn get_cheese_from_uid(server_state : &mut SneakyMouseServer, cheese_uid : u64, trans_mem : &mut Vec<u8>) -> Option<CheeseData> {
	trans_mem.extend(KEY_CHEESE_DATA_PREFIX.as_bytes());
	push_u64(trans_mem, cheese_uid);
	let mut cmdgetdata = redis::cmd("HGETALL");
	cmdgetdata.arg(&trans_mem[..]);

	let val = auto_retry_cmd(server_state, &mut cmdgetdata)?;
	return Some(_get_cheese_from_val(server_state, val, false, trans_mem));
}


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

pub fn invalid_db_entry(server_state : &mut event::SneakyMouseServer, key : &[u8], field : &str, val : &[u8]) {
	let mut error = format!("database error: key '{}:{}' had incorrect value {}, will attempt recovery with default values", String::from_utf8_lossy(key), field, String::from_utf8_lossy(val));

	print!("{}\n", error);
	send_error(server_state, &error);
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
