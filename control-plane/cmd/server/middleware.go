// VERIDACTUS 控制平面 — HTTP 中间件（CORS / Admin 认证）
package main

import (
	"net/http"
	"os"
)

// corsMiddleware 处理跨域请求，允许配置的来源访问管理 API。
func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		allowedOrigin := os.Getenv("VERIDACTUS_CORS_ORIGIN")
		if allowedOrigin == "" {
			allowedOrigin = "http://localhost:3000"
		}
		w.Header().Set("Access-Control-Allow-Origin", allowedOrigin)
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Admin-Key")
		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

// adminAuthMiddleware 使用 X-Admin-Key 头部保护管理 API 端点。
// 健康检查端点 (/api/v1/health) 公开，无需认证。
func adminAuthMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Health check is public
		if r.URL.Path == "/api/v1/health" {
			next.ServeHTTP(w, r)
			return
		}
		adminKey := os.Getenv("VERIDACTUS_ADMIN_KEY")
		if adminKey == "" {
			// No admin key configured: allow all in dev mode, warn in production
			logWarn("VERIDACTUS_ADMIN_KEY not set — management API is unprotected")
			next.ServeHTTP(w, r)
			return
		}
		provided := r.Header.Get("X-Admin-Key")
		if provided == "" {
			provided = r.URL.Query().Get("admin_key")
		}
		if provided != adminKey {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusUnauthorized)
			w.Write([]byte(`{"error":"unauthorized: valid X-Admin-Key header required"}`))
			return
		}
		next.ServeHTTP(w, r)
	})
}
