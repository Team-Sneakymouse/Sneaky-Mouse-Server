//By Mami

pub const REDIS_STREAM_READ_COUNT : usize = 55;
pub const REDIS_STREAM_TIMEOUT_MS : usize = 2000;
pub const REDIS_TIME_BETWEEN_RETRY_CON : u64 = 5;
pub const REDIS_RETRY_CON_MAX_ATTEMPTS : i32 = 5;

pub const REDIS_LAST_ID_PREFIX : &str = "last_id";
pub const REDIS_LAST_ID_DEFAULT : &str = "0-0";//either $ or 0-0 are acceptable here

pub const KEY_USERUUID_HM : &str = "uuid";
pub const KEY_MAXUUID : &str = "uuid-max";

pub const DEBUG_FLOOD_ALL_STREAMS : bool = true;//sets all streams to 0-0 as their last id and disables last id saving
