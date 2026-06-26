// VERIDACTUS 控制平面 — Redis 限流器
// 使用 scripts/redis/rate_limit.lua 的令牌桶算法
package auth

import (
	"context"
	"fmt"
	"log"
	"os"
	"sync"
	"time"

	"github.com/redis/go-redis/v9"
)

// RedisRateLimiter Redis 令牌桶限流器
type RedisRateLimiter struct {
	client   *redis.Client
	luaSHA   string // 预加载的 Lua 脚本 SHA
	mu       sync.RWMutex
	enabled  bool
}

var (
	globalRedisLimiter *RedisRateLimiter
	redisLimiterOnce   sync.Once
)

// rate_limit.lua 令牌桶 Lua 脚本（与 scripts/redis/rate_limit.lua 一致）
const rateLimitLuaScript = `
local key = KEYS[1]
local rate = tonumber(ARGV[1])      -- tokens per second
local capacity = tonumber(ARGV[2])  -- max tokens (burst)
local now = tonumber(ARGV[3])       -- current unix timestamp in seconds

local bucket = redis.call('HMGET', key, 'tokens', 'last_refill')
local tokens = tonumber(bucket[1])
local last_refill = tonumber(bucket[2])

if tokens == nil then
    tokens = capacity
    last_refill = now
end

-- 计算自上次填充以来应添加的 token 数
local elapsed = math.max(0, now - last_refill)
local refill = elapsed * rate
local new_tokens = math.min(capacity, tokens + refill)

-- 检查是否有可用 token
if new_tokens < 1 then
    -- 计算下次可用时间
    local retry_after = math.ceil((1 - new_tokens) / rate)
    redis.call('HMSET', KEYS[1], 'tokens', new_tokens, 'last_refill', now)
    redis.call('EXPIRE', KEYS[1], math.ceil(capacity / rate) + 10)
    return {0, retry_after}
end

-- 扣减 1 个 token
new_tokens = new_tokens - 1
redis.call('HMSET', KEYS[1], 'tokens', new_tokens, 'last_refill', now)
redis.call('EXPIRE', KEYS[1], math.ceil(capacity / rate) + 10)
return {1, 0}
`

// InitRedisLimiter 初始化全局 Redis 限流器（幂等）
func InitRedisLimiter() *RedisRateLimiter {
	redisLimiterOnce.Do(func() {
		host := os.Getenv("REDIS_HOST")
		if host == "" {
			host = "localhost"
		}
		port := os.Getenv("REDIS_PORT")
		if port == "" {
			port = "6379"
		}

		client := redis.NewClient(&redis.Options{
			Addr:     fmt.Sprintf("%s:%s", host, port),
			Password: os.Getenv("REDIS_PASSWORD"),
			DB:       0,
		})

		limiter := &RedisRateLimiter{client: client}

		// 测试连接
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()
		if err := client.Ping(ctx).Err(); err != nil {
			log.Printf("WARN: Redis rate limiter unavailable (%v), falling back to in-memory", err)
			limiter.enabled = false
		} else {
			// 预加载 Lua 脚本
			sha, err := client.ScriptLoad(ctx, rateLimitLuaScript).Result()
			if err != nil {
				log.Printf("WARN: Redis Lua script load failed (%v), falling back to in-memory", err)
				limiter.enabled = false
			} else {
				limiter.luaSHA = sha
				limiter.enabled = true
				log.Println("INFO: Redis rate limiter initialized successfully")
			}
		}

		globalRedisLimiter = limiter
	})
	return globalRedisLimiter
}

// GetRedisLimiter 获取全局 Redis 限流器实例
func GetRedisLimiter() *RedisRateLimiter {
	if globalRedisLimiter == nil {
		return InitRedisLimiter()
	}
	return globalRedisLimiter
}

// IsEnabled 检查 Redis 限流是否可用
func (rl *RedisRateLimiter) IsEnabled() bool {
	rl.mu.RLock()
	defer rl.mu.RUnlock()
	return rl.enabled
}

// Allow 检查是否允许一个请求通过
// key: 限流键（如 "rate:login:user@email.com"）
// rate: 每秒允许的请求数
// capacity: 突发容量（令牌桶最大 token 数）
// 返回 (allowed bool, retryAfterSeconds int)
func (rl *RedisRateLimiter) Allow(ctx context.Context, key string, rate float64, capacity int) (bool, int) {
	if !rl.IsEnabled() {
		return true, 0 // Redis 不可用时放行
	}

	now := time.Now().Unix()
	result, err := rl.client.EvalSha(ctx, rl.luaSHA, []string{key}, rate, capacity, now).Result()
	if err != nil {
		// 脚本可能因 Redis 重启而丢失，尝试重新加载
		log.Printf("WARN: Redis Lua eval failed, reloading script: %v", err)
		sha, loadErr := rl.client.ScriptLoad(ctx, rateLimitLuaScript).Result()
		if loadErr != nil {
			return true, 0 // 降级放行
		}
		rl.luaSHA = sha
		result, err = rl.client.EvalSha(ctx, rl.luaSHA, []string{key}, rate, capacity, now).Result()
		if err != nil {
			return true, 0 // 降级放行
		}
	}

	arr, ok := result.([]interface{})
	if !ok || len(arr) < 2 {
		return true, 0
	}

	allowed := false
	retryAfter := 0
	if v, ok := arr[0].(int64); ok {
		allowed = v == 1
	}
	if v, ok := arr[1].(int64); ok {
		retryAfter = int(v)
	}

	return allowed, retryAfter
}

// Close 关闭 Redis 连接
func (rl *RedisRateLimiter) Close() error {
	if rl.client != nil {
		return rl.client.Close()
	}
	return nil
}
