// VERIDACTUS 控制平面 — JWT 认证模块
package auth

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// VeridactusClaims 自定义 JWT Claims
type VeridactusClaims struct {
	jwt.RegisteredClaims
	UserID      string `json:"user_id"`
	Email       string `json:"email"`
	OrgID       string `json:"org_id"`
	WorkspaceID string `json:"workspace_id"`
	Role        string `json:"role"`
	Plan        string `json:"plan"` // "personal" | "enterprise"
}

var (
	jwtSecret      []byte
	accessTokenTTL  = 15 * time.Minute
	refreshTokenTTL = 30 * 24 * time.Hour // 30 天
)

// InitJWT 初始化 JWT 签名密钥
func InitJWT(secret string) {
	if secret == "" {
		// 开发环境生成随机密钥
		b := make([]byte, 32)
		rand.Read(b)
		jwtSecret = []byte(hex.EncodeToString(b))
	} else {
		jwtSecret = []byte(secret)
	}
}

// GenerateAccessToken 签发访问令牌
func GenerateAccessToken(userID, email, orgID, workspaceID, role string) (string, error) {
	return GenerateAccessTokenWithPlan(userID, email, orgID, workspaceID, role, "personal")
}

func GenerateAccessTokenWithPlan(userID, email, orgID, workspaceID, role, plan string) (string, error) {
	now := time.Now()
	claims := VeridactusClaims{
		RegisteredClaims: jwt.RegisteredClaims{
			Issuer:    "veridactus",
			Subject:   userID,
			IssuedAt:  jwt.NewNumericDate(now),
			ExpiresAt: jwt.NewNumericDate(now.Add(accessTokenTTL)),
			ID:        generateTokenID(),
		},
		UserID:      userID,
		Email:       email,
		OrgID:       orgID,
		WorkspaceID: workspaceID,
		Role:        role,
		Plan:        plan,
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(jwtSecret)
}

// ValidateAccessToken 验证并解析访问令牌
func ValidateAccessToken(tokenString string) (*VeridactusClaims, error) {
	token, err := jwt.ParseWithClaims(tokenString, &VeridactusClaims{}, func(t *jwt.Token) (interface{}, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", t.Header["alg"])
		}
		return jwtSecret, nil
	})
	if err != nil {
		return nil, fmt.Errorf("parse token: %w", err)
	}

	claims, ok := token.Claims.(*VeridactusClaims)
	if !ok || !token.Valid {
		return nil, fmt.Errorf("invalid token claims")
	}

	return claims, nil
}

// SetAccessTokenTTL 设置访问令牌有效期
func SetAccessTokenTTL(ttl time.Duration) {
	accessTokenTTL = ttl
}

func generateTokenID() string {
	b := make([]byte, 16)
	rand.Read(b)
	return hex.EncodeToString(b)
}
