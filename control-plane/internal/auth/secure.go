// VERIDACTUS 安全增强 — Rate Limit + 账户锁定 + 密码强度 + 安全响应头
package auth

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"time"
	"unicode"

	"github.com/veridactus/control-plane/internal/store"
)

// ==================== Rate Limiter (Token Bucket) ====================

type rateLimiter struct {
	mu      sync.Mutex
	buckets map[string]*tokenBucket
}

type tokenBucket struct {
	tokens   float64
	lastTime time.Time
	limit    float64 // tokens per second
	burst    float64
}

func newRateLimiter(limit, burst float64) *rateLimiter {
	return &rateLimiter{buckets: make(map[string]*tokenBucket)}
}

func (rl *rateLimiter) allow(key string) bool {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	b, ok := rl.buckets[key]
	if !ok {
		b = &tokenBucket{tokens: 1.0, lastTime: time.Now(), limit: 1.0, burst: 5.0}
		rl.buckets[key] = b
	}

	now := time.Now()
	elapsed := now.Sub(b.lastTime).Seconds()
	b.tokens += elapsed * b.limit
	if b.tokens > b.burst { b.tokens = b.burst }
	b.lastTime = now

	if b.tokens < 1.0 { return false }
	b.tokens -= 1.0
	return true
}

// 定期清理旧 bucket
func (rl *rateLimiter) cleanup() {
	rl.mu.Lock()
	defer rl.mu.Unlock()
	for k, b := range rl.buckets {
		if time.Since(b.lastTime) > 10*time.Minute {
			delete(rl.buckets, k)
		}
	}
}

var (
	loginLimiter    = newRateLimiter(0.2, 5)  // 每 5 秒 1 次登录尝试
	registerLimiter = newRateLimiter(0.1, 3)   // 每 10 秒 1 次注册
	phoneLimiter    = newRateLimiter(0.1, 2)   // 每 10 秒 1 次短信
)

func init() {
	go func() {
		for range time.Tick(5 * time.Minute) {
			loginLimiter.cleanup()
			registerLimiter.cleanup()
			phoneLimiter.cleanup()
		}
	}()
}

// RateLimitLogin 登录频率限制 (Redis 优先，内存兜底)
func RateLimitLogin(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ip := clientIP(r)
		redisKey := fmt.Sprintf("rate:login:%s", ip)
		rl := GetRedisLimiter()
		if rl.IsEnabled() {
			allowed, retryAfter := rl.Allow(r.Context(), redisKey, 0.2, 5)
			if !allowed {
				w.Header().Set("Retry-After", fmt.Sprintf("%d", retryAfter))
				writeRateLimitError(w)
				return
			}
		} else {
			if !loginLimiter.allow(ip) {
				writeRateLimitError(w)
				return
			}
		}
		next.ServeHTTP(w, r)
	})
}

// RateLimitRegister 注册频率限制 (Redis 优先，内存兜底)
func RateLimitRegister(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		ip := clientIP(r)
		redisKey := fmt.Sprintf("rate:register:%s", ip)
		rl := GetRedisLimiter()
		if rl.IsEnabled() {
			allowed, retryAfter := rl.Allow(r.Context(), redisKey, 0.1, 3)
			if !allowed {
				w.Header().Set("Retry-After", fmt.Sprintf("%d", retryAfter))
				writeRateLimitError(w)
				return
			}
		} else {
			if !registerLimiter.allow(ip) {
				writeRateLimitError(w)
				return
			}
		}
		next.ServeHTTP(w, r)
	})
}

// RateLimitPhone 短信频率限制 (Redis 优先，内存兜底)
func RateLimitPhone(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		phone := r.URL.Query().Get("phone")
		if phone == "" {
			// 尝试从 body 解析
			r.ParseForm()
			phone = r.FormValue("phone")
		}
		if phone == "" {
			phone = clientIP(r)
		}
		redisKey := fmt.Sprintf("rate:phone:%s", phone)
		rl := GetRedisLimiter()
		if rl.IsEnabled() {
			allowed, retryAfter := rl.Allow(r.Context(), redisKey, 0.1, 2)
			if !allowed {
				w.Header().Set("Retry-After", fmt.Sprintf("%d", retryAfter))
				writeRateLimitError(w)
				return
			}
		} else {
			if !phoneLimiter.allow(clientIP(r)) {
				writeRateLimitError(w)
				return
			}
		}
		next.ServeHTTP(w, r)
	})
}

// ==================== 密码强度验证 ====================

var (
	errPasswordTooShort   = fmt.Errorf("密码至少需要 8 个字符")
	errPasswordNoUpper     = fmt.Errorf("密码需要包含大写字母")
	errPasswordNoLower     = fmt.Errorf("密码需要包含小写字母")
	errPasswordNoDigit    = fmt.Errorf("密码需要包含数字")
	errPasswordNoSpecial  = errors.New("密码需要包含特殊字符 (!@#$%^&*)")
	errPasswordCommon     = fmt.Errorf("密码过于常见，请使用更强的密码")
)

