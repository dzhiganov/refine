import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MotionConfig } from 'motion/react';
import { App } from './App';
import { initApi } from './lib/api';
import { setPlatform } from './lib/platform';
import { useApp } from './stores/app';
import './styles/global.css';

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false, retry: false } },
});

async function start() {
  const api = await initApi();
  const boot = await api.bootstrap();
  setPlatform(boot.platform);
  useApp.getState().init(boot);
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <MotionConfig reducedMotion="user" transition={{ type: 'spring', stiffness: 380, damping: 34, mass: 0.9 }}>
          <App />
        </MotionConfig>
      </QueryClientProvider>
    </StrictMode>,
  );
}

void start();
