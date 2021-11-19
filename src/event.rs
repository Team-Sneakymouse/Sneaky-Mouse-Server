//By Mami

use rand_pcg::*;
use crate::config::*;
use crate::util;

pub struct SneakyMouseServer<'a> {
	pub redis_con : redis::Connection,
	pub redis_address : &'a str,
	pub rng : Pcg64,
	pub trans_mem : Vec<u8>,
	// pub xadd_trans_mem : Vec<(&'a[u8], &'a[u8])>,
}

pub fn get_event_list() -> Vec<&'static [u8]> {
	//Normally I wouldn't have a function return an allocation, but this is only called once by main to configure itself
	vec![//this list is considered unordered
		EVENT_DEBUG_CONSOLE,
		EVENT_CHEESE_REQUEST,
		EVENT_CHEESE_SPAWN,
	]
}

pub fn server_event_received(server_state : &mut SneakyMouseServer, event_name : &[u8], event_uid : &[u8], keys : &[&[u8]], vals : &[&[u8]]) -> Option<bool> {
	server_state.trans_mem.clear();
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
				server_state.trans_mem.extend(KEY_USER_PREFIX.as_bytes());
				util::push_u64(&mut server_state.trans_mem, uuid);
				cmd.arg(&server_state.trans_mem).arg(KEY_MOUSE_BODY).arg(KEY_MOUSE_HAT);

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
							let mut cmdset = redis::Cmd::hset(&server_state.trans_mem, KEY_MOUSE_BODY, VAL_MOUSE_BODY_DEFAULT);
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
				if let Some(cheese_id) = util::find_val(FIELD_CHEESE_ID, keys, vals) {
					let time_min = util::find_u64_field_or_default(FIELD_TIME_MIN, 0, server_state, event_name, event_uid, keys, vals);
					let time_max = util::find_u64_field_or_default(FIELD_TIME_MAX, 0, server_state, event_name, event_uid, keys, vals);
					let size = util::find_u64_field_or_default(FIELD_SIZE, 0, server_state, event_name, event_uid, keys, vals);
					let mut exclusive : bool = false;

					if let Some(raw) = util::find_val(FIELD_EXCLUSIVE, keys, vals) {
						if let Some(i) = util::to_bool(raw) {
							exclusive = i;
						} else {util::invalid_value(server_state, event_name, event_uid, keys, vals, FIELD_EXCLUSIVE);}
					}


				}

			} else {util::missing_field(server_state, event_name, event_uid, keys, vals, FIELD_ROOM_UID);}
		}
		_ => {
			panic!("fatal error: we received an unrecognized event from redis, '{}', please check the events list\n", String::from_utf8_lossy(event_name));
		}
	}
	Some(true)
}

