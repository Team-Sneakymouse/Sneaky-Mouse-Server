//By Mami

pub const REDIS_STREAM_READ_COUNT : usize = 55;
pub const REDIS_STREAM_TIMEOUT_MS : usize = 2000;
pub const REDIS_TIME_BETWEEN_RETRY_CON : u64 = 5;
pub const REDIS_RETRY_CON_MAX_ATTEMPTS : i32 = 5;

pub const REDIS_LAST_ID_PREFIX : &str = "last_id";
pub const REDIS_LAST_ID_DEFAULT : &str = "0-0";//either $ or 0-0 are acceptable here

pub const KEY_USERUUID_HM : &str = "uuid";
pub const KEY_MAXUUID : &str = "uuid-max";
pub const KEY_USER_PREFIX : &str = "user:";
pub const KEY_MOUSE_BODY : &str = "mouse:body";
pub const KEY_MOUSE_HAT : &str = "mouse:hat";

pub const VAL_MOUSE_BODY_DEFAULT : &str = "danipls";

pub const EVENT_DEBUG_CONSOLE : &[u8] = b"debug:console";
pub const EVENT_DEBUG_ERROR : &[u8] = b"debug:error";
pub const EVENT_CHEESE_REQUEST : &[u8] = b"sm-cheese:request";
pub const EVENT_CHEESE_COLLECT : &[u8] = b"sm-cheese:collect";
pub const EVENT_CHEESE_SPAWN : &[u8] = b"sm-cheese:spawn";

pub const FIELD_MESSAGE : &str = "message";
pub const FIELD_MOUSE_BODY : &str = "body";
pub const FIELD_MOUSE_HAT : &str = "hat";
pub const FIELD_USER_UID : &str = "user-uid";
pub const FIELD_USER_NAME : &str = "user-name";
pub const FIELD_ROOM_UID : &str = "room-uid";


pub const DEBUG_FLOOD_ALL_STREAMS : bool = true;//sets all streams to 0-0 as their last id and disables last id saving
