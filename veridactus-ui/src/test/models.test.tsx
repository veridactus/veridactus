import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { I18nProvider } from '../i18n';
import Models from '../pages/Models';

vi.mock('../api', async () => {
  const mockModels = [
    {
      id: '1',
      name: 'DeepSeek-R1-14B',
      upstream_url: 'https://api.deepseek.com',
      upstream_model: 'deepseek-chat',
      is_default: true,
      supported_versions: ['v1', 'v2'],
      status: 'active',
    },
    {
      id: '2',
      name: 'Qwen2.5-14B',
      upstream_url: 'https://dashscope.aliyuncs.com',
      upstream_model: 'qwen-turbo',
      is_default: false,
      supported_versions: ['v1'],
      status: 'active',
    },
  ];
  return {
    getModelsConfig: vi.fn().mockResolvedValue(mockModels),
    updateModel: vi.fn().mockResolvedValue({ success: true }),
    createModel: vi.fn().mockResolvedValue({ id: '3', name: 'New Model' }),
    deleteModel: vi.fn().mockResolvedValue({ success: true }),
  };
});

vi.mock('../i18n', async () => {
  return {
    useI18n: vi.fn().mockReturnValue({
      t: (key: string) => key,
      locale: 'en',
    }),
    I18nProvider: ({ children }: { children: React.ReactNode }) => children,
  };
});

describe('Models Page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders page header correctly', async () => {
    render(
      <BrowserRouter>
        <I18nProvider>
          <Models />
        </I18nProvider>
      </BrowserRouter>
    );
    await waitFor(() => {
      expect(screen.getByText('models.title')).toBeInTheDocument();
    });
  });

  it('shows add model button', async () => {
    render(
      <BrowserRouter>
        <I18nProvider>
          <Models />
        </I18nProvider>
      </BrowserRouter>
    );
    await waitFor(() => {
      expect(screen.getByText('models.add')).toBeInTheDocument();
    });
  });

  it('displays model cards when data loads', async () => {
    render(
      <BrowserRouter>
        <I18nProvider>
          <Models />
        </I18nProvider>
      </BrowserRouter>
    );
    await waitFor(() => {
      expect(screen.getByText('DeepSeek-R1-14B')).toBeInTheDocument();
      expect(screen.getByText('Qwen2.5-14B')).toBeInTheDocument();
    });
  });

  it('displays model details correctly', async () => {
    render(
      <BrowserRouter>
        <I18nProvider>
          <Models />
        </I18nProvider>
      </BrowserRouter>
    );
    await waitFor(() => {
      expect(screen.getByText('https://api.deepseek.com')).toBeInTheDocument();
      expect(screen.getByText('https://dashscope.aliyuncs.com')).toBeInTheDocument();
    });
  });

  it('shows model status badges', async () => {
    render(
      <BrowserRouter>
        <I18nProvider>
          <Models />
        </I18nProvider>
      </BrowserRouter>
    );
    await waitFor(() => {
      expect(screen.getAllByText('models.active').length).toBeGreaterThan(0);
    });
  });
});