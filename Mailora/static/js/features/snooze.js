// Snooze feature — isolated module
import { store, ACTION } from '../store.js';
import { CONFIG } from '../config.js';
export function snoozeMessage(id, durationMs) {
    store.dispatch({ type: ACTION.SNOOZE_MESSAGE, payload: { id, until: Date.now() + durationMs } });
}
export function unsnoozeMessage(id) {
    store.dispatch({ type: ACTION.UNSNOOZE_MESSAGE, payload: id });
}
export function getSnoozeOptions() { return CONFIG.snoozeOptions; }
// Auto-wake check every 30 seconds
setInterval(() => { store.dispatch({ type: 'TICK' }); }, 30000);
