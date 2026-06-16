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
    if (res.status === 401) {
        localStorage.removeItem('auth_token');
        window.location.href = '/static/login.html';
        throw new Error('Unauthorized');
    }
    if (res.status === 413) throw new Error('Dosya çok büyük (maks. 10MB)');
    if (!res.ok) {
        const e = await res.json().catch(() => ({}));
        throw new Error(e.error || res.statusText);
    }
    return res;
}

// ─── RFC 2047 Decoding & UTF-8 Fix ──────────────────────────────────
function decodeBytes(bytes, cs) {
    try { return new TextDecoder((cs || 'utf-8').toLowerCase()).decode(bytes); }
    catch { try { return new TextDecoder('utf-8').decode(bytes); } catch { return String.fromCharCode(...bytes); } }
}
function fromBase64ToBytes(b64) {
    try { const bin = atob(b64.replace(/\s/g, '')); const arr = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i) & 255; return arr; }
    catch { return new Uint8Array(); }
}
function fromQPToBytes(txt) {
    const clean = txt.replace(/_/g, ' '); const bytes = [];
    for (let i = 0; i < clean.length; i++) {
        const c = clean[i];
        if (c === '=' && /[0-9A-Fa-f]{2}/.test(clean.slice(i + 1, i + 3))) { bytes.push(parseInt(clean.slice(i + 1, i + 3), 16)); i += 2; }
        else { bytes.push(clean.charCodeAt(i) & 255); }
    }
    return new Uint8Array(bytes);
}
export function decodeRfc2047(str) {
    if (!str) return '';
    return str.replace(/=\?([^?]+)\?([BQbq])\?([^?]+)\?=/g, (_, cs, mode, payload) => {
        if (/[bB]/.test(mode)) return decodeBytes(fromBase64ToBytes(payload), cs);
        else return decodeBytes(fromQPToBytes(payload), cs);
    });
}
export function tryFixUtf8Mojibake(s) {
    if (!s) return s;
    if (/[ÃÄÅÂØÐÞ]/.test(s)) {
        try { const bytes = new Uint8Array(Array.from(s, ch => ch.charCodeAt(0) & 255)); return new TextDecoder('utf-8').decode(bytes); }
        catch { return s; }
    }
    return s;
}
export function fixText(s) { return tryFixUtf8Mojibake(decodeRfc2047(s || '')); }

// ─── Folder Resolution ──────────────────────────────────────────────
const folderCache = {};
const folderMappings = {};

async function ensureFoldersLoaded(accountId) {
    if (folderCache[accountId]) return;
    try {
        const r = await apiFetch(`/test/folders/${encodeURIComponent(accountId)}`);
        const list = await r.json();
        folderCache[accountId] = list;
        const map = {};
        const find = (flags, names) => {
            const byFlag = list.find(f => f.flags.some(fl => flags.some(target => fl.toLowerCase() === target.toLowerCase())));
            if (byFlag) return byFlag.name;
            const byName = list.find(f => { const n = f.name.toLowerCase(); return names.some(target => n === target || n.endsWith('/' + target) || n.endsWith('.' + target)); });
            if (byName) return byName.name;
            const byFuzzy = list.find(f => names.some(target => f.name.toLowerCase().includes(target)));
            return byFuzzy ? byFuzzy.name : null;
        };
        map['INBOX'] = 'INBOX';
        map['Inbox'] = 'INBOX';
        map['Sent'] = find(['\\Sent'], ['sent', 'sent items', 'sent mail', 'gönderilen']);
        map['Drafts'] = find(['\\Drafts'], ['drafts', 'taslak']);
        map['Trash'] = find(['\\Trash'], ['trash', 'deleted', 'bin', 'çöp']);
        map['Spam'] = find(['\\Junk', '\\Spam'], ['spam', 'junk', 'gereksiz']);
        folderMappings[accountId] = map;
    } catch (e) {
        console.error("Folder resolution failed:", e);
        folderCache[accountId] = [];
    }
}

export async function resolveFolderName(accountId, folder) {
    const generics = ['Inbox', 'INBOX', 'Sent', 'Drafts', 'Trash', 'Spam'];
    if (generics.includes(folder)) {
        await ensureFoldersLoaded(accountId);
        const map = folderMappings[accountId];
        if (map && map[folder]) return map[folder];
    }
    return folder;
}

