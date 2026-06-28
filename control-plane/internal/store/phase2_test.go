// VERIDACTUS Phase 2 测试：Virtual Key, Wallet, Key Resolution
package store_test

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/google/uuid"
	"github.com/veridactus/control-plane/internal/crypto"
	"github.com/veridactus/control-plane/internal/model"
)

func TestCreateVirtualKey_BYOK(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	userID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "Org", Slug: "org"})
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "WS", Slug: "ws"})
	st.CreateUser(ctx, &model.User{ID: userID, Email: "u@t.com", AuthProvider: "github"})

	// 加密 Provider Key
	envelope, err := crypto.EncryptProviderKey("sk-test-real-key-123456", "")
	if err != nil {
		t.Fatalf("encrypt: %v", err)
	}
	encJSON, _ := json.Marshal(envelope)

	fullKey, _ := crypto.GenerateAPIKey()
	vk := &model.VirtualKey{
		ID:                   uuid.New().String(),
		WorkspaceID:          wsID,
		Name:                 "My BYOK Key",
		KeyPrefix:            "vd-" + fullKey[:8],
		KeyHash:              crypto.HashKey(fullKey),
		Type:                 "byok",
		ProviderKeyEncrypted: string(encJSON),
		AllowedModels:        `["gpt-4o","claude-3"]`,
		RateLimitRPM:         30,
		RateLimitTPM:         50000,
		Status:               "active",
		CreatedBy:            userID,
	}

	created, err := st.CreateVirtualKey(ctx, vk)
	if err != nil {
		t.Fatalf("create virtual key: %v", err)
	}
	if created.KeyPrefix != vk.KeyPrefix {
		t.Errorf("key prefix mismatch: %s vs %s", created.KeyPrefix, vk.KeyPrefix)
	}

	// 验证解密
	retrieved, err := st.GetVirtualKeyByHash(ctx, vk.KeyHash)
	if err != nil {
		t.Fatalf("get by hash: %v", err)
	}
	if retrieved.Type != "byok" {
		t.Errorf("expected byok, got %s", retrieved.Type)
	}

	var retrievedEnvelope crypto.EnvelopeEncrypted
	json.Unmarshal([]byte(retrieved.ProviderKeyEncrypted), &retrievedEnvelope)
	decrypted, err := crypto.DecryptProviderKey(&retrievedEnvelope, "")
	if err != nil {
		t.Fatalf("decrypt: %v", err)
	}
	if decrypted != "sk-test-real-key-123456" {
		t.Errorf("decrypted key mismatch: got %s", decrypted)
	}
}

func TestCreateVirtualKey_Platform(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	userID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "Org", Slug: "org-p"})
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "WS", Slug: "ws-p"})
	st.CreateUser(ctx, &model.User{ID: userID, Email: "p@t.com", AuthProvider: "github"})

	fullKey, _ := crypto.GenerateAPIKey()
	vk := &model.VirtualKey{
		ID:            uuid.New().String(),
		WorkspaceID:   wsID,
		Name:          "Platform Key",
		KeyPrefix:     "vd-" + fullKey[:8],
		KeyHash:       crypto.HashKey(fullKey),
		Type:          "platform",
		AllowedModels: `["*"]`,
		Status:        "active",
		CreatedBy:     userID,
	}

	created, err := st.CreateVirtualKey(ctx, vk)
	if err != nil {
		t.Fatalf("create: %v", err)
	}
	if created.Type != "platform" {
		t.Errorf("expected platform, got %s", created.Type)
	}
	if created.ProviderKeyEncrypted != "" {
		t.Errorf("platform key should not have encrypted provider key")
	}
}

func TestRevokeVirtualKey(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	userID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "O", Slug: "o"})
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "W", Slug: "w"})
	st.CreateUser(ctx, &model.User{ID: userID, Email: "r@t.com", AuthProvider: "github"})

	fullKey, _ := crypto.GenerateAPIKey()
	vk := &model.VirtualKey{
		ID: uuid.New().String(), WorkspaceID: wsID, Name: "RevokeTest",
		KeyPrefix: "vd-" + fullKey[:8], KeyHash: crypto.HashKey(fullKey),
		Type: "platform", Status: "active", CreatedBy: userID,
	}
	st.CreateVirtualKey(ctx, vk)

	// 吊销
	if err := st.RevokeVirtualKey(ctx, vk.ID); err != nil {
		t.Fatalf("revoke: %v", err)
	}

	revoked, _ := st.GetVirtualKey(ctx, vk.ID)
	if revoked.Status != "revoked" {
		t.Errorf("expected revoked, got %s", revoked.Status)
	}
}

func TestWalletTopup(t *testing.T) {
	st := setupTestStore(t)
	ctx := context.Background()

	orgID := uuid.New().String()
	wsID := uuid.New().String()
	st.CreateOrganization(ctx, &model.Organization{ID: orgID, Name: "O", Slug: "o-w"})
	st.CreateWorkspace(ctx, &model.Workspace{ID: wsID, OrgID: orgID, Name: "W", Slug: "w-w"})

	// 获取自动创建的钱包
	wallet, err := st.GetWallet(ctx, wsID)
	if err != nil {
		t.Fatalf("wallet should exist: %v", err)
	}
	if wallet.BalanceUSDMicro != 0 {
		t.Errorf("initial balance should be 0, got %d", wallet.BalanceUSDMicro)
	}

	// 充值 $10.00 = 10,000,000 微美元
	if err := st.UpdateWalletBalance(ctx, wsID, 10_000_000); err != nil {
		t.Fatalf("topup: %v", err)
	}
	// 记录交易
	tx := &model.Transaction{
		ID: uuid.New().String(), WorkspaceID: wsID, WalletID: wallet.ID,
		Type: "credit", AmountUSDMicro: 10_000_000,
		BalanceAfterMicro: 10_000_000, Description: "Test topup",
	}
	if err := st.CreateTransaction(ctx, tx); err != nil {
		t.Fatalf("create transaction: %v", err)
	}

	updated, _ := st.GetWallet(ctx, wsID)
	if updated.BalanceUSDMicro != 10_000_000 {
		t.Errorf("expected 10M micro, got %d", updated.BalanceUSDMicro)
	}

	// 检查交易记录
	txs, err := st.ListTransactions(ctx, wsID, 10, 0)
	if err != nil {
		t.Fatalf("list transactions: %v", err)
	}
	if len(txs) < 1 {
		t.Fatal("expected at least 1 transaction")
	}
	if txs[0].Type != "credit" {
		t.Errorf("expected credit, got %s", txs[0].Type)
	}
}

func TestEnvelopeEncryptionRoundtrip(t *testing.T) {
	masterKey := "phase2-test-master-key-2026"
	plaintexts := []string{
		"sk-proj-short-key",
		"sk-ant-api03-very-long-anthropic-api-key-format-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
		"bce-v3/ALTAK-complex/baidu-ernie-key-with-slashes",
		"", // 空 key 应报错
	}

	for _, pt := range plaintexts {
		if pt == "" {
			_, err := crypto.EncryptProviderKey(pt, masterKey)
			if err == nil {
				t.Error("expected error for empty plaintext")
			}
			continue
		}
		enc, err := crypto.EncryptProviderKey(pt, masterKey)
		if err != nil {
			t.Fatalf("encrypt %q: %v", pt, err)
		}
		dec, err := crypto.DecryptProviderKey(enc, masterKey)
		if err != nil {
			t.Fatalf("decrypt %q: %v", pt, err)
		}
		if dec != pt {
			t.Errorf("roundtrip failed: got %q, want %q", dec, pt)
		}
	}
}
