//By Mami
use crate::config::*;
use crate::com;
use crate::event;
use redis::FromRedisValue;

pub fn push_u64(mem: &mut Vec<u8>, i: u64) {
	if i >= 10 {
		push_u64(mem, i/10);
	}
	mem.push(((i%10) as u8) + ASCII_0);
}
pub fn push_u64_prefix<'a>(stack: &mut Vec<u8>, prefix: &str, i: u64) -> &'a[u8] {
	//The actual lifetime of the return value is the lifetime of the bytes in stack, which rust's borrow checker struggles to comprehend
	let start = stack.len();
	stack.extend_from_slice(prefix.as_bytes());
	push_u64(stack, i);
	let end = stack.len();
	let ptr = stack.as_ptr();
	return unsafe {
		std::slice::from_raw_parts(ptr.add(start), end - start)
	}
}
pub fn push_u8s<'a>(stack: &mut Vec<u8>, src: &[u8]) -> &'a[u8] {
	//The actual lifetime of the return value is the lifetime of the bytes in stack, which rust's borrow checker struggles to comprehend
	let start = stack.len();
	stack.extend_from_slice(src);
	let end = stack.len();
	let ptr = stack.as_ptr();
	return unsafe {
		std::slice::from_raw_parts(ptr.add(start), end - start)
	}
}

pub fn to_u32(mem: &[u8]) -> Option<u32> {//eats leading 0s
	let mut i: u32 = 0;
	for c in mem {
		if *c >= ASCII_0 && *c <= ASCII_9 {
			i = i.saturating_mul(10);
			i = i.saturating_add((*c - ASCII_0) as u32);
		} else {
			return None;
		}
	}
	return Some(i);
}
pub fn to_u64(mem: &[u8]) -> Option<u64> {//eats leading 0s
	let mut i: u64 = 0;
	for c in mem {
		if *c >= ASCII_0 && *c <= ASCII_9 {
			i = i.saturating_mul(10);
			i = i.saturating_add((*c - ASCII_0) as u64);
		} else {
			return None;
		}
	}
	return Some(i);
}
pub fn to_i32(mem: &[u8]) -> Option<i32> {//eats leading 0s
	let mut i: i32 = 0;
	let s = (mem[0] == ASCII_NEG) as usize;
	for c in &mem[s..] {
		if *c >= ASCII_0 && *c <= ASCII_9 {
			i = i.saturating_mul(10);
			i = i.saturating_add((*c - ASCII_0) as i32);
		} else {
			return None;
		}
	}
	return Some((1 - (s as i32)*2)*i);
}
pub fn to_i64(mem: &[u8]) -> Option<i64> {//eats leading 0s
	let mut i: i64 = 0;
	let s = (mem[0] == ASCII_NEG) as usize;
	for c in &mem[s..] {
		if *c >= ASCII_0 && *c <= ASCII_9 {
			i = i.saturating_mul(10);
			i = i.saturating_add((*c - ASCII_0) as i64);
		} else {
			return None;
		}
	}
	return Some((1 - (s as i64)*2)*i);
}
pub fn to_f32(mem: &[u8]) -> Option<f32> {
	if let Ok(s) = std::str::from_utf8(mem) {
		if let Ok(v) = s.parse::<f32>() {
			return Some(v);
		}
	}
	return None;
}
pub fn to_bool(mem: &[u8]) -> Option<bool> {
	match mem {
		b"true" | b"1" => Some(true),
		b"false" | b"0" => Some(false),
		_ => None,
	}
}
pub fn is_str_null(mem: &[u8]) -> bool {
	return mem == b"" || mem == STR_NULL.as_bytes();
}