// ─── Accounts ────────────────────────────────────────────────────────
export async function getAccounts() {
    return (await apiFetch('/accounts')).json();
}

// ─── Folders ─────────────────────────────────────────────────────────
export async function getFolders(accountId) {
    if (!accountId) return ['Inbox', 'Sent', 'Drafts', 'Spam', 'Trash'];
    try {
        const r = await apiFetch(`/test/folders/${encodeURIComponent(accountId)}`);
        const list = await r.json();
        return list.map(f => f.name);
    } catch {
        return ['Inbox', 'Sent', 'Drafts', 'Spam', 'Trash'];
    }
}

// ─── Messages ────────────────────────────────────────────────────────
export async function getMessages(accountId, folder) {
    if (!accountId) return [];
    if (folder === 'Outbox') {
        const r = await apiFetch(`/outbox?account_id=${encodeURIComponent(accountId)}`);
        const data = await r.json();
        return (data.messages || []).map(m => ({
            id: `outbox_${m.id}`,
            uid: m.id,
            accountId: m.account_id,
            from: fixText('Kuyruktaki E-posta'),
            email: m.to_addr,
            subject: fixText(m.subject || '(Konu Yok)'),
            preview: `[${m.status.toUpperCase()}] Alıcı: ${m.to_addr}. ${m.last_error ? 'Hata: ' + m.last_error : ''}`,
            date: new Date(m.created_at * 1000).toISOString(),
            folder: 'Outbox',
            read: true,
            hasAttachment: false,
            flags: '',
            important: false,
            isOutbox: true,
            outboxStatus: m.status,
            outboxError: m.last_error,
            toAddr: m.to_addr,
            body: m.body
        }));
    }
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    const r = await apiFetch(`/messages/${encodeURIComponent(accountId)}/${encodeURIComponent(resolvedFolder)}`);
    const data = await r.json();
    return (data.messages || []).map(m => ({
        id: `${accountId}_${m.uid}`,
        uid: Number(m.uid),
        accountId: accountId,
        from: fixText(m.from_addr || ''),
        email: m.from_addr || '',
        subject: fixText(m.subject || ''),
        preview: fixText(m.subject || ''),
        date: m.date || new Date().toISOString(),
        folder: resolvedFolder,
        read: (m.flags || '').includes('\\Seen'),
        hasAttachment: !!m.has_attachments,
        flags: m.flags || '',
        important: (m.flags || '').includes('\\Flagged'),
    }));
}

// ─── Unified Inbox ───────────────────────────────────────────────────
export async function getUnifiedInbox(folder, limit) {
    const f = folder || 'INBOX';
    if (f === 'Outbox') {
        const r = await apiFetch(`/outbox`);
        const data = await r.json();
        return (data.messages || []).map(m => ({
            id: `outbox_${m.id}`,
            uid: m.id,
            accountId: m.account_id,
            from: fixText('Kuyruktaki E-posta'),
            email: m.to_addr,
            subject: fixText(m.subject || '(Konu Yok)'),
            preview: `[${m.status.toUpperCase()}] Alıcı: ${m.to_addr}. ${m.last_error ? 'Hata: ' + m.last_error : ''}`,
            date: new Date(m.created_at * 1000).toISOString(),
            folder: 'Outbox',
            read: true,
            hasAttachment: false,
            flags: '',
            important: false,
            isOutbox: true,
            outboxStatus: m.status,
            outboxError: m.last_error,
            toAddr: m.to_addr,
            body: m.body
        }));
    }
    const l = limit || 200;
    const r = await apiFetch(`/unified/inbox?folder=${encodeURIComponent(f)}&limit=${l}`);
    const data = await r.json();
    return (data.messages || []).map(m => ({
        id: `${m.account_id}_${m.uid}`,
        uid: Number(m.uid),
        accountId: m.account_id,
        from: fixText(m.from_addr || ''),
        email: m.from_addr || '',
        subject: fixText(m.subject || ''),
        preview: fixText(m.subject || ''),
        date: m.date || new Date().toISOString(),
        folder: f,
        read: (m.flags || '').includes('\\Seen'),
        hasAttachment: !!m.has_attachments,
        flags: m.flags || '',
        important: (m.flags || '').includes('\\Flagged'),
    }));
}

