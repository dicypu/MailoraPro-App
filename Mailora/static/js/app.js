// Mailora v2 — App Entry Point
import { store, ACTION } from './store.js';
import { dataSource } from './data-source.js';
import { mountSidebar } from './components/sidebar.js';
import { mountMessageList } from './components/message-list.js';
import { mountPreview } from './components/message-preview.js';
import { mountCompose, handleFileInput, sendEmail, closeCompose } from './components/compose-modal.js?v=2';
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
    try {
        const accounts = await dataSource.getAccounts();
        store.dispatch({ type: ACTION.SET_ACCOUNTS, payload: accounts });

        if (accounts.length > 0) {
            // Add unified as the default account view
            store.dispatch({ type: ACTION.SET_FOLDERS, payload: ['INBOX', 'Sent', 'Drafts', 'Spam', 'Trash'] });
            store.dispatch({ type: ACTION.SELECT_ACCOUNT, payload: 'unified' });

            const messages = await dataSource.getUnifiedInbox('INBOX', 200);
            store.dispatch({ type: ACTION.SET_MESSAGES, payload: messages });
        }
    } catch (e) {
        console.error("Init error:", e);
    }
    // Apply saved theme
    document.documentElement.setAttribute('data-theme', store.getState().theme);
}

// Watch store changes to reload data
let lastAcc = null;
let lastFold = null;
let fetchGeneration = 0;

store.subscribe('selectedAccountId', async (accId) => {
    if (!accId || accId === lastAcc) return;
    lastAcc = accId;
    
    const currentGen = ++fetchGeneration;
    
    // Always reset to INBOX when switching accounts
    store.dispatch({ type: ACTION.SELECT_FOLDER, payload: 'INBOX' });
    lastFold = 'INBOX'; 

    if (accId === 'unified') {
        store.dispatch({ type: ACTION.SET_FOLDERS, payload: ['INBOX', 'Sent', 'Drafts', 'Trash', 'Spam', 'Outbox'] });
        const msgs = await dataSource.getUnifiedInbox('INBOX');
        if (currentGen === fetchGeneration) store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
    } else {
        const folders = await dataSource.getFolders(accId);
        if (!folders.includes('Outbox')) {
            folders.push('Outbox');
        }
        if (currentGen === fetchGeneration) store.dispatch({ type: ACTION.SET_FOLDERS, payload: folders });
        const msgs = await dataSource.getMessages(accId, 'INBOX');
        if (currentGen === fetchGeneration) store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
    }
});

store.subscribe('selectedFolder', async (folder) => {
    if (!folder || folder === lastFold) return;
    lastFold = folder;
    
    const currentGen = ++fetchGeneration;
    const accId = store.getState().selectedAccountId;
    
    if (accId) {
        if (accId === 'unified') {
            const msgs = await dataSource.getUnifiedInbox(folder);
            if (currentGen === fetchGeneration) store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
        } else {
            const msgs = await dataSource.getMessages(accId, folder);
            if (currentGen === fetchGeneration) store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
        }
    }
});

// Sync handler
async function handleSync() {
    const accId = store.getState().selectedAccountId;
    if (!accId) return;
    try {
        if (accId === 'unified') {
            const accounts = store.getState().accounts;
            for (const acc of accounts) { await dataSource.syncAccount(acc.id).catch(() => { }); }
            const msgs = await dataSource.getUnifiedInbox(store.getState().selectedFolder);
            store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
        } else {
            await dataSource.syncAccount(accId);
            const msgs = await dataSource.getMessages(accId, store.getState().selectedFolder);
            store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
        }
    } catch (e) { console.error("Sync error:", e); }
}

init();

// Global event bindings
let searchTimeout;
window.mailora = {
    logout: () => { localStorage.removeItem('auth_token'); window.location.href = '/static/login.html'; },
    selectFolder: (f) => store.dispatch({ type: ACTION.SELECT_FOLDER, payload: f }),
    selectAccount: (id) => store.dispatch({ type: ACTION.SELECT_ACCOUNT, payload: id }),
    compose: () => store.dispatch({ type: ACTION.TOGGLE_COMPOSE }),
    closeCompose,
    sendEmail,
    handleFileInput,
    toggleAnalytics: () => store.dispatch({ type: ACTION.TOGGLE_ANALYTICS }),
    toggleTheme: () => store.dispatch({ type: ACTION.TOGGLE_THEME }),
    toggleFocus,
    search: (q) => {
        store.dispatch({ type: ACTION.SET_SEARCH, payload: q });
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(async () => {
            if (q.trim().length > 0) {
                try {
                    const s = store.getState();
                    const results = await api.searchMessages(q.trim(), { accountId: s.selectedAccountId, folder: s.selectedFolder });
                    store.dispatch({ type: ACTION.SET_SEARCH_RESULTS, payload: results });
                } catch (e) {
                    console.error('Search failed:', e);
                }
            } else {
                store.dispatch({ type: ACTION.SET_SEARCH_RESULTS, payload: null });
            }
        }, 300);
    },
    sync: handleSync,
};

// Keyboard shortcuts
document.addEventListener('keydown', e => {
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    const msgs = store.getVisibleMessages();
    const s = store.getState();
    const idx = msgs.findIndex(m => m.id === s.selectedMessageId);
    if (e.key === 'j' && idx < msgs.length - 1) store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: msgs[idx + 1]?.id });
    else if (e.key === 'k' && idx > 0) store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: msgs[idx - 1]?.id });
    else if (e.key === 'c') store.dispatch({ type: ACTION.TOGGLE_COMPOSE });
    else if (e.key === '/' || e.key === 'f') { e.preventDefault(); document.getElementById('search')?.focus(); }
});
