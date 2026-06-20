import React from 'react';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: string;
}

/**
 * VERIDACTUS ErrorBoundary — 前端结构化错误捕获
 * 
 * 功能：
 * 1. 捕获渲染错误，防止整个页面白屏
 * 2. 结构化记录错误到 console（JSON 格式，便于日志采集）
 * 3. 显示友好的降级 UI
 */
export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: '' };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    const errorJson = {
      timestamp: new Date().toISOString(),
      type: 'react-error-boundary',
      error: {
        name: error.name,
        message: error.message,
        stack: error.stack?.split('\n').slice(0, 5).join('\n'),
      },
      componentStack: errorInfo.componentStack?.split('\n').slice(0, 3).join('\n'),
    };
    console.error(JSON.stringify(errorJson));
    this.setState({ errorInfo: errorInfo.componentStack || '' });
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null, errorInfo: '' });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;

      return (
        <div style={{ padding: 40, textAlign: 'center', fontFamily: 'system-ui' }}>
          <h2 style={{ color: '#ff7675', marginBottom: 16 }}>
            ⚠️ VERIDACTUS UI Error
          </h2>
          <p style={{ color: '#b2bec3', fontSize: 14 }}>
            {this.state.error?.message || 'An unexpected error occurred'}
          </p>
          <pre style={{
            maxWidth: 600, margin: '16px auto', padding: 16,
            background: '#2d3436', color: '#dfe6e9', borderRadius: 8,
            fontSize: 12, textAlign: 'left', overflow: 'auto',
            whiteSpace: 'pre-wrap',
          }}>
            {this.state.errorInfo || this.state.error?.stack || 'No details'}
          </pre>
          <button
            onClick={this.handleReset}
            style={{
              marginTop: 16, padding: '8px 24px',
              background: '#00d4aa', color: '#000', border: 'none',
              borderRadius: 6, cursor: 'pointer', fontSize: 14,
            }}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

/**
 * 结构化前端日志工具
 * 替代裸 console.log，输出 JSON 格式便于日志平台采集
 */
export const structuredLog = {
  info: (message: string, data?: Record<string, unknown>) => {
    console.log(JSON.stringify({ timestamp: new Date().toISOString(), level: 'info', message, ...data }));
  },
  warn: (message: string, data?: Record<string, unknown>) => {
    console.warn(JSON.stringify({ timestamp: new Date().toISOString(), level: 'warn', message, ...data }));
  },
  error: (message: string, data?: Record<string, unknown>) => {
    console.error(JSON.stringify({ timestamp: new Date().toISOString(), level: 'error', message, ...data }));
  },
};
