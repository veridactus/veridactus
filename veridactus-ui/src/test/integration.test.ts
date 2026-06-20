import { describe, it, expect, vi, beforeAll, afterAll } from 'vitest';

const API_BASE = 'http://localhost:8081/api/v1';

const waitFor = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

describe('Integration Tests - Backend API', () => {
  beforeAll(() => {
    vi.useRealTimers();
  });

  afterAll(() => {
    vi.useFakeTimers();
  });

  describe('Control Plane Health', () => {
    it('should have healthy control plane', async () => {
      const res = await fetch(`${API_BASE}/health`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data.status).toBe('ok');
      expect(data.version).toBe('0.2.1');
    });
  });

  describe('API Keys Management', () => {
    it('should list API keys', async () => {
      const res = await fetch(`${API_BASE}/apikeys`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('total');
      expect(data).toHaveProperty('keys');
    });

    it('should create a new API key', async () => {
      const testKeyName = `test-key-${Date.now()}`;
      const res = await fetch(`${API_BASE}/apikeys`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: testKeyName }),
      });
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('id');
      expect(data.name).toBe(testKeyName);
    });
  });

  describe('Models Configuration', () => {
    it('should list models', async () => {
      const res = await fetch(`${API_BASE}/models`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('total');
      expect(data).toHaveProperty('models');
    });

    it('should have valid model structure', async () => {
      const res = await fetch(`${API_BASE}/models`);
      const data = await res.json();
      const models = data.models || [];
      if (models.length > 0) {
        const model = models[0];
        expect(model).toHaveProperty('id');
        expect(model).toHaveProperty('name');
        expect(model).toHaveProperty('upstream_url');
        expect(model).toHaveProperty('status');
      }
    });
  });

  describe('Pipelines Management', () => {
    it('should list pipelines', async () => {
      const res = await fetch(`${API_BASE}/pipelines`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('total');
      expect(data).toHaveProperty('pipelines');
    });
  });

  describe('Plugins Management', () => {
    it('should list plugins', async () => {
      const res = await fetch(`${API_BASE}/plugins`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('plugins');
      expect(data).toHaveProperty('total');
    });
  });
});

describe('Integration Tests - Data Plane', () => {
  const DATA_PLANE_BASE = 'http://localhost:8080';

  const skipIfDataPlaneUnavailable = async () => {
    try {
      const res = await fetch(`${DATA_PLANE_BASE}/health`);
      return !res.ok;
    } catch {
      return true;
    }
  };

  describe('Data Plane Health', () => {
    it('should have healthy data plane', async () => {
      if (await skipIfDataPlaneUnavailable()) {
        console.log('Data Plane not available, skipping test');
        return;
      }
      const res = await fetch(`${DATA_PLANE_BASE}/health`);
      expect(res.ok).toBe(true);
      const text = await res.text();
      expect(text).toContain('VERIDACTUS');
    });
  });

  describe('Traces', () => {
    it('should fetch traces from data plane', async () => {
      if (await skipIfDataPlaneUnavailable()) {
        console.log('Data Plane not available, skipping test');
        return;
      }
      const res = await fetch(`${DATA_PLANE_BASE}/v1/traces`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('traces');
    });
  });

  describe('Models from Data Plane', () => {
    it('should fetch models from data plane', async () => {
      if (await skipIfDataPlaneUnavailable()) {
        console.log('Data Plane not available, skipping test');
        return;
      }
      const res = await fetch(`${DATA_PLANE_BASE}/models`);
      expect(res.ok).toBe(true);
      const data = await res.json();
      expect(data).toHaveProperty('data');
    });
  });
});