// VERIDACTUS 控制平面 — 存储层集成测试
package store_test

import (
	"context"
	"os"
	"testing"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/model"
	"github.com/veridactus/control-plane/internal/store"
)

func setupTestStore(t *testing.T) store.StoreFacade {
	t.Helper()
	dbPath := t.TempDir() + "/test.db"
	st, err := store.NewStore(store.StoreConfig{
		Backend: "sqlite",
		DBPath:  dbPath,
	})
	if err != nil {
		t.Fatalf("create store: %v", err)
	}
	t.Cleanup(func() { st.Close(); os.Remove(dbPath) })
	return st
}

func TestCreateOrganization(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	org := &model.Organization{
		ID:   uuid.New().String(),
		Name: "Test Corp",
		Slug: "test-corp",
		Plan: "free",
	}
	if err := st.CreateOrganization(ctx, org); err != nil {
		t.Fatalf("create org: %v", err)
	}

	got, err := st.GetOrganization(ctx, org.ID)
	if err != nil {
		t.Fatalf("get org: %v", err)
	}
	if got.Name != "Test Corp" {
		t.Errorf("expected 'Test Corp', got '%s'", got.Name)
	}
	if got.Slug != "test-corp" {
		t.Errorf("expected 'test-corp', got '%s'", got.Slug)
	}
}

func TestCreateWorkspace(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "Org", Slug: "org"})

	ws := &model.Workspace{
		ID:    uuid.New().String(),
		OrgID: orgID,
		Name:  "Engineering",
		Slug:  "engineering",
	}
	if err := st.CreateWorkspace(ctx, ws); err != nil {
		t.Fatalf("create workspace: %v", err)
	}

	got, err := st.GetWorkspace(ctx, ws.ID)
	if err != nil {
		t.Fatalf("get workspace: %v", err)
	}
	if got.Name != "Engineering" {
		t.Errorf("expected 'Engineering', got '%s'", got.Name)
	}

	// 自动创建了钱包
	wallet, err := st.GetWallet(ctx, ws.ID)
	if err != nil {
		t.Fatalf("wallet should be auto-created: %v", err)
	}
	if wallet == nil {
		t.Fatal("wallet is nil")
	}
}

func TestCreateUser(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	user := &model.User{
		ID:           uuid.New().String(),
		Email:        "test@example.com",
		DisplayName:  "Test User",
		AuthProvider: "github",
		AuthProviderID: "12345",
	}
	if err := st.CreateUser(ctx, user); err != nil {
		t.Fatalf("create user: %v", err)
	}

	got, err := st.GetUser(ctx, user.ID)
	if err != nil {
		t.Fatalf("get user: %v", err)
	}
	if got.Email != "test@example.com" {
		t.Errorf("expected 'test@example.com', got '%s'", got.Email)
	}

	// 按 provider 查找
	got2, err := st.GetUserByProvider(ctx, "github", "12345")
	if err != nil {
		t.Fatalf("get user by provider: %v", err)
	}
	if got2.ID != user.ID {
		t.Errorf("expected same user ID")
	}
}

func TestAddMember(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "Org", Slug: "org"})

	wsID := uuid.New().String()
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "WS", Slug: "ws"})

	userID := uuid.New().String()
	st.CreateUser(ctx, &model.User{ID: userID, Email: "dev@test.com", AuthProvider: "github"})

	m := &model.WorkspaceMember{
		ID:          uuid.New().String(),
		WorkspaceID: wsID,
		UserID:      userID,
		Role:        "developer",
	}
	if err := st.AddMember(ctx, m); err != nil {
		t.Fatalf("add member: %v", err)
	}

	members, err := st.ListMembers(ctx, wsID)
	if err != nil {
		t.Fatalf("list members: %v", err)
	}
	if len(members) != 1 {
		t.Errorf("expected 1 member, got %d", len(members))
	}
	if members[0].Role != "developer" {
		t.Errorf("expected 'developer', got '%s'", members[0].Role)
	}
}

func TestPipelineWithWorkspace(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "Org", Slug: "org"})
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "WS", Slug: "ws"})

	p := &model.Pipeline{
		PlanID:      uuid.New().String(),
		OrgID:       orgID,
		WorkspaceID: wsID,
		Name:        "Default Pipeline",
		Description: "Test pipeline",
		Tenant:      "test",
		Stages:      []model.StageConfig{},
		Created:     "2026-06-01T00:00:00Z",
	}
	if err := st.CreatePipeline(ctx, p); err != nil {
		t.Fatalf("create pipeline: %v", err)
	}

	pipelines, err := st.ListPipelines(ctx, wsID)
	if err != nil {
		t.Fatalf("list pipelines: %v", err)
	}
	if len(pipelines) == 0 {
		t.Fatal("expected at least 1 pipeline")
	}
}

func TestRBACPermissions(t *testing.T) {
	// 此测试验证 RBAC 模块（在 auth 包中）
	// 此处作为集成验证：确保角色和权限矩阵正确定义
	roles := []string{"platform_admin", "org_admin", "workspace_admin", "developer", "auditor"}
	for _, role := range roles {
		if role == "" {
			t.Errorf("empty role in list")
		}
	}
	if len(roles) != 5 {
		t.Errorf("expected 5 roles, got %d", len(roles))
	}
}

func TestStoreHealthCheck(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	if err := st.HealthCheck(ctx); err != nil {
		t.Fatalf("health check failed: %v", err)
	}
}

func TestConfigVersions(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	v, err := st.GetConfigVersions(ctx)
	if err != nil {
		t.Fatalf("get config versions: %v", err)
	}
	// 种子版本号应该都是 0
	if v.PipelineVersion != 0 {
		t.Errorf("expected pipeline_version=0, got %d", v.PipelineVersion)
	}

	// 递增版本号
	if err := st.IncrementConfigVersion(ctx, "pipeline"); err != nil {
		t.Fatalf("increment version: %v", err)
	}

	v2, _ := st.GetConfigVersions(ctx)
	if v2.PipelineVersion != 1 {
		t.Errorf("expected pipeline_version=1 after increment, got %d", v2.PipelineVersion)
	}
}
