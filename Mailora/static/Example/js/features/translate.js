// Translate feature — isolated module
import { CONFIG } from '../config.js';
export function getLanguages() { return CONFIG.supportedLanguages; }
export async function translateText(text, targetLang) {
    // In demo mode, simulate translation
    if (CONFIG.isDemo) {
        await new Promise(r => setTimeout(r, 500));
        return `[${targetLang.toUpperCase()} çevirisi] ${text}`;
    }
    // Production: call translation API
    const res = await fetch(`${CONFIG.apiBase}/translate`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text, target: targetLang })
    });
    return (await res.json()).translated;
}
