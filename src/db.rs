//By Mami
use rand_pcg::*;
use crate::config::*;
use crate::config::layer::*;
use crate::util::*;
use crate::com;
use rand::{Rng};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


pub fn mismatch_spec(db: &mut LayerData, file: &'static str, line: u32) {
	let error = format!("fatal error: file:{} line:{}, redis response does not match expected specification", file, line);

	print!("{}\n", error);
	com::send_error(db, &error);
	panic!("shutting down due to fatal error\n");
}
fn unrecognized_db_entry(db: &mut LayerData, key: &[u8], field: &[u8], val: &[u8]) {
	let error = format!("database warning: key '{}:{}' is not recognized as a valid database entry, it had value '{}', will ignore", String::from_utf8_lossy(key), String::from_utf8_lossy(field), String::from_utf8_lossy(val));

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn invalid_db_key(db: &mut LayerData, key: &str, val: &[u8]) {
	let error = format!("database warning: key '{}' had unexpected value '{}', will attempt to use default value", key, String::from_utf8_lossy(val));

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn invalid_db_entry(db: &mut LayerData, key: &[u8], field: &str, val: &[u8]) {
	let error = format!("database warning: key '{}:{}' had unexpected value '{}', will attempt to use default value", String::from_utf8_lossy(key), field, String::from_utf8_lossy(val));

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn missing_db_entry(db: &mut LayerData, key: &[u8], field: &str) {
	let error = format!("database warning: key '{}:{}' had no value, will attempt to use default value", String::from_utf8_lossy(key), field);

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn invalid_db_uuid(db: &mut LayerData, key: &str, field: &[u8], val: &[u8]) {
	let error = format!("database warning: key '{}:{}' had unexpected value '{}', was expecting a valid uuid, will attempt to use default value", key, String::from_utf8_lossy(field), String::from_utf8_lossy(val));

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn invalid_db_entry_attempt_to_repair(db: &mut LayerData, key: &[u8], field: &[u8], val: &[u8]) {
	let error = format!("database warning: key '{}:{}' had incorrect value '{}', will attempt to repair entry", String::from_utf8_lossy(key), String::from_utf8_lossy(field), String::from_utf8_lossy(val));

	print!("{}\n", error);
	com::send_error(db, &error);
}
fn get_u64_from_val_or_panic(db: &mut LayerData, val: &redis::Value) -> u64 {
	match redis::FromRedisValue::from_redis_value(val) {
		Ok(uuid) => uuid,
		Err(_) => {
			mismatch_spec(db, file!(), line!());
			0//unreachable
		}
	}
}


fn _get_cheese_from_val<'a>(db: &mut LayerData, cheese_val: redis::Value, set_original_size: bool, key: &[u8]) -> CheeseData<'a> {

    let mut mem = Vec::<u8>::new();
	let mut image = layer::default::CHEESE_IMAGE;
	let mut radical_image = Some(layer::default::CHEESE_RADICAL_IMAGE);
	let mut time_min = layer::default::CHEESE_TIME_MIN;
	let mut time_max = layer::default::CHEESE_TIME_MAX;
	let mut size = layer::default::CHEESE_SIZE;
	let mut original_size = None;
	let mut squirrel_mult = 0.0;
	let mut gems = 0;
	let mut silent = false;
	let mut exclusive = false;

	if let redis::Value::Bulk(vals) = cheese_val {

	if vals.len()%2 == 1 {mismatch_spec(db, file!(), line!());}
	let mut has_set_time = false;
	for i2 in 0..vals.len()/2 {
		let i = i2*2;
		if let redis::Value::Data(field) = &vals[i] {
		if let redis::Value::Data(val) = &vals[i + 1] {

		if field == key::cheese::IMAGE.as_bytes() {
			image = push_u8s(&mut mem, val);
		} else if field == key::cheese::RADICAL.as_bytes() {
			if is_str_null(&val[..]) {
				radical_image = None;
			} else {
				radical_image = Some(push_u8s(&mut mem, val));
			}
		} else if field == key::cheese::TIME_MIN.as_bytes() {
			if let Some(time) = to_u32(&val) {
				time_min = time;
				if !has_set_time {
					has_set_time = true;
					time_max = time;
				}
			} else {invalid_db_entry(db, key, key::cheese::TIME_MIN, &val)}
		} else if field == key::cheese::TIME_MAX.as_bytes() {
			if let Some(time) = to_u32(&val) {
				time_max = time;
				if !has_set_time {
					has_set_time = true;
					time_min = time;
				}
			} else {invalid_db_entry(db, key, key::cheese::TIME_MAX, &val)}
		} else if field == key::cheese::SIZE.as_bytes() {
			if let Some(s) = to_i32(&val) {
				size = s;
			} else {invalid_db_entry(db, key, key::cheese::SIZE, &val)}
		} else if field == key::cheese::GEMS.as_bytes() {
			if let Some(s) = to_i32(&val) {
				gems = s;
			} else {invalid_db_entry(db, key, key::cheese::SIZE, &val)}
		} else if field == key::cheese::SQUIRREL_MULT.as_bytes() {
			if let Some(mult) = to_f32(&val) {
				squirrel_mult = mult;
			} else {invalid_db_entry(db, key, key::cheese::SIZE, &val)}
		} else if field == key::cheese::SILENT.as_bytes() {
			if let Some(is) = to_bool(&val) {
				silent = is;
			} else {invalid_db_entry(db, key, key::cheese::SIZE, &val)}
		} else if field == key::cheese::EXCLUSIVE.as_bytes() {
			if let Some(is) = to_bool(&val) {
				exclusive = is;
			} else {invalid_db_entry(db, key, key::cheese::SIZE, &val)}
		} else if field == key::cheese::ORIGINAL_SIZE.as_bytes() {
			if let Some(s) = to_i32(&val) {
				original_size = Some(s);
			} else {invalid_db_entry(db, key, key::cheese::ORIGINAL_SIZE, &val)}
		} else {
			unrecognized_db_entry(db, key, field, &val)
		}

		} else {mismatch_spec(db, file!(), line!());}
		} else {mismatch_spec(db, file!(), line!());}
	}
	} else {mismatch_spec(db, file!(), line!());}
	return CheeseData{
		str_mem: mem,
        image: image,
        radical_image: radical_image,
        time_min: time_min,
        time_max: time_max,
        size: size,
        original_size: match original_size {
			Some(s) => s,
			None => size,
		},
		gems: gems,
        squirrel_mult: squirrel_mult,
        silent: silent,
        exclusive: exclusive,
	};
}

pub fn get_cheese_from_id<'a>(db: &mut LayerData, trans_mem: &mut Vec<u8>, id: &[u8]) -> Result<CheeseData<'a>, ()> {
	let s = trans_mem.len();
	trans_mem.extend(key::prefix::CHEESE.as_bytes());
	trans_mem.extend(id);
	let e = trans_mem.len();
	let key = &trans_mem[s..e];

	db.pipe.cmd("HGETALL");
	db.pipe.arg(key);

	let val = com::auto_retry_flush_pipe(db)?;
	return Ok(_get_cheese_from_val(db, val, true, key));
}
pub fn get_cheese_from_uuid<'a>(db: &mut LayerData, trans_mem: &mut Vec<u8>, uuid: u64) -> Result<CheeseData<'a>, ()> {
	let key = push_u64_prefix(trans_mem, key::prefix::CHEESE_DATA, uuid);

	db.pipe.cmd("HGETALL");
	db.pipe.arg(key);

	let val = com::auto_retry_flush_pipe(db)?;
	return Ok(_get_cheese_from_val(db, val, true, key));
}


pub fn add_new_cheese(db: &mut LayerData, trans_mem: &mut Vec<u8>, cheese: &CheeseData) -> Result<u64, ()> {
	db.pipe.incr(key::CHEESE_UID_MAX, 1i32);
	let val = com::auto_retry_flush_pipe(db)?;

	let uuid = get_u64_from_val_or_panic(db, &val);
	let cheese_key = push_u64_prefix(trans_mem, key::prefix::CHEESE, uuid);


	db.pipe.cmd("HMSET").ignore();
	db.pipe.arg(cheese_key);

	com::save_cheese_to_pipe(db, cheese);

	db.pipe.expire(cheese_key, CHEESE_TTL as usize).ignore();

	return Ok(uuid);
}
pub fn set_cheese(db: &mut LayerData, trans_mem: &mut Vec<u8>, uuid: u64, cheese: &CheeseData) {
	let cheese_key = push_u64_prefix(trans_mem, key::prefix::CHEESE, uuid);

	db.pipe.cmd("HMSET").ignore();
	db.pipe.arg(cheese_key);

	com::save_cheese_to_pipe(db, cheese);
}



pub fn get_user_from_uuid<'a>(db: &mut LayerData, trans_mem: &mut Vec<u8>, uuid: u64) -> Result<UserData<'a>, LayerError> {
	let key = push_u64_prefix(trans_mem, key::prefix::USER, uuid);
	db.pipe.cmd("HGETALL");
	db.pipe.arg(key);


	if let Ok(val) = com::auto_retry_flush_pipe(db) {
	if let redis::Value::Bulk(vals) = val {

	let mut str_mem = Vec::<u8>::new();
	let mut screen_name_opt = None;
	let mut body_opt = None;
	let mut hat = None;
	let mut cheese = 0;
	let mut gems = 0;

	if vals.len()%2 == 1 {mismatch_spec(db, file!(), line!());}
	if vals.len() == 0 {return Err(LayerError::NotFound)}

	for i2 in 0..vals.len()/2 {
		let i = i2*2;
		if let redis::Value::Data(field) = &vals[i] {
		if let redis::Value::Data(val) = &vals[i + 1] {

		if field == key::user::SCREEN_NAME.as_bytes() {
			screen_name_opt = Some(push_u8s(&mut str_mem, &val[..]));
		} else if field == key::user::HAT.as_bytes() {
			if !is_str_null(val) {
				hat = Some(push_u8s(&mut str_mem, &val[..]));
			}
		} else if field == key::user::BODY.as_bytes() {
			body_opt = Some(push_u8s(&mut str_mem, &val[..]));
		} else if field == key::user::CHEESE.as_bytes() {
			if let Some(v) = to_i64(val) {
				cheese = v;
			} else {invalid_db_entry(db, key, key::user::CHEESE, &val)}
		} else if field == key::user::GEMS.as_bytes() {
			if let Some(v) = to_i64(val) {
				gems = v;
			} else {invalid_db_entry(db, key, key::user::GEMS, &val)}
		} else {
			unrecognized_db_entry(db, key, field, &val)
		}
		} else {mismatch_spec(db, file!(), line!());}
		} else {mismatch_spec(db, file!(), line!());}
	}

	let screen_name = if let Some(name) = screen_name_opt {
		name
	} else {
		missing_db_entry(db, key, key::user::SCREEN_NAME);
		default::SCREEN_NAME.as_bytes()
	};
	let body = if let Some(body) = body_opt {
		body
	} else {
		missing_db_entry(db, key, key::user::BODY);
		default::MOUSE_BODY.as_bytes()
	};


	return Ok(UserData{
		str_mem: str_mem,
		screen_name: screen_name,
		hat: hat,
		body: body,
		cheese: cheese,
		gems: gems,
	});


	} else {mismatch_spec(db, file!(), line!());}
	}
	return Err(LayerError::Fatal);
}
pub fn get_or_create_user_from_id<'a>(db: &mut LayerData, trans_mem: &mut Vec<u8>, id: &[u8], screen_name: &[u8]) -> Result<(u64, UserData<'a>), ()> {
	db.pipe.hget(key::USERUUID_HM, id);
	let uuid = match com::auto_retry_flush_pipe(db)? {
		redis::Value::Data(data) => match to_u64(&data) {
			Some(uuid) => match get_user_from_uuid(db, trans_mem, uuid) {
				Ok(user) => return Ok((uuid, user)),
				Err(LayerError::NotFound) => {
					invalid_db_entry_attempt_to_repair(db, key::USERUUID_HM.as_bytes(), id, &data);

					uuid
				},
				Err(LayerError::Fatal) => return Err(()),
			},
			None => {
				invalid_db_entry_attempt_to_repair(db, key::USERUUID_HM.as_bytes(), id, &data);
				db.pipe.incr(key::MAXUUID, 1i32);
				let val = &com::auto_retry_flush_pipe(db)?;

				get_u64_from_val_or_panic(db, val)
			},
		},
		_ => {
			//we are assuming that the user does not exist
			db.pipe.incr(key::MAXUUID, 1i32);
			let val = &com::auto_retry_flush_pipe(db)?;

			get_u64_from_val_or_panic(db, val)
		},
	};
	let user = generate_default_user(screen_name);
	let user_key = push_u64_prefix(trans_mem, key::prefix::USER, uuid);

	db.pipe.cmd("HSET").ignore();
	db.pipe.arg(key::USERUUID_HM).arg(id).arg(uuid);

	db.pipe.cmd("HMSET").ignore();
	db.pipe.arg(user_key);

	db.pipe.arg(key::user::SCREEN_NAME).arg(user.screen_name);
	if let Some(hat) = user.hat {
		db.pipe.arg(key::user::HAT).arg(hat);
	}
	db.pipe.arg(key::user::BODY).arg(user.body);
	db.pipe.arg(key::user::CHEESE).arg(user.cheese);
	db.pipe.arg(key::user::GEMS).arg(user.gems);

	db.pipe.hset(key::USERUUID_HM, id, uuid).ignore();

	return Ok((uuid, user))
}
pub fn get_user_from_id<'a>(db: &mut LayerData, trans_mem: &mut Vec<u8>, id: &[u8]) -> Result<(u64, UserData<'a>), LayerError> {
	db.pipe.hget(key::USERUUID_HM, id);
	return match com::auto_retry_flush_pipe(db) {
		Ok(redis::Value::Data(data)) => match to_u64(&data) {
			Some(uuid) => match get_user_from_uuid(db, trans_mem, uuid) {
				Ok(user) => Ok((uuid, user)),
				Err(LayerError::NotFound) => Err(LayerError::NotFound),
				Err(LayerError::Fatal) => Err(LayerError::Fatal),
			},
			None => {
				invalid_db_uuid(db, key::USERUUID_HM, id, &data);
				Err(LayerError::NotFound)
			},
		},
		Err(()) => Err(LayerError::Fatal),
		_ => Err(LayerError::NotFound),
	};
}

pub fn incr_user_currency(db: &mut LayerData, trans_mem: &mut Vec<u8>, uuid: u64, user: &UserData, currency: Currency, delta: i32) {
	if delta != 0 {
		let key = push_u64_prefix(trans_mem, key::prefix::USER, uuid);
		let field = match currency {
			Currency::CHEESE => key::user::CHEESE,
			Currency::GEMS => key::user::GEMS,
		};
		db.pipe.hincr(key, field, delta).ignore();
		if let Currency::CHEESE = currency {
			//increment rankings
			let new_cheese = user.cheese + delta as i64;
			db.pipe.zadd(key::GLOBAL_CHEESE_RANKING, key, new_cheese).ignore();
			db.pipe.zincr(key::DAILY_CHEESE_RANKING, key, delta).ignore();
		} else if let Currency::GEMS = currency {
			//increment rankings
			let new_gems = user.gems + delta as i64;
			db.pipe.zadd(key::GLOBAL_GEMS_RANKING, key, new_gems).ignore();
		}
	}
}

pub fn get_last_reset_unix(db: &mut LayerData, trans_mem: &mut Vec<u8>) -> Result<i64, LayerError> {
	db.pipe.get(key::DAILY_RESET_TIMESTAMP_UNIX);
	return match com::auto_retry_flush_pipe(db) {
		Ok(v) => match v {
			redis::Value::Data(data) => match to_u64(&data[..]) {
				Some(timestamp) => Ok(timestamp as i64),
				None => {
					invalid_db_key(db, key::DAILY_RESET_TIMESTAMP_UNIX, &data);
					Err(LayerError::NotFound)
				},
			},
			redis::Value::Nil => Err(LayerError::NotFound),
			_ => {
				mismatch_spec(db, file!(), line!());
				Err(LayerError::Fatal)
			}
		},
		Err(()) => Err(LayerError::Fatal),
	}
}
pub fn daily_reset(db: &mut LayerData, trans_mem: &mut Vec<u8>, reset_timestamp_unix: i64) {
	db.pipe.zremrangebyrank(key::DAILY_CHEESE_RANKING, 0, -1).ignore();
	db.pipe.set(key::DAILY_RESET_TIMESTAMP_UNIX, reset_timestamp_unix).ignore();
}

pub fn flush(db: &mut LayerData, trans_mem: &mut Vec<u8>) -> Result<(), ()> {
	com::auto_retry_flush_pipe(db)?;
	return Ok(());
}
