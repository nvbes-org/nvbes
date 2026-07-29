pub(super) const CLEANUP_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score or tonumber(score) > tonumber(ARGV[4]) then
    return 0
end
local current = redis.call("GET", KEYS[2])
if ARGV[2] == "missing" then
    if current then return 0 end
elseif current ~= ARGV[3] then
    return 0
end
local dead_letter_type = redis.call("TYPE", KEYS[3])
if type(dead_letter_type) == "table" then
    dead_letter_type = dead_letter_type["ok"]
end
if dead_letter_type == "set" then
    redis.call("DEL", KEYS[3])
end
redis.call("ZREM", KEYS[1], ARGV[1])
redis.call("DEL", KEYS[4])
if ARGV[2] == "corrupt" then
    redis.call("DEL", KEYS[2])
    redis.call("ZREMRANGEBYSCORE", KEYS[3], "-inf", ARGV[7])
    redis.call("ZADD", KEYS[3], ARGV[6], ARGV[1])
    redis.call("EXPIRE", KEYS[3], ARGV[5])
end
return 1
"#;

pub(super) const REPAIR_SCORE_SCRIPT: &str = r#"
local score = redis.call("ZSCORE", KEYS[1], ARGV[1])
if not score
    or tonumber(score) > tonumber(ARGV[4])
    or redis.call("GET", KEYS[2]) ~= ARGV[2] then
    return 0
end
redis.call("ZADD", KEYS[1], ARGV[3], ARGV[1])
return 1
"#;
