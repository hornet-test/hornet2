import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import App from './App';
import './style.css';
import './styles/components.css';

// OpenObserve RUM initialization (conditional based on environment)
const isOpenObserveEnabled = import.meta.env.VITE_OPENOBSERVE_ENABLED === 'true';
const isDevelopment = import.meta.env.MODE === 'development';

if (isOpenObserveEnabled) {
  // Dynamic import to avoid loading OpenObserve libraries when disabled
  Promise.all([import('@openobserve/browser-rum'), import('@openobserve/browser-logs')])
    .then(([{ openobserveRum }, { openobserveLogs }]) => {
      // Build configuration from environment variables
      const env = import.meta.env;

      // Type-safe privacy level extraction
      const privacyLevel = env.VITE_OPENOBSERVE_PRIVACY_LEVEL;
      const validPrivacyLevels = ['allow', 'mask-user-input', 'mask'] as const;
      const defaultPrivacyLevel: 'allow' | 'mask-user-input' | 'mask' =
        privacyLevel && validPrivacyLevels.includes(privacyLevel)
          ? privacyLevel
          : 'mask-user-input';

      const config = {
        clientToken: env.VITE_OPENOBSERVE_CLIENT_TOKEN ?? '',
        applicationId: env.VITE_OPENOBSERVE_APP_ID ?? 'hornet2-ui',
        site: env.VITE_OPENOBSERVE_ENDPOINT ?? `${window.location.host}/openobserve`,
        organizationIdentifier: env.VITE_OPENOBSERVE_ORG ?? 'default',
        service: env.VITE_OPENOBSERVE_SERVICE ?? 'hornet2-ui',
        env: env.VITE_OPENOBSERVE_ENV ?? (isDevelopment ? 'development' : 'production'),
        version: env.VITE_APP_VERSION ?? '0.1.0',
        apiVersion: env.VITE_OPENOBSERVE_API_VERSION ?? 'v1',
        insecureHTTP: isDevelopment,
        defaultPrivacyLevel,
      };

      // Validate required configuration
      if (!config.clientToken) {
        console.warn(
          'OpenObserve is enabled but VITE_OPENOBSERVE_CLIENT_TOKEN is not set. Skipping initialization.',
        );
        return;
      }

      // Initialize RUM (Real User Monitoring)
      openobserveRum.init({
        applicationId: config.applicationId,
        clientToken: config.clientToken,
        site: config.site,
        organizationIdentifier: config.organizationIdentifier,
        service: config.service,
        env: config.env,
        version: config.version,
        trackResources: true,
        trackLongTasks: true,
        trackUserInteractions: true,
        apiVersion: config.apiVersion,
        insecureHTTP: config.insecureHTTP,
        defaultPrivacyLevel: config.defaultPrivacyLevel,
      });

      // Initialize Logs
      openobserveLogs.init({
        clientToken: config.clientToken,
        site: config.site,
        organizationIdentifier: config.organizationIdentifier,
        service: config.service,
        env: config.env,
        version: config.version,
        forwardErrorsToLogs: true,
        insecureHTTP: config.insecureHTTP,
        apiVersion: config.apiVersion,
      });

      // Start session replay recording
      openobserveRum.startSessionReplayRecording();

      console.info(`OpenObserve RUM initialized for ${config.env} environment`);
    })
    .catch((error: unknown) => {
      console.error('Failed to initialize OpenObserve:', error);
    });
}

const rootElement = document.getElementById('root');

if (!rootElement) {
  throw new Error('Root element #root not found');
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>,
);
