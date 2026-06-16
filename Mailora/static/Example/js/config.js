// Mailora v2 — Configuration
export const CONFIG = {
    mode: localStorage.getItem('mailora-mode') || 'demo',
    apiBase: '',
    maxFileSize: 10 * 1024 * 1024,
    maxAttachments: 10,
    snoozeOptions: [
        { label: '1 saat', ms: 3600000 },
        { label: '4 saat', ms: 14400000 },
        { label: 'Yarın', ms: 86400000 },
        { label: 'Gelecek hafta', ms: 604800000 },
    ],
    supportedLanguages: [
        { code: 'en', label: 'English' },
        { code: 'de', label: 'Deutsch' },
        { code: 'fr', label: 'Français' },
        { code: 'es', label: 'Español' },
        { code: 'ar', label: 'العربية' },
    ],
    get isDemo() { return this.mode === 'demo'; },
    setMode(m) { this.mode = m; localStorage.setItem('mailora-mode', m); }
};
