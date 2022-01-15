//By Mami
use crate::com;
use crate::db;
use crate::config::*;
use crate::util;
use crate::util::*;
use rand_pcg::*;
use rand::{Rng, RngCore};

use dvz::*;

//NOTE: NONE OF THIS CODE DOES INPUT VALIDATION, FOR THE LOVE OF GOD MAKE SURE THE CALLER DOES IT

fn rand_class(rng: &mut Pcg64) -> u32 {
	return rng.gen_range(0..=class::MAX);
}
fn defeat(server: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room: &mut Room) {
	//event defeat
	if room.dvz_defenders.len() > 0 {
		let survivor_i: usize = server.rng.gen_range(0..room.dvz_defenders.len());
		let survivor_uuid = room.dvz_defenders[survivor_i];
		let survivor_class = room.dvz_defender_classes[survivor_i];

		let survivor = match db::get_user_from_uuid(&mut server.db, trans_mem, survivor_uuid) {
			Ok(v) => v,
			Err(LayerError::NotFound) => {
				com::?;
				return ;
			},
			Err(LayerError::Fatal) => return Err(()),
		};
		survivor.dvz_powers[survivor_class - 1] += 1;
		com::dvz_defeat()?;

		for (i, defender) in room.dvz_defenders.iter().enumerate() {
			if i != survivor_i {
				db::kill_mouse(defender)?;
			}
		}
	} else {

	}
	room.dvz_state = state::INACTIVE;
}

pub fn start(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8], raid_size: u32) {
	let room = util::get_room_from_id(server_state, trans_mem, room_id);
	//get raid type
	let class = rand_class(&mut server_state.rng);
	//get defender main class and min number
	let repel_amount = i32::max(1, server_state.rng.gen_range((-3)..=(3)) + (raid_size as i32)) as u32;
	//flavor text?
	match room.dvz_state {
		state::INACTIVE => {
			room.dvz_state = state::READY_UP;
			room.dvz_raid_sizes = [0; dvz::class::TOTAL];
			room.dvz_raid_sizes[class] = repel_amount;
			room.dvz_defenders.clear();
			room.dvz_defender_classes.clear();
			room.dvz_defender_class_totals = [0; dvz::class::TOTAL];
			room.dvz_money_donated = 0;
			room.dvz_events_total = 0;
			com::dvz_raid(&mut server_state.db, trans_mem, );
		},
		_ => {
			com::dvz_reinforcements(&mut server_state.db, trans_mem, );
			room.dvz_raid_sizes[class] += repel_amount;
		},
	}
}

pub fn march_defender(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8], uuid: u64, user: &UserData, class_id: &[u8]) {
	let room: Room = util::get_room_from_id(server_state, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}
	for cur_uuid in room.dvz_defenders {
		if cur_uuid == uuid {
			return;
		}
	}
	//validate and get class
	let (class, outfit) = match class_id {
		dvz::DEFENDER_CLASS_A => (class::A, DEFENDER_OUTFIT_A),
		dvz::DEFENDER_CLASS_B => (class::B, DEFENDER_OUTFIT_B),
		dvz::DEFENDER_CLASS_C => (class::C, DEFENDER_OUTFIT_C),
		dvz::DEFENDER_CLASS_D => (class::D, DEFENDER_OUTFIT_D),
		dvz::DEFENDER_CLASS_E => (class::E, DEFENDER_OUTFIT_E),
		_ => {
			return;
		}
	};
	//queue mouse anim + class equip anim
	com::dvz_queue_defender(&mut server_state.db, trans_mem, room_id, user.screen_name, user, outfit);
}