var commonPasswords = map[string]bool{
	"password": true, "12345678": true, "123456789": true, "qwerty123": true,
	"admin123": true, "password1": true, "veridactus123": true, "letmein123": true,
	"welcome1": true, "monkey123": true, "abc123456": true,
}

// ValidatePassword 验证密码强度
// 要求: >=8 chars, 大写+小写+数字+特殊字符, 非常见密码
func ValidatePassword(password string, requireSpecial bool) error {
	pw := strings.TrimSpace(password)
	if len(pw) < 8 { return errPasswordTooShort }

	hasUpper, hasLower, hasDigit, hasSpecial := false, false, false, false
	for _, r := range pw {
		switch {
		case unicode.IsUpper(r): hasUpper = true
		case unicode.IsLower(r): hasLower = true
		case unicode.IsDigit(r): hasDigit = true
		case strings.ContainsRune("!@#$%^&*()_+-=[]{}|;:,.<>?~", r): hasSpecial = true
		}
	}

	if !hasUpper { return errPasswordNoUpper }
	if !hasLower { return errPasswordNoLower }
	if !hasDigit { return errPasswordNoDigit }
	if requireSpecial && !hasSpecial { return errPasswordNoSpecial }
	if commonPasswords[strings.ToLower(pw)] { return errPasswordCommon }

	return nil
}

// ==================== 账户锁定 ====================

// IsAccountLocked 检查账户是否被锁定
// 使用 store settings 存储失败计数，生产环境应使用 Redis
func IsAccountLocked(ctx context.Context, s store.StoreFacade, email string) (bool, string) {
	const lockoutKey = "account-locks"
	const maxAttempts = 5
	const lockDuration = 15 * time.Minute

	settings, _ := s.GetSettings(ctx, lockoutKey)
	lockKey := "lock:" + email

	// 检查锁定状态
	if lockUntil, ok := settings[lockKey]; ok {
		if until, err := time.Parse(time.RFC3339, lockUntil); err == nil {
			if time.Now().Before(until) {
				return true, fmt.Sprintf("账户已锁定，请在 %s 后重试", until.Format("15:04:05"))
			}
			// 锁定已过期，清除
			s.UpdateSettings(ctx, lockoutKey, map[string]string{lockKey: ""})
		}
	}
	return false, ""
}

// RecordLoginFailure 记录登录失败
func RecordLoginFailure(ctx context.Context, s store.StoreFacade, email string) int {
	const lockoutKey = "account-locks"
	const maxAttempts = 5
	const lockDuration = 15 * time.Minute

	settings, _ := s.GetSettings(ctx, lockoutKey)
	failKey := "fail:" + email
	count := 0
	if v, ok := settings[failKey]; ok {
		fmt.Sscanf(v, "%d", &count)
	}
	count++
	s.UpdateSettings(ctx, lockoutKey, map[string]string{
		failKey: fmt.Sprintf("%d", count),
	})
	if count >= maxAttempts {
		lockUntil := time.Now().Add(lockDuration).UTC().Format(time.RFC3339)
		s.UpdateSettings(ctx, lockoutKey, map[string]string{
			"lock:" + email: lockUntil,
		})
	}
	return count
}

// ClearLoginFailures 清除登录失败记录（成功登录后调用）
func ClearLoginFailures(ctx context.Context, s store.StoreFacade, email string) {
	const lockoutKey = "account-locks"
	s.UpdateSettings(ctx, lockoutKey, map[string]string{
		"fail:" + email: "0",
	})
}

// ==================== 安全响应头 ====================

// SecurityHeaders 添加安全响应头
func SecurityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("X-Frame-Options", "DENY")
		w.Header().Set("X-XSS-Protection", "1; mode=block")
		w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
		w.Header().Set("Cache-Control", "no-store, max-age=0")
		w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
		next.ServeHTTP(w, r)
	})
}

// ==================== Utilities ====================

func clientIP(r *http.Request) string {
	if fwd := r.Header.Get("X-Forwarded-For"); fwd != "" {
		return strings.Split(fwd, ",")[0]
	}
	if fwd := r.Header.Get("X-Real-IP"); fwd != "" {
		return fwd
	}
	return r.RemoteAddr
}

func writeRateLimitError(w http.ResponseWriter) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Retry-After", "5")
	w.WriteHeader(http.StatusTooManyRequests)
	w.Write([]byte(`{"error":"rate_limited","message":"请求过于频繁，请稍后再试"}`))
}

// GenerateCSRFToken 生成 CSRF token
func GenerateCSRFToken() string {
	b := make([]byte, 32)
	rand.Read(b)
	return hex.EncodeToString(b)
}

// GenerateSecureToken 生成安全随机 token
func GenerateSecureToken() string {
	b := make([]byte, 32)
	rand.Read(b)
	return hex.EncodeToString(b)
}