// ─── Single Message Body ─────────────────────────────────────────────
export async function getMessage(accountId, uid, folder, signal) {
    if (folder === 'Outbox') {
        const r = await apiFetch(`/outbox/${uid}`);
        const body = await r.json();
        return {
            subject: fixText(body.subject || ''),
            from: fixText('Giden Kutusu Kuyruğu'),
            html_body: '',
            plain_text: body.body || '',
            date: new Date(body.created_at * 1000).toISOString(),
        };
    }
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    const r = await apiFetch(`/test/body/${encodeURIComponent(accountId)}/${uid}?folder=${encodeURIComponent(resolvedFolder)}`, { signal });
    const body = await r.json();
    return {
        subject: fixText(body.subject || ''),
        from: fixText(body.from || ''),
        html_body: body.html_body || '',
        plain_text: body.plain_text || '',
        date: body.date || '',
    };
}

// ─── Attachments ─────────────────────────────────────────────────────
export async function getAttachments(accountId, uid, folder) {
    if (folder === 'Outbox') {
        return [];
    }
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    const r = await apiFetch(`/attachments?accountId=${encodeURIComponent(accountId)}&uid=${encodeURIComponent(uid)}&folder=${encodeURIComponent(resolvedFolder)}`);
    return r.json();
}

export function getAttachmentDownloadUrl(accountId, uid, partId, folder) {
    return `/attachments/download?accountId=${encodeURIComponent(accountId)}&uid=${encodeURIComponent(uid)}&part=${encodeURIComponent(partId)}&folder=${encodeURIComponent(folder)}`;
}

// ─── Flags ───────────────────────────────────────────────────────────
export async function updateFlags(accountId, folder, uid, flags) {
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    return apiFetch(`/messages/${encodeURIComponent(accountId)}/${encodeURIComponent(resolvedFolder)}/${uid}/flags`, {
        method: 'POST', body: JSON.stringify(flags)
    });
}

// ─── Sync ────────────────────────────────────────────────────────────
export async function syncAccount(accountId) {
    return apiFetch(`/sync/${encodeURIComponent(accountId)}`, { method: 'POST' });
}

// ─── Snooze ──────────────────────────────────────────────────────────
export async function snoozeMessage(accountId, folder, uid, until) {
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    return apiFetch(`/snooze/${encodeURIComponent(accountId)}/${encodeURIComponent(resolvedFolder)}/${uid}`, {
        method: 'POST', body: JSON.stringify({ until })
    });
}

export async function unsnoozeMessage(accountId, folder, uid) {
    const resolvedFolder = await resolveFolderName(accountId, folder || 'Inbox');
    return apiFetch(`/unsnooze/${encodeURIComponent(accountId)}/${encodeURIComponent(resolvedFolder)}/${uid}`, { method: 'POST' });
}

// ─── Search ──────────────────────────────────────────────────────────
export async function searchMessages(query, opts = {}) {
    const params = new URLSearchParams();
    if (query) params.set('q', query);
    if (opts.accountId) params.set('account_id', opts.accountId);
    if (opts.folder) params.set('folder', opts.folder);
    if (opts.unread) params.set('unread', 'true');
    if (opts.attachments) params.set('attachments', 'true');
    params.set('limit', opts.limit || '100');
    if (opts.beforeUid) params.set('before_uid', opts.beforeUid);
    const r = await apiFetch(`/search?${params.toString()}`);
    const data = await r.json();
    return (data.messages || []).map(m => ({
        id: `${m.account_id}_${m.uid}`,
        uid: Number(m.uid),
        accountId: m.account_id,
        from: fixText(m.from_addr || ''),
        email: m.from_addr || '',
        subject: fixText(m.subject || ''),
        preview: m.preview || fixText(m.subject || ''),
        date: m.date || new Date().toISOString(),
        read: (m.flags || '').includes('\\Seen'),
        hasAttachment: !!m.has_attachments,
        flags: m.flags || '',
    }));
}

// ─── Send ────────────────────────────────────────────────────────────
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

// ─── Auth ────────────────────────────────────────────────────────────
export async function login(username, password) {
    return (await apiFetch('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) })).json();
}

export async function register(username, password) {
    return (await apiFetch('/auth/register', { method: 'POST', body: JSON.stringify({ username, password }) })).json();
}

// ─── Settings ────────────────────────────────────────────────────────
export async function getSettings() {
    return (await apiFetch('/settings')).json();
}