pub fn add_defender(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8], uuid: u64, user: &UserData, class_id: &[u8]) {
	let room: Room = util::get_room_from_id(server_state, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}
	for cur_uuid in room.dvz_defenders {
		if cur_uuid == uuid {
			return;
		}
	}
	//validate and get class
	let (class, outfit) = match class_id {
		dvz::DEFENDER_CLASS_A => (class::A, DEFENDER_OUTFIT_A),
		dvz::DEFENDER_CLASS_B => (class::B, DEFENDER_OUTFIT_B),
		dvz::DEFENDER_CLASS_C => (class::C, DEFENDER_OUTFIT_C),
		dvz::DEFENDER_CLASS_D => (class::D, DEFENDER_OUTFIT_D),
		dvz::DEFENDER_CLASS_E => (class::E, DEFENDER_OUTFIT_E),
		_ => {
			return;
		}
	};
	//queue mouse anim + class equip anim
	//save mouse data for the game
	room.dvz_defenders.push(uuid);
	room.dvz_defender_classes.push(class as u8);

	com::dvz_defenders_update(&mut server_state.db, trans_mem, room_id, );
}

pub fn donation_event(server_state: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8], amount: u64) {
	let room = util::get_room_from_id(server_state, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}
	//NOTE: amount is expected to be in units of usd cents
	//TODO: what happens if a user donates <$1?
	//validate and get donation request
	let mut repel_delta = (amount/100) as i32;
	//add to raid size or add different raid class or spawn event
	let repel_class: u32 = {
		const OFF_CLASS_DONATION_REDUCTION: i32 = 5;
		let do_off_class = repel_delta >= OFF_CLASS_DONATION_REDUCTION && server_state.rng.gen_range(0..3) == 0i32;

		if do_off_class {
			let d = repel_delta%OFF_CLASS_DONATION_REDUCTION;
			repel_delta = repel_delta/OFF_CLASS_DONATION_REDUCTION + ((server_state.rng.gen_range(0..OFF_CLASS_DONATION_REDUCTION) < d) as i32);

			rand_class(&mut server_state.rng)
		} else {
			let mut dvz_raid_classes: [u32; dvz::class::TOTAL];
			let mut dvz_raids_total = 0;
			for i in 0..dvz::class::TOTAL {
				if room.dvz_raid_sizes[i] > 0 {
					dvz_raid_classes[dvz_raids_total] = i as u32;
					dvz_raids_total += 1;
				}
			}

			if dvz_raids_total > 0 {
				dvz_raid_classes[server_state.rng.gen_range(0..dvz_raids_total)]
			} else {
				//there is no raid currently?
				rand_class(&mut server_state.rng)
			}
		}
	};

	room.dvz_raid_sizes[repel_class] += repel_delta;
	room.dvz_money_donated += amount;

	com::dvz_raid_update(&mut server_state.db, trans_mem, room_id, );
}

pub fn defender_action(server: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8], uuid: u64, user: &UserData, action: &[u8]) {
	let room: &mut Room = util::get_room_from_id(server, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}
	//validate and get action
	let class = 0;
	let mut is_approved = false;
	for (i, cur_uuid) in room.dvz_defenders.iter().enumerate() {
		if *cur_uuid == uuid {
			is_approved = true;
			class = room.dvz_defender_classes[i];
			break;
		}
	}
	if !is_approved {
		return
	}
	for cur_uuid in room.dvz_event_defenders {
		if cur_uuid == uuid {
			return;
		}
	}
	//validate and get class
	let action_uid = match action {
		dvz::DEFENDER_CLASS_A => class::A,
		dvz::DEFENDER_CLASS_B => class::B,
		dvz::DEFENDER_CLASS_C => class::C,
		dvz::DEFENDER_CLASS_D => class::D,
		dvz::DEFENDER_CLASS_E => class::E,
		_ => {
			return;
		}
	};
	//save mouse data for the game
	room.dvz_event_defenders.push(uuid);
	room.dvz_event_defenders_attacks.push(action_uid as u8);

	// com;
}