pub fn find_field_u8s<'a>(key: &str, keys: &[&[u8]], vals: &[&'a[u8]]) -> Option<&'a[u8]> {
	for (i, cur_key) in keys.iter().enumerate() {
		if &key.as_bytes() == cur_key {
			return Some(vals[i]);
		}
	}
	return None;
}
pub fn find_field_bool(key: &'static str, server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]]) -> Option<bool> {
	match find_field_u8s(key, keys, vals) {
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
pub fn find_field_u32(key: &'static str, server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]]) -> Option<u32> {
	match find_field_u8s(key, keys, vals) {
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
pub fn find_field_u64(key: &'static str, server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]]) -> Option<u64> {
	match find_field_u8s(key, keys, vals) {
		Some(raw) => match to_u64(raw) {
			Some(i) => Some(i),
			None => {
				invalid_value(server_state, event_name, event_uid, keys, vals, key);
				None
			}
		}
		None => None
	}
}
pub fn find_field_i32(key: &'static str, server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]]) -> Option<i32> {
	match find_field_u8s(key, keys, vals) {
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
pub fn find_field_f32(key: &'static str, server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]]) -> Option<f32> {
	match find_field_u8s(key, keys, vals) {
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

pub fn check_user_saturating_currency(user: &UserData, currency: Currency, sat_delta: i32, cancel_delta: i32) -> (i32, bool) {
	//NOTE: sat_delta is the amount being added to the currency, return value is the largest sat_delta possible without setting the user's currency negative, or false if it is not possible to satisfy cancel_delta
	if sat_delta + cancel_delta >= 0 {
		return (sat_delta, true);
	} else {
		let total = match currency {
			Currency::CHEESE => user.cheese,
			Currency::GEMS => user.gems,
		};
		let unconditional_total = total + (cancel_delta as i64);
		if unconditional_total >= 0 {
			let new_total = u64::saturating_sub(unconditional_total as u64, -sat_delta as u64) as i64;
			let new_sat_delta = (new_total - unconditional_total) as i32;
			// proof sketch of correctness:
			// sat_delta == unconditional_total - (-sat_delta) - unconditional_total == new_total - unconditional_total;
			// sat_delta <= unconditional_total -sat_sub- (-sat_delta) - unconditional_total == new_sat_delta;
			// if sat_delta >= 0, sat_delta <= new_sat_delta <= unconditional_total + sat_delta - unconditional_total == sat_delta, which implies new_sat_delta == sat_delta
			return (new_sat_delta, true);
		} else {
			return (0, false);
		}
	}
}
pub fn check_user_has_enough_currency(user: &UserData, currency: Currency, cancel_delta: i32) -> bool {
	//NOTE: cancel_delta is the amount being added to the currency
	let total = match currency {
		Currency::CHEESE => user.cheese,
		Currency::GEMS => user.gems,
	};
	return (cancel_delta >= 0) | (total + (cancel_delta as i64) >= 0);
}



pub fn push_kvs(error: &mut String, keys: &[&[u8]], vals: &[&[u8]]) {
	error.push_str("{{\n");
	for (i, key) in keys.iter().enumerate() {
		error.push_str(&*String::from_utf8_lossy(key));
		error.push_str(":");
		error.push_str(&*String::from_utf8_lossy(vals[i]));
		if i + 1 == keys.len() {
			error.push_str("\n}}");
		} else {
			error.push_str(",\n");
		}
	}
}

pub fn invalid_value(server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]], field: &'static str) {
	let mut error = format!("invalid event error: field '{}' had an incorrect value, the event will still be attempted with default values, name:'{}' id:'{}' contents:", field, String::from_utf8_lossy(event_name), String::from_utf8_lossy(event_uid));
	push_kvs(&mut error, keys, vals);

	print!("{}\n", error);
	com::send_error(&mut server_state.db, &error);
}
pub fn missing_user(server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]], field: &'static str) {
	let mut error = format!("invalid event error: field '{}' does not identify a known user, event is cancelled, name:'{}' id:'{}' contents:", field, String::from_utf8_lossy(event_name), String::from_utf8_lossy(event_uid));
	push_kvs(&mut error, keys, vals);

	print!("{}\n", error);
	com::send_error(&mut server_state.db, &error);
}
pub fn missing_field(server_state: &mut SneakyMouseServer, event_name: &[u8], event_uid: &[u8], keys: &[&[u8]], vals: &[&[u8]], field: &'static str) {
	let mut error = format!("invalid event error: missing critical field '{}', name:'{}' id:'{}' contents:", field, String::from_utf8_lossy(event_name), String::from_utf8_lossy(event_uid));
	push_kvs(&mut error, keys, vals);

	print!("{}\n", error);
	com::send_error(&mut server_state.db, &error);
}
