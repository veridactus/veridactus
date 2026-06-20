import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { lazy, Suspense } from 'react';
import { I18nProvider } from './i18n';
import { useThemeStore } from './store';
import Sidebar from './components/layout/Sidebar';
import BottomStatusBar from './components/layout/BottomStatusBar';
import DataFlowBackground from './components/viz/DataFlowBackground';
import { useMetricsStream } from './hooks/useMetricsStream';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const Pipelines = lazy(() => import('./pages/Pipelines'));
const PipelineEdit = lazy(() => import('./pages/PipelineEdit'));
const PipelineDesigner = lazy(() => import('./pages/PipelineDesigner'));
const AuditCenter = lazy(() => import('./pages/AuditCenter'));
const Plugins = lazy(() => import('./pages/Plugins'));
const ApiKeys = lazy(() => import('./pages/ApiKeys'));
const Models = lazy(() => import('./pages/Models'));
const Settings = lazy(() => import('./pages/Settings'));

function LoadingScreen() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: 'var(--bg-primary)' }}>
      <div style={{ textAlign: 'center' }}>
        <div style={{ width: 40, height: 40, border: '3px solid var(--border-default)', borderTopColor: '#00d4aa', borderRadius: '50%', animation: 'spin 0.8s linear infinite', margin: '0 auto 16px' }} />
        <p style={{ color: 'var(--text-secondary)', fontSize: 14, fontWeight: 500 }}>Loading VERIDACTUS...</p>
      </div>
      <style>{'@keyframes spin { to { transform: rotate(360deg); } }'}</style>
    </div>
  );
}

function AppLayout() {
  const theme = useThemeStore(s => s.theme);
  useMetricsStream();

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  return (
    <div style={{ display: 'flex', minHeight: '100vh', background: 'var(--bg-primary)', transition: 'background 0.3s ease' }}>
      <DataFlowBackground />
      <Sidebar />
      <main className="main-content">
        <div className="content-area">
          <Suspense fallback={<LoadingScreen />}>
            <Routes>
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/pipelines" element={<Pipelines />} />
              <Route path="/pipelines/new" element={<PipelineDesigner />} />
              <Route path="/pipelines/design/:id?" element={<PipelineDesigner />} />
              <Route path="/pipelines/edit/:id" element={<PipelineEdit />} />
              <Route path="/audit" element={<AuditCenter />} />
              <Route path="/plugins" element={<Plugins />} />
              <Route path="/api-keys" element={<ApiKeys />} />
              <Route path="/models" element={<Models />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Routes>
          </Suspense>
        </div>
      </main>
      <BottomStatusBar />
    </div>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <I18nProvider>
        <AppLayout />
      </I18nProvider>
    </BrowserRouter>
  );
}
