// Omschrift brand + app config (whitelabel of Odysseus)
export const BRAND = {
  name: 'Omschrift Inventions',
  appTitle: 'Omschrift',
  // Colors sampled from omsinv.com
  primary: '#2563eb', // brand blue
  primaryDark: '#1d4ed8',
  accent: '#7c3aed', // purple
  accentGreen: '#4ade80',
  slate: '#1e293b', // dark slate background
  slate2: '#0f172a',
  textLight: '#ffffff',
  textMuted: '#cbd5e1',
};

// Odysseus backend URLs.
export const URL_PRESETS = [
  { label: 'Server (LAN)', url: 'http://187.127.137.38:7000/' },
  { label: 'Localhost', url: 'http://localhost:7000/' },
];

export const DEFAULT_URL = URL_PRESETS[0].url;

export const STORAGE_KEY = '@omschrift/odysseus_url';
