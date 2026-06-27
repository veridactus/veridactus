// VERIDACTUS 控制平面 v2.0 — 多租户重构入口
// 支持 PostgreSQL (生产) 和 SQLite (单机开发) 双模式
package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"regexp"
	"strings"
	"syscall"
	"time"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/auth"
	"github.com/veridactus/control-plane/internal/crypto"
	"github.com/veridactus/control-plane/internal/store"
)

func main() {
	// ==================== 配置加载 ====================
	storeBackend := getEnv("STORE_BACKEND", "postgres") // postgres (生产默认) | sqlite (仅本地调试)
	dbPath := getEnv("DB_PATH", "./veridactus.db")
	jwtSecret := getEnv("JWT_SECRET", "")
	adminKey := getEnv("VERIDACTUS_ADMIN_KEY", "")
	port := getEnv("PORT", "8081")
	corsOrigin := getEnv("CORS_ORIGINS", "") // 默认仅允许同源，生产设置具体域名
	dpHost := getEnv("DP_HOST", "localhost:8080")          // 数据面地址，用于模型更新通知

	// PostgreSQL 为生产默认后端，未配置时给出明确错误
	if storeBackend == "postgres" && getEnv("PG_HOST", "") == "" {
		logInfo("PG_HOST not set, using localhost")
	}

	// 初始化 JWT
	auth.InitJWT(jwtSecret)
	if jwtSecret == "" {
		if getEnv("VERIDACTUS_ENV", "") == "development" {
			logWarn("JWT_SECRET not set, using random key (tokens invalid on restart)")
		} else {
			logFatal("JWT_SECRET must be set for non-development environments")
		}
	}

	// 初始化 Casbin RBAC 引擎
	if err := auth.InitRBAC(); err != nil {
		logFatal("Casbin RBAC initialization failed", "error", err)
	}

	// 初始化主密钥（加密服务依赖，必须在任何加密操作前调用）
	if err := crypto.InitMasterKey(); err != nil {
		logFatal("Master key initialization failed", "error", err)
	}

	// ==================== 存储初始化 ====================
	cfg := store.StoreConfig{
		Backend:   storeBackend,
		DBPath:    dbPath,
		PGHost:    getEnv("PG_HOST", "localhost"),
		PGPort:    getEnvInt("PG_PORT", 5432),
		PGUser:    getEnv("PG_USER", "veridactus"),
		PGPass:    getEnv("PG_PASS", "veridactus"),
		PGDBName:  getEnv("PG_DB_NAME", "veridactus"),
		PGSSLMode: getEnv("PG_SSLMODE", "disable"),
	}

	st, err := store.NewStore(cfg)
	if err != nil {
		logFatal("store init failed", "error", err)
	}
	defer st.Close()
	logInfo("Storage initialized", "backend", storeBackend)

	// ==================== 路由注册 ====================
	srv := NewServer(st, jwtSecret, adminKey, dpHost)

	mux := http.NewServeMux()
	srv.RegisterRoutes(mux)

	// 中间件链: CORS → JWT → AdminKey → Logger
	// CORS 最外层（处理 OPTIONS 预检）
	// JWT 解析 Bearer token 并注入 claims
	// AdminKey 兜底认证（公开端点跳过）
	// Logger 最内层（记录已认证的用户信息）
	var handler http.Handler = mux
	handler = auth.RequestLogger(handler)
	handler = auth.AdminKeyMiddleware(adminKey)(handler)
	handler = auth.JWTMiddleware(handler)
	handler = auth.CORSMiddleware(corsOrigin)(handler)

	// ==================== 启动服务 ====================
	httpServer := &http.Server{
		Addr:    fmt.Sprintf(":%s", port),
		Handler: handler,
	}

	// 后台: 等待数据面就绪并推送配置
	go func() {
		time.Sleep(3 * time.Second)
		waitAndPushConfig(st)
	}()

	go func() {
		logInfo("Control Plane started", "port", port, "store_backend", storeBackend)
		if err := httpServer.ListenAndServe(); err != http.ErrServerClosed {
			logFatal("server listen error", "error", err)
		}
	}()

	// 优雅关闭
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit
	logInfo("Shutting down...")
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	httpServer.Shutdown(ctx)
	logInfo("Shutdown complete")
}

func waitAndPushConfig(st store.StoreFacade) {
	for i := 0; i < 30; i++ {
		c := &http.Client{Timeout: 5 * time.Second}
		resp, err := c.Get(dpURL + "/health")
		if err == nil && resp.StatusCode == 200 {
			resp.Body.Close()
			logInfo("Data plane ready")
			return
		}
		if resp != nil { resp.Body.Close() }
		time.Sleep(2 * time.Second)
	}
	logWarn("Data plane not ready after 30 attempts")
}

// ==================== 工具函数 ====================

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" { return v }
	return fallback
}

func getEnvInt(key string, fallback int) int {
	v := os.Getenv(key)
	if v == "" { return fallback }
	var result int
	fmt.Sscanf(v, "%d", &result)
	return result
}

func generateState() string {
	return uuid.New().String()
}

func hashToken(token string) string {
	h := sha256.Sum256([]byte(token))
	return hex.EncodeToString(h[:])
}

func timeNow() time.Time { return time.Now() }

func sanitizeSlug(name string) string {
	slug := strings.ToLower(name)
	slug = strings.ReplaceAll(slug, " ", "-")
	reg := regexp.MustCompile(`[^a-z0-9-]`)
	slug = reg.ReplaceAllString(slug, "")
	if slug == "" { slug = "workspace" }
	return slug
}

var dpURL = getEnv("DATA_PLANE_URL", "http://localhost:8080")
