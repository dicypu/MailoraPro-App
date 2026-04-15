// Pin feature — isolated module
import { store, ACTION, MSG_STATE } from '../store.js';
export function togglePin(id) {
    const state = store.getMsgState(id);
    store.dispatch({ type: state === MSG_STATE.PINNED ? ACTION.UNPIN_MESSAGE : ACTION.PIN_MESSAGE, payload: id });
}
