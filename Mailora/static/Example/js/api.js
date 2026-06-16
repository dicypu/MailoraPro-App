// Mailora v2 — Real API (Production Mode)
import { CONFIG } from './config.js';
function authHeaders() {
    const h = { 'Content-Type': 'application/json' };
    const t = localStorage.getItem('auth_token');
    if (t) h['Authorization'] = t;
    return h;
}
async function apiFetch(url, opts = {}) {
    opts.headers = { ...authHeaders(), ...opts.headers };
    const res = await fetch(CONFIG.apiBase + url, opts);
    if (res.status === 401) { localStorage.removeItem('auth_token'); window.location.href = '/static/index.html#login'; throw new Error('Unauthorized'); }
    if (res.status === 413) throw new Error('Dosya çok büyük (maks. 10MB)');
    if (!res.ok) { const e = await res.json().catch(() => ({})); throw new Error(e.error || res.statusText); }
    return res;
}
export async function getAccounts() { return (await apiFetch('/accounts')).json(); }
export async function getMessages(accountId, folder) { return (await apiFetch(`/messages?account=${accountId||''}&folder=${folder||'Inbox'}`)).json(); }
export async function getMessage(id) { return (await apiFetch(`/messages/${id}`)).json(); }
export async function sendMessage(data) {
    if (data.attachments?.length) {
        const fd = new FormData();
        fd.append('to', data.to); fd.append('subject', data.subject); fd.append('body', data.body);
        if (data.accountId) fd.append('account_id', data.accountId);
        data.attachments.forEach(f => fd.append('files', f));
        const h = {}; const t = localStorage.getItem('auth_token'); if (t) h['Authorization'] = t;
        return (await fetch(CONFIG.apiBase + '/send', { method: 'POST', headers: h, body: fd })).json();
    }
    return (await apiFetch('/send', { method: 'POST', body: JSON.stringify(data) })).json();
}
export async function login(username, password) { return (await apiFetch('/auth/login', { method:'POST', body:JSON.stringify({username,password}) })).json(); }
export async function register(username, password) { return (await apiFetch('/auth/register', { method:'POST', body:JSON.stringify({username,password}) })).json(); }
export async function getFolders() { return (await apiFetch('/folders')).json(); }
export async function getSettings() { return (await apiFetch('/settings')).json(); }
