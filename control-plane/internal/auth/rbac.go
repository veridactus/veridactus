// VERIDACTUS 控制平面 — RBAC 权限系统 (Casbin)
package auth

import (
	"log"
	"strings"
	"sync"

	"github.com/casbin/casbin/v2"
	"github.com/casbin/casbin/v2/model"
)

// 角色定义
const (
	RolePlatformAdmin  = "platform_admin"
	RoleOrgAdmin       = "org_admin"
	RoleWorkspaceAdmin = "workspace_admin"
	RoleDeveloper      = "developer"
	RoleAuditor        = "auditor"
)

var (
	enforcer     *casbin.Enforcer
	enforcerOnce sync.Once
)

// Casbin RBAC 模型定义 (直接使用 Casbin model 字符串，无需外部文件)
const casbinModel = `
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act

[role_definition]
g = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = (r.sub == p.sub || g(r.sub, p.sub)) && keyMatch(r.obj, p.obj) && regexMatch(r.act, p.act)
`

// 角色继承策略（g-type，Casbin GroupingPolicy）
var roleHierarchy = [][]string{
	{RoleOrgAdmin, RoleWorkspaceAdmin},       // org_admin 继承 workspace_admin 权限
	{RoleWorkspaceAdmin, RoleDeveloper},       // workspace_admin 继承 developer 权限
}

// 权限策略（p-type，Casbin Policy）
var permissionPolicies = [][]string{
	// ============ platform_admin ============
	{RolePlatformAdmin, "*", "*"},

	// ============ org_admin (继承 workspace_admin + 以下扩展) ============
	{RoleOrgAdmin, "org", "*"},
	{RoleOrgAdmin, "workspace", "*"},
	{RoleOrgAdmin, "member", "*"},
	{RoleOrgAdmin, "billing", "*"},
	{RoleOrgAdmin, "settings", "*"},

	// ============ workspace_admin (继承 developer + 以下扩展) ============
	{RoleWorkspaceAdmin, "pipeline", ".*"},
	{RoleWorkspaceAdmin, "apikey", ".*"},
	{RoleWorkspaceAdmin, "virtual_key", ".*"},
	{RoleWorkspaceAdmin, "trace", ".*"},
	{RoleWorkspaceAdmin, "member", "read|invite"},
	{RoleWorkspaceAdmin, "settings", "read|write"},
	{RoleWorkspaceAdmin, "billing", "read"},

	// ============ developer (基础权限) ============
	{RoleDeveloper, "pipeline", "read"},
	{RoleDeveloper, "plugin", "read"},
	{RoleDeveloper, "model", "read"},
	{RoleDeveloper, "apikey", "create_own"},
	{RoleDeveloper, "virtual_key", "create_own"},
	{RoleDeveloper, "trace", "read"},
	{RoleDeveloper, "chat", "use"},
	{RoleDeveloper, "playground", "use"},

	// ============ auditor (仅审计) ============
	{RoleAuditor, "trace", "read|export"},
	{RoleAuditor, "compliance", "read|export"},
	{RoleAuditor, "audit", "read|export"},
}

// InitRBAC 初始化 Casbin RBAC 引擎（服务启动时调用）
func InitRBAC() error {
	var initErr error
	enforcerOnce.Do(func() {
		m, err := model.NewModelFromString(casbinModel)
		if err != nil {
			initErr = err
			return
		}
		e, err := casbin.NewEnforcer(m)
		if err != nil {
			initErr = err
			return
		}
		// 批量添加权限策略（p-type）
		_, err = e.AddPolicies(permissionPolicies)
		if err != nil {
			initErr = err
			return
		}
		// 批量添加角色继承（g-type）
		_, err = e.AddGroupingPolicies(roleHierarchy)
		if err != nil {
			initErr = err
			return
		}
		enforcer = e
		log.Println("INFO: Casbin RBAC engine initialized with 5 roles")
	})
	return initErr
}

// GetEnforcer 获取 Casbin Enforcer 实例（用于动态策略管理）
func GetEnforcer() *casbin.Enforcer {
	if enforcer == nil {
		_ = InitRBAC()
	}
	return enforcer
}

// CheckPermission 检查角色是否拥有指定权限 (Casbin 驱动)
// permission 格式: "resource:action" (如 "pipeline:write")
func CheckPermission(role, permission string) bool {
	if enforcer == nil {
		_ = InitRBAC()
	}
	if enforcer == nil {
		return false
	}

	parts := strings.SplitN(permission, ":", 2)
	if len(parts) != 2 {
		return false
	}
	resource, action := parts[0], parts[1]

	ok, err := enforcer.Enforce(role, resource, action)
	if err != nil {
		log.Printf("Casbin enforce error: %v (role=%s, perm=%s)", err, role, permission)
	}
	return err == nil && ok
}

// RequiresPermission 返回权限检查函数的快捷方法
func RequiresPermission(role string) func(string) bool {
	return func(permission string) bool {
		return CheckPermission(role, permission)
	}
}

// GetRolePermissions 获取角色的所有直接权限 (不含继承)
func GetRolePermissions(role string) []string {
	if enforcer == nil {
		_ = InitRBAC()
	}
	if enforcer == nil {
		return nil
	}
	perms := enforcer.GetPermissionsForUser(role)
	var result []string
	for _, p := range perms {
		if len(p) >= 3 {
			result = append(result, p[1]+":"+p[2])
		}
	}
	return result
}

// IsValidRole 检查角色是否有效
func IsValidRole(role string) bool {
	switch role {
	case RolePlatformAdmin, RoleOrgAdmin, RoleWorkspaceAdmin, RoleDeveloper, RoleAuditor:
		return true
	}
	return false
}

// AddRoleForUser 为用户添加角色（动态 RBAC — 用于自定义角色/ABAC 扩展）
func AddRoleForUser(userID, role, domain string) (bool, error) {
	if enforcer == nil {
		_ = InitRBAC()
	}
	return enforcer.AddRoleForUserInDomain(userID, role, domain)
}
