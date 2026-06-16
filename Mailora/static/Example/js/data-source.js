// Mailora v2 — Data Source Switcher
import { CONFIG } from './config.js';
let ds;
if (CONFIG.isDemo) {
    ds = await import('./mock.js');
} else {
    ds = await import('./api.js');
}
export const dataSource = ds;
