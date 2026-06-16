// Mailora v2 — Sidebar Component
import { store, ACTION } from '../store.js';
const el = id => document.getElementById(id);
export function mountSidebar() {
    store.subscribe('accounts', renderAccounts);
    store.subscribe('selectedAccountId', renderAccounts);
    store.subscribe('folders', renderFolders);
    store.subscribe('selectedFolder', renderFolders);
    store.subscribe('focusMode', renderFocusBtn);
    store.subscribe('theme', renderThemeBtn);
}
function renderAccounts() {
    const s = store.getState();
    const c = el('account-list');
    if (!c) return;
    c.innerHTML = s.accounts.map(a => `
        <div class="account-item ${s.selectedAccountId===a.id?'active':''}" data-id="${a.id}">
            <div class="account-dot" style="background:${a.color}"></div>
            <span class="account-name">${a.displayName||a.email}</span>
        </div>
    `).join('');
    c.querySelectorAll('.account-item').forEach(el => {
        el.onclick = () => store.dispatch({ type: ACTION.SELECT_ACCOUNT, payload: el.dataset.id });
    });
}
function renderFolders() {
    const s = store.getState();
    const c = el('folder-list');
    if (!c) return;
    const icons = { Inbox:'📥', Sent:'📤', Drafts:'📝', Spam:'⚠️', Trash:'🗑️' };
    c.innerHTML = s.folders.map(f => `
        <div class="folder-item ${s.selectedFolder===f?'active':''}" data-folder="${f}">
            <span>${icons[f]||'📁'}</span><span>${f}</span>
        </div>
    `).join('');
    c.querySelectorAll('.folder-item').forEach(el => {
        el.onclick = () => store.dispatch({ type: ACTION.SELECT_FOLDER, payload: el.dataset.folder });
    });
}
function renderFocusBtn() {
    const b = el('focus-btn');
    if (b) b.textContent = store.getState().focusMode ? '🎯 Focus: ON' : '🎯 Focus';
}
function renderThemeBtn() {
    const b = el('theme-btn');
    if (b) b.textContent = store.getState().theme === 'dark' ? '☀️' : '🌙';
}
