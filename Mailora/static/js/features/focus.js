// Focus mode feature
import { store, ACTION } from '../store.js';
export function toggleFocus() { store.dispatch({ type: ACTION.TOGGLE_FOCUS }); }
export function isFocused() { return store.getState().focusMode; }
