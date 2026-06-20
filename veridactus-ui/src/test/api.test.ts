import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const mockFetch = vi.fn();
global.fetch = mockFetch;

const API_BASE = '/api/v1';

describe('API Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  describe('getApiKeys', () => {
    it('fetches API keys successfully', async () => {
      const mockResponse = {
        total: 2,
        keys: [
          { id: '1', name: 'Test Key', key: 'vk_xxx', tenant_id: 't1', status: 'active', created_at: '2024-01-01' },
        ],
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      });

      const { getApiKeys } = await import('../api');
      const result = await getApiKeys();

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/apikeys`);
      expect(result).toEqual(mockResponse.keys);
    });

    it('handles fetch error', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      const { getApiKeys } = await import('../api');
      await expect(getApiKeys()).rejects.toThrow('Network error');
    });

    it('handles non-ok response', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
      });

      const { getApiKeys } = await import('../api');
      await expect(getApiKeys()).rejects.toThrow('401');
    });
  });

  describe('createApiKey', () => {
    it('creates API key successfully', async () => {
      const newKeyName = 'New Key';
      const mockResponse = {
        id: '3',
        name: 'New Key',
        key: 'vk_new123',
        tenant_id: 't1',
        status: 'active',
        created_at: '2024-01-15',
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      });

      const { createApiKey } = await import('../api');
      const result = await createApiKey(newKeyName);

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/apikeys`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: newKeyName, tenant_id: 'default' }),
      });
      expect(result).toEqual(mockResponse);
    });
  });

  describe('rotateApiKey', () => {
    it('rotates API key successfully', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      const { rotateApiKey } = await import('../api');
      const result = await rotateApiKey('1');

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/apikeys/1`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ status: 'rotated' }),
      });
      expect(result).toBe(true);
    });
  });

  describe('deleteApiKey', () => {
    it('deletes API key successfully', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
      });

      const { deleteApiKey } = await import('../api');
      const result = await deleteApiKey('1');

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/apikeys/1`, {
        method: 'DELETE',
      });
      expect(result).toBe(true);
    });
  });

  describe('getModelsConfig', () => {
    it('fetches models config successfully', async () => {
      const mockResponse = {
        total: 1,
        models: [
          { id: '1', name: 'DeepSeek-R1', upstream_url: 'https://api.deepseek.com', status: 'active' },
        ],
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      });

      const { getModelsConfig } = await import('../api');
      const result = await getModelsConfig();

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/models`);
      expect(result).toEqual(mockResponse.models);
    });
  });

  describe('updateModel', () => {
    it('updates model successfully', async () => {
      const updates = { name: 'Updated Model', status: 'inactive' };
      const mockResponse = { id: '1', ...updates };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResponse),
      });

      const { updateModel } = await import('../api');
      const result = await updateModel('1', updates);

      expect(mockFetch).toHaveBeenCalledWith(`${API_BASE}/models/1`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
      });
      expect(result).toEqual(mockResponse);
    });
  });
});