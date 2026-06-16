// Mailora v2 — App Entry Point
import { store, ACTION } from './store.js';
import { dataSource } from './data-source.js';
import { mountSidebar } from './components/sidebar.js';
import { mountMessageList } from './components/message-list.js';
import { mountPreview } from './components/message-preview.js';
import { mountCompose, handleFileInput, sendEmail, closeCompose } from './components/compose-modal.js';
import { mountAnalytics } from './components/analytics.js';
import { toggleFocus } from './features/focus.js';

// Mount all components
mountSidebar();
mountMessageList();
mountPreview();
mountCompose();
mountAnalytics();

// Load initial data
async function init() {
    const accounts = await dataSource.getAccounts();
    store.dispatch({ type: ACTION.SET_ACCOUNTS, payload: accounts });
    const messages = await dataSource.getMessages();
    store.dispatch({ type: ACTION.SET_MESSAGES, payload: messages });
    // Apply saved theme
    document.documentElement.setAttribute('data-theme', store.getState().theme);
}
init();

// Global event bindings
window.mailora = {
    compose: () => store.dispatch({ type: ACTION.TOGGLE_COMPOSE }),
    closeCompose,
    sendEmail,
    handleFileInput,
    toggleAnalytics: () => store.dispatch({ type: ACTION.TOGGLE_ANALYTICS }),
    toggleTheme: () => store.dispatch({ type: ACTION.TOGGLE_THEME }),
    toggleFocus,
    search: (q) => store.dispatch({ type: ACTION.SET_SEARCH, payload: q }),
};

// Keyboard shortcuts
document.addEventListener('keydown', e => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    const msgs = store.getVisibleMessages();
    const s = store.getState();
    const idx = msgs.findIndex(m => m.id === s.selectedMessageId);
    if (e.key === 'j' && idx < msgs.length - 1) store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: msgs[idx+1]?.id });
    else if (e.key === 'k' && idx > 0) store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: msgs[idx-1]?.id });
    else if (e.key === 'c') store.dispatch({ type: ACTION.TOGGLE_COMPOSE });
    else if (e.key === '/' || e.key === 'f') { e.preventDefault(); document.getElementById('search')?.focus(); }
});
