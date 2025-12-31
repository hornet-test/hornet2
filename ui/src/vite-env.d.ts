/// <reference types="vite/client" />

interface ImportMetaEnv {
  // OpenObserve configuration
  readonly VITE_OPENOBSERVE_ENABLED?: string;
  readonly VITE_OPENOBSERVE_ENDPOINT?: string;
  readonly VITE_OPENOBSERVE_CLIENT_TOKEN?: string;
  readonly VITE_OPENOBSERVE_ORG?: string;
  readonly VITE_OPENOBSERVE_APP_ID?: string;
  readonly VITE_OPENOBSERVE_SERVICE?: string;
  readonly VITE_OPENOBSERVE_ENV?: string;
  readonly VITE_OPENOBSERVE_API_VERSION?: string;
  readonly VITE_OPENOBSERVE_PRIVACY_LEVEL?: 'allow' | 'mask-user-input' | 'mask';
  readonly VITE_APP_VERSION?: string;
  // Standard Vite variables
  readonly MODE: string;
  readonly BASE_URL: string;
  readonly PROD: boolean;
  readonly DEV: boolean;
  readonly SSR: boolean;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
