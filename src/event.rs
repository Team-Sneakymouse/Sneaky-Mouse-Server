//By Mami
use rand_pcg::*;
use crate::config::*;
use crate::util;
use rand::{Rng, RngCore, SeedableRng};


pub struct SneakyMouseServer<'a> {
	pub redis_con : redis::Connection,
	pub redis_address : &'a str,
	pub rng : Pcg64,
	pub cur_time : f64,
	pub next_timeout : f64,
	pub cheese_timeouts : Vec<f64>,
	pub cheese_uids : Vec<u64>,
	pub cheese_rooms : Vec<Vec<u8>>,//I don't like this
	pub cheese_ids : Vec<&'static [u8]>,
	// pub xadd_trans_mem : Vec<(&'a[u8], &'a[u8])>,
}

pub fn get_event_list() -> Vec<&'static [u8]> {
	//Normally I wouldn't have a function return an allocation, but this is only called once by main to configure itself
	vec![//this list is considered unordered
		EVENT_SHUTDOWN,
		EVENT_DEBUG_CONSOLE,
		EVENT_CHEESE_REQUEST,
		EVENT_CHEESE_SPAWN,
		EVENT_CHEESE_COLLECTED,
	]
}

pub fn server_event_received(server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]], trans_mem : &mut Vec<u8>) -> Option<bool> {
	trans_mem.clear();
	match event_name {
		EVENT_DEBUG_CONSOLE => {
			print!("debug event: {} <", String::from_utf8_lossy(event_uid));
			for (i, key) in keys.iter().enumerate() {
				print!("{}:{}", String::from_utf8_lossy(key), String::from_utf8_lossy(vals[i]));
				if i+1 == keys.len() {
					print!("> {}\n", server_state.redis_address);
				} else {
					print!(", ");
				}
			}
		}
		EVENT_CHEESE_REQUEST => {
			if let Some(useruid_raw) = util::find_val(FIELD_USER_UID, keys, vals) {
			if let Some(username) = util::find_val(FIELD_USER_NAME, keys, vals) {
			if let Some(room) = util::find_val(FIELD_ROOM_UID, keys, vals) {

				let uuid = util::lookup_user_uuid(server_state, useruid_raw)?;

				let mut cmd = redis::cmd("HGET");
				trans_mem.extend(KEY_USER_PREFIX.as_bytes());
				util::push_u64(trans_mem, uuid);
				cmd.arg(&trans_mem[..]).arg(KEY_MOUSE_BODY).arg(KEY_MOUSE_HAT);

				if let redis::Value::Bulk(values) = util::auto_retry_cmd(server_state, &mut cmd)? {
					let mut cmdxadd = redis::cmd("XADD");
					cmdxadd.arg(EVENT_CHEESE_COLLECT).arg("*");
					cmdxadd.arg(FIELD_ROOM_UID).arg(room);
					cmdxadd.arg(FIELD_USER_UID).arg(useruid_raw);
					cmdxadd.arg(FIELD_USER_NAME).arg(username);

					match &values[0] {
						redis::Value::Data(body) => {
							cmdxadd.arg(FIELD_MOUSE_BODY).arg(body);
						}
						redis::Value::Nil => {//set the default body
							cmdxadd.arg(FIELD_MOUSE_BODY).arg(VAL_MOUSE_BODY_DEFAULT);
							let mut cmdset = redis::Cmd::hset(&trans_mem[..], KEY_MOUSE_BODY, VAL_MOUSE_BODY_DEFAULT);
							let _ : redis::Value = util::auto_retry_cmd(server_state, &mut cmdset)?;
						}
						_ => util::mismatch_spec(server_state, file!(), line!())
					}
					match &values[1] {
						redis::Value::Data(hat) => {
							cmdxadd.arg(FIELD_MOUSE_HAT).arg(hat);
						}
						redis::Value::Nil => (),
						_ => util::mismatch_spec(server_state, file!(), line!())
					}
					let _ : redis::Value = util::auto_retry_cmd(server_state, &mut cmdxadd)?;
				} else {
					util::mismatch_spec(server_state, file!(), line!());
				}
			} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_ROOM_UID);}
			} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_USER_NAME);}
			} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_USER_UID);}
		}
		EVENT_CHEESE_SPAWN => {
			if let Some(room) = util::find_val(FIELD_ROOM_UID, keys, vals) {
			if let Some(cheese_id_raw) = util::find_val(FIELD_CHEESE_ID, keys, vals) {

			let cheese_id : &'static [u8] = match cheese_id_raw {
				_ => CHEESE_DEFAULT,
			};

			let mut cheese_uid = u64::MAX;
			let mut cheese_i = 0;
			for (i, server_room) in server_state.cheese_rooms.iter().enumerate() {
				if server_room == room {
					if server_state.cheese_ids[i] == cheese_id {
						cheese_uid = server_state.cheese_uids[i];
						cheese_i = i;
					}
					break;
				}
			}
			let time_max = util::find_u64_field(FIELD_TIME_MAX, server_state, event_name, event_uid, keys, vals).unwrap_or(u64::MAX);
			let time_min = util::find_u64_field(FIELD_TIME_MIN, server_state, event_name, event_uid, keys, vals).unwrap_or(u64::MAX);

			if cheese_uid == u64::MAX {//set cheese to room
				let size = util::find_u64_field(FIELD_SIZE, server_state, event_name, event_uid, keys, vals).unwrap_or(1);
				let mut exclusive : bool = false;

				if let Some(raw) = util::find_val(FIELD_EXCLUSIVE, keys, vals) {
					if let Some(i) = util::to_bool(raw) {
						exclusive = i;
					} else {util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_EXCLUSIVE);}
				}

				let mut time_out;
				if time_min == u64::MAX || time_max == time_min {
					time_out = time_max;
				} else if time_max == u64::MAX  {
					time_out = time_min;
				} else if time_max < time_min {
					util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_TIME_MIN);
					time_out = time_max;
				} else {
					time_out = server_state.rng.gen_range(time_min..=time_max);
				}
				time_out = u64::min(time_out, (VAL_CHEESE_MAX_TTL_S*1000) as u64);

				let (image, silent) = util::get_cheese_data(server_state, cheese_id, trans_mem)?;

				let mut cmdgetuid = redis::Cmd::incr(KEY_CHEESE_UID_MAX, 1i32);
				let val = &util::auto_retry_cmd(server_state, &mut cmdgetuid)?;
				cheese_uid = util::get_u64_from_val_or_panic(server_state, val);

				let mut cmdhset = redis::cmd("HMSET");
				trans_mem.extend(KEY_CHEESE_PREFIX.as_bytes());
				util::push_u64(trans_mem, cheese_uid);
				cmdhset.arg(&trans_mem[..]);

				let mut cmdex = redis::Cmd::expire(&trans_mem[..], VAL_CHEESE_MAX_TTL_S);
				trans_mem.clear();

				let mut cmdxadd = redis::cmd("XADD");
				cmdxadd.arg(EVENT_CHEESE_UPDATE).arg("*");
				cmdxadd.arg(FIELD_CHEESE_UID).arg(cheese_uid);

				cmdhset.arg(FIELD_ROOM_UID).arg(room);
				cmdxadd.arg(FIELD_ROOM_UID).arg(room);

				cmdxadd.arg(FIELD_IMAGE).arg(image);
				if size > 1 {
					cmdhset.arg(FIELD_SIZE).arg(size);
					cmdxadd.arg(FIELD_SIZE).arg(size);
				}
				if silent {
					cmdhset.arg(FIELD_SILENT).arg(silent);
					cmdxadd.arg(FIELD_SILENT).arg(silent);
				}
				if exclusive {
					cmdhset.arg(FIELD_EXCLUSIVE).arg(exclusive);
					cmdxadd.arg(FIELD_EXCLUSIVE).arg(exclusive);
				}

				server_state.cheese_timeouts.push((time_out as f64)/1000.0);
				server_state.cheese_uids.push(cheese_uid);
				server_state.cheese_rooms.push(Vec::from(room));
				server_state.cheese_ids.push(&cheese_id);

				util::auto_retry_cmd(server_state, &mut cmdhset)?;
				util::auto_retry_cmd(server_state, &mut cmdex)?;
				util::auto_retry_cmd(server_state, &mut cmdxadd)?;
			} else {//augment current cheese
				let size = util::find_u64_field(FIELD_SIZE, server_state, event_name, event_uid, keys, vals);
				let mut exclusive : Option<bool> = None;

				if let Some(raw) = util::find_val(FIELD_EXCLUSIVE, keys, vals) {
					if let Some(i) = util::to_bool(raw) {
						exclusive = Some(i);
					} else {util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_EXCLUSIVE);}
				}

				let (image, silent) = util::get_cheese_data(server_state, cheese_id, trans_mem)?;

				let mut cmdxadd = redis::cmd("XADD");
				cmdxadd.arg(EVENT_CHEESE_UPDATE).arg("*");
				cmdxadd.arg(FIELD_CHEESE_UID).arg(cheese_uid);

				let mut cmdhget = redis::cmd("HMGET");
				let mut cmdhset = redis::cmd("HMSET");
				trans_mem.extend(KEY_CHEESE_PREFIX.as_bytes());
				util::push_u64(trans_mem, cheese_uid);
				cmdhget.arg(&trans_mem[..]);
				cmdhset.arg(&trans_mem[..]);
				trans_mem.clear();
				let flag = false;
				cmdhget.arg(FIELD_SIZE);
				cmdhget.arg(FIELD_SILENT);
				if let None = exclusive {
					cmdhget.arg(FIELD_EXCLUSIVE);
				}

				match util::auto_retry_cmd(server_state, &mut cmdhget)? {
					redis::Value::Bulk(values) => {
						if values.len() != 3 {util::mismatch_spec(server_state, file!(), line!());}

						if let Some(v) = size {
							let new_size = v;
							if let redis::Value::Data(size_raw) = values[0] {
								if let Some(pre_size) = util::to_u64(&size_raw) {
									new_size += pre_size;
								} else {util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_SIZE);}
							}
							cmdxadd.arg(FIELD_SIZE).arg(new_size);
							cmdhset.arg(FIELD_SIZE).arg(new_size);
							flag = true;
						} else {
							if let redis::Value::Data(size_raw) = values[0] {
								cmdxadd.arg(FIELD_SIZE).arg(size_raw);
							}
						}
						if let redis::Value::Data(exclusive_raw) = values[1] {
							cmdxadd.arg(FIELD_EXCLUSIVE).arg(exclusive_raw);
						}
						if let Some(v) = exclusive {
							cmdxadd.arg(FIELD_EXCLUSIVE).arg(v);
							cmdhset.arg(FIELD_SIZE).arg(v);
							flag = true;
						} else {
							if let redis::Value::Data(silent_raw) = values[2] {
								cmdxadd.arg(FIELD_SILENT).arg(silent_raw);
							} else if silent {
								cmdxadd.arg(FIELD_SILENT).arg(silent);
							}
						}
					}
					_ => {util::mismatch_spec(server_state, file!(), line!());}
				}

				let mut time_out;
				if time_min == u64::MAX || time_max == time_min {
					time_out = time_max;
				} else if time_max == u64::MAX  {
					time_out = time_min;
				} else if time_max < time_min {
					util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_TIME_MIN);
					time_out = time_max;
				} else {
					time_out = server_state.rng.gen_range(time_min..=time_max);
				}
				if time_out != u64::MAX {
					time_out = u64::min(time_out, (VAL_CHEESE_MAX_TTL_S*1000) as u64);
					server_state.cheese_timeouts[cheese_i] = server_state.cur_time + (time_out as f64)/1000.0;
				}

				if flag {
					util::auto_retry_cmd(server_state, &mut cmdhset)?;
				}
				util::auto_retry_cmd(server_state, &mut cmdxadd)?;

				}
				} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_CHEESE_ID);}
			} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_ROOM_UID);}
		}
		EVENT_CHEESE_COLLECTED => {

		}
		EVENT_SHUTDOWN => {
			print!("shutdown request acknowledged, closing the server\n");

			for room in server_state.cheese_rooms {

				let mut cmdxadd = redis::cmd("XADD");
				cmdxadd.arg(EVENT_CHEESE_UPDATE).arg("*");
				cmdxadd.arg(FIELD_ROOM_UID).arg(room);

				util::auto_retry_cmd(server_state, &mut cmdxadd)?;
			}
			return None
		}
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, '{}', please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Some(true)
}


pub fn server_update(server_state : &mut SneakyMouseServer, trans_mem : &mut Vec<u8>, delta : f64) -> Option<f64> {
	server_state.cur_time += delta;

	let i = 0;
	let next_timeout = REDIS_STREAM_TIMEOUT_MAX;
	while i < server_state.cheese_timeouts.len() {
		if server_state.cheese_timeouts[i] <= server_state.cur_time {
			server_state.cheese_timeouts.swap_remove(i);
			let cheese_uid = server_state.cheese_uids.swap_remove(i);
			let room = server_state.cheese_rooms.swap_remove(i);
			server_state.cheese_ids.swap_remove(i);

			let mut cmdxadd = redis::cmd("XADD");
			cmdxadd.arg(EVENT_CHEESE_UPDATE).arg("*");
			cmdxadd.arg(FIELD_ROOM_UID).arg(room);

			util::auto_retry_cmd(server_state, &mut cmdxadd)?;
		} else {
			next_timeout = f64::min(next_timeout, server_state.cheese_timeouts[i] - server_state.cur_time);
		}
		i += 1;
	}

	Some(next_timeout)
}
