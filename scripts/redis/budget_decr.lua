-- VERIDACTUS Redis Lua: 原子预算扣减 + 日限额检查
-- 调用方式: EVALSHA <sha> 2 <budget_key> <daily_key> <amount_micro> <daily_limit_micro> <request_id>
-- 返回: {status, reason, remaining}
--   status: 1=ok, 0=blocked
--   reason: "ok" | "budget_exceeded" | "daily_limit_exceeded"
--   remaining: 剩余预算 (微美元 字符串)

local remaining = redis.call('DECRBY', KEYS[1], ARGV[1])

if remaining < 0 then
    redis.call('INCRBY', KEYS[1], ARGV[1])
    return {0, "budget_exceeded", tostring(redis.call('GET', KEYS[1]))}
end

local daily_limit = tonumber(ARGV[2])
if daily_limit > 0 then
    local daily_spent = redis.call('INCRBY', KEYS[2], ARGV[1])
    if daily_spent > daily_limit then
        redis.call('INCRBY', KEYS[1], ARGV[1])
        redis.call('DECRBY', KEYS[2], ARGV[1])
        return {0, "daily_limit_exceeded", tostring(daily_spent)}
    end
    local now = redis.call('TIME')
    local seconds_today = now[1] % 86400
    redis.call('EXPIRE', KEYS[2], 86400 - seconds_today + 60)
end

return {1, "ok", tostring(remaining)}