pub fn event_end(server: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8]) {
	let room: &mut Room = util::get_room_from_id(server, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}

	{//check what to do next
		let do_event = {
			let events_to_spawn = f64::sqrt((room.dvz_money_donated as f64)/20_00.0);
			let events_pending = events_to_spawn - room.dvz_events_total as f64;
			if events_pending > 1.0 {
				true
			} else if events_pending > 0.0 {
				if rand_f64(&mut server.rng) < events_pending {
					true
				} else {
					false
				}
			} else {
				false
			}
		};

		if do_event {
			room.dvz_events_total += 1;
			room.dvz_event_type = rand_class(&mut server.rng);
			room.dvz_event_death_roll = rand_d20(&mut server.rng);
			room.dvz_event_defenders_needed = server.rng.gen_range(1..=5);
			room.dvz_event_defenders.clear();
			room.dvz_event_defenders_attacks.clear();
			com::dvz_start_event(&mut server.db, trans_mem, room_id, room.dvz_event_type, room.dvz_event_death_roll, room.dvz_event_defenders_needed);
		} else {
			let mut deficit = 0;
			let mut classes_missed_len = 0;
			let mut classes_missed = [0; dvz::class::TOTAL];
			for (class, raid_size) in room.dvz_raid_sizes.iter().enumerate() {
				let defenders_total = room.dvz_defender_class_totals[class];
				if *raid_size > defenders_total {
					deficit += (*raid_size - defenders_total);
					classes_missed[classes_missed_len] = class;
					classes_missed_len += 1;
				}
			}
			if classes_missed_len == 0 {
				//victory
				for (i, defender_uuid) in room.dvz_defenders.iter().enumerate() {
					let class = room.dvz_defender_classes[i];
					let defender = match db::get_user_from_uuid(&mut server.db, trans_mem, *defender_uuid) {
						Ok(v) => v,
						Err(LayerError::NotFound) => {
							com::?;
							return ;
						},
						Err(LayerError::Fatal) => return Err(()),
					};
					defender.dvz_powers[class] += 1;
					db::
				}
			} else {
				//defeat
				defeat(server, trans_mem, room);
			}
		}
	}
}

pub fn event_start(server: &mut SneakyMouseServer, trans_mem: &mut Vec<u8>, room_id: &[u8]) -> Result<(), ()> {
	//determine outcome of event
	let room: Room = util::get_room_from_id(server, trans_mem, room_id);
	match room.dvz_state {
		state::INACTIVE => {
			return;
		},
		_ => {},
	}

	if room.dvz_event_defenders.len() as u32 >= room.dvz_event_defenders_needed {
		assert!(room.dvz_event_defenders_needed <= 5);
		let mut chosen: [usize; 5];//here
		for i in 0..(room.dvz_event_defenders_needed as usize) {
			loop {
				chosen[i] = server.rng.gen_range(0..room.dvz_event_defenders.len());
				let is_unique = true;
				for j in 0..i {
					if chosen[i] == chosen[j] {
						is_unique = false;
						break;
					}
				}
				if is_unique {
					break;
				}
			}
		}

		//event victory
		for i in 0..(room.dvz_event_defenders_needed as usize) {
			//decide who lives
			let defender_uuid = room.dvz_event_defenders[chosen[i]];
			let defender = match db::get_user_from_uuid(&mut server.db, trans_mem, defender_uuid) {
				Ok(v) => v,
				Err(LayerError::NotFound) => {
					com::defender_defeated_event()?;
					continue;
				},
				Err(LayerError::Fatal) => return Err(()),
			};
			let power_level = defender.dvz_powers[room.dvz_event_type as usize];

			let has_survived = {
				let r0 = rand_d20(&mut server.rng);
				if power_level == 0 && r0 + 3 > 20 {//beginner's luck survival
					com::defender_defeated_event()?;
					true
				} else if power_level > 0 && r0 + power_level > 20 && r0 > 1 {
					com;
					true
				} else {
					let r1 = rand_d20(&mut server.rng);
					if r1 >= room.dvz_event_death_roll {
						com;
						false
					} else {
						com;
						true
					}
				}
			};
			if has_survived {
				defender.dvz_powers[room.dvz_event_type] += 1;
				db::write_mouse()?;
			} else {
				db::kill_mouse(defender)?;
			}
		}
	} else {

	}
	return Ok(());
}

