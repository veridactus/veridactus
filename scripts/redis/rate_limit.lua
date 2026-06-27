-- VERIDACTUS Redis Lua: 限流令牌桶 (Rate Limiting)
-- 调用方式: EVALSHA <sha> 1 <rate_key> <max_tokens> <refill_interval_sec>
-- 返回: {allowed, remaining, reset_in_sec}
local key = KEYS[1]
local max_tokens = tonumber(ARGV[1])
local refill_interval = tonumber(ARGV[2])

local now = redis.call('TIME')
local current_time = now[1] * 1000 + now[2] / 1000

-- HGETALL: 获取 tokens, last_refill
local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
local tokens = tonumber(bucket[1]) or max_tokens
local last_refill = tonumber(bucket[2]) or current_time

-- 计算补充的 token 数
local elapsed = (current_time - last_refill) / 1000
local refill_tokens = math.floor(elapsed * (max_tokens / refill_interval))
if refill_tokens > 0 then
    tokens = math.min(max_tokens, tokens + refill_tokens)
    last_refill = current_time
end

-- 检查是否有可用 token
local allowed = 0
if tokens >= 1 then
    tokens = tokens - 1
    allowed = 1
end

-- 更新 bucket
redis.call('HMSET', key, 'tokens', tokens, 'last_refill', last_refill)
redis.call('EXPIRE', key, refill_interval * 2)

local reset_in = math.ceil(refill_interval - elapsed)
if reset_in < 0 then reset_in = 0 end

return {allowed, tokens, reset_in}
