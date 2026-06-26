// VERIDACTUS 控制平面 — 认证中间件
package auth

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"strings"
)

type contextKey string

const (
	// ContextKeyClaims 上下文中存放 JWT claims 的 key
	ContextKeyClaims contextKey = "veridactus_claims"
	// ContextKeyUserID 上下文中存放 user_id 的 key
	ContextKeyUserID contextKey = "veridactus_user_id"
	// ContextKeyWorkspaceID 上下文中存放 workspace_id 的 key
	ContextKeyWorkspaceID contextKey = "veridactus_workspace_id"
	// ContextKeyRole 上下文中存放 role 的 key
	ContextKeyRole contextKey = "veridactus_role"
)

// isPublicPath 判断是否为公开端点（无需认证）
func isPublicPath(path string) bool {
	publicPaths := map[string]bool{
		"/api/v1/health":     true,
		"/api/v1/config/poll": true,
	}
	if publicPaths[path] {
		return true
	}
	// /api/v1/auth/* 路径公开
	if strings.HasPrefix(path, "/api/v1/auth/") {
		return true
	}
	// /internal/* 路径是内部端点（数据面调用）
	if strings.HasPrefix(path, "/internal/") {
		return true
	}
	return false
}

// JWTMiddleware JWT 认证中间件
// 从 Authorization: Bearer <token> 头部提取并验证 JWT
// 公开端点跳过认证，受保护端点要求有效 JWT
func JWTMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// 公开端点：有 JWT 则注入 claims，无 JWT 则放行
		if isPublicPath(r.URL.Path) {
			authHeader := r.Header.Get("Authorization")
			if authHeader != "" {
				parts := strings.SplitN(authHeader, " ", 2)
				if len(parts) == 2 && strings.EqualFold(parts[0], "bearer") {
					if claims, err := ValidateAccessToken(parts[1]); err == nil {
						ctx := injectClaims(r.Context(), claims)
						next.ServeHTTP(w, r.WithContext(ctx))
						return
					}
				}
			}
			next.ServeHTTP(w, r)
			return
		}

		// 受保护端点：必须提供有效 JWT
		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			// 不放行——留待 AdminKeyMiddleware 兜底
			next.ServeHTTP(w, r)
			return
		}

		parts := strings.SplitN(authHeader, " ", 2)
		if len(parts) != 2 || !strings.EqualFold(parts[0], "bearer") {
			writeAuthError(w, "invalid authorization format, expected 'Bearer <token>'")
			return
		}

		claims, err := ValidateAccessToken(parts[1])
		if err != nil {
			writeAuthError(w, "invalid or expired token: "+err.Error())
			return
		}

		// 将 claims 注入 context
		ctx := injectClaims(r.Context(), claims)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// injectClaims 将 JWT claims 注入 context
func injectClaims(ctx context.Context, claims *VeridactusClaims) context.Context {
	ctx = context.WithValue(ctx, ContextKeyClaims, claims)
	ctx = context.WithValue(ctx, ContextKeyUserID, claims.UserID)
	ctx = context.WithValue(ctx, ContextKeyWorkspaceID, claims.WorkspaceID)
	ctx = context.WithValue(ctx, ContextKeyRole, claims.Role)
	return ctx
}

// AdminKeyMiddleware X-Admin-Key 中间件（兜底认证）
// 在 JWT 中间件之后运行。如果请求已有 JWT claims 则放行；
// 否则检查 X-Admin-Key（公开端点跳过检查）。
// 必须放在 JWTMiddleware 之后。
func AdminKeyMiddleware(adminKey string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// 公开端点直接放行
			if isPublicPath(r.URL.Path) {
				next.ServeHTTP(w, r)
				return
			}

			// 如果已经有 JWT claims（由 JWTMiddleware 注入）则跳过
			if claims := r.Context().Value(ContextKeyClaims); claims != nil {
				next.ServeHTTP(w, r)
				return
			}

			// 如果没有配置 admin_key，允许通过（开发模式）
			if adminKey == "" {
				next.ServeHTTP(w, r)
				return
			}

			// 检查 X-Admin-Key
			key := r.Header.Get("X-Admin-Key")
			if key != adminKey {
				writeAuthError(w, "invalid admin key")
				return
			}

			// 注入 admin 角色 context
			ctx := context.WithValue(r.Context(), ContextKeyRole, RolePlatformAdmin)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// RequireRole 角色中间件守卫
func RequireRole(allowedRoles ...string) func(http.Handler) http.Handler {
	roleSet := make(map[string]bool, len(allowedRoles))
	for _, r := range allowedRoles {
		roleSet[r] = true
	}

	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			role, _ := r.Context().Value(ContextKeyRole).(string)
			if role == "" {
				writeAuthError(w, "authentication required")
				return
			}
			if !roleSet[role] && role != RolePlatformAdmin {
				writeForbiddenError(w, "insufficient permissions")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

// GetUserID 从 context 中获取当前用户 ID
func GetUserID(ctx context.Context) string {
	id, _ := ctx.Value(ContextKeyUserID).(string)
	return id
}

// GetWorkspaceID 从 context 中获取当前工作空间 ID
func GetWorkspaceID(ctx context.Context) string {
	id, _ := ctx.Value(ContextKeyWorkspaceID).(string)
	return id
}

// GetRole 从 context 中获取当前用户角色
func GetRole(ctx context.Context) string {
	role, _ := ctx.Value(ContextKeyRole).(string)
	return role
}

func writeAuthError(w http.ResponseWriter, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusUnauthorized)
	json.NewEncoder(w).Encode(map[string]string{
		"error": "authentication_error",
		"message": msg,
	})
}

func writeForbiddenError(w http.ResponseWriter, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusForbidden)
	json.NewEncoder(w).Encode(map[string]string{
		"error": "forbidden",
		"message": msg,
	})
}

// CORS 中间件
func CORSMiddleware(allowedOrigins string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			origin := r.Header.Get("Origin")
			if origin == "" {
				origin = "*"
			}
			if allowedOrigins == "" || allowedOrigins == "*" {
				w.Header().Set("Access-Control-Allow-Origin", "*")
			} else {
				// 简单白名单检查
				allowed := strings.Split(allowedOrigins, ",")
				for _, a := range allowed {
					if strings.TrimSpace(a) == origin || strings.TrimSpace(a) == "*" {
						w.Header().Set("Access-Control-Allow-Origin", origin)
						break
					}
				}
			}
			w.Header().Set("Access-Control-Allow-Methods", "GET,POST,PUT,DELETE,OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type,Authorization,X-Admin-Key,X-Request-ID")
			w.Header().Set("Access-Control-Max-Age", "86400")

			if r.Method == http.MethodOptions {
				w.WriteHeader(http.StatusNoContent)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequestLogger 请求日志中间件
func RequestLogger(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		userID := GetUserID(r.Context())
		if userID == "" {
			userID = "anonymous"
		}
		log.Printf("[%s] %s %s user=%s", r.Method, r.URL.Path, r.RemoteAddr, userID)
		next.ServeHTTP(w, r)
	})
}
