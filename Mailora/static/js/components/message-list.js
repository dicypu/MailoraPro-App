// Mailora v2 — Message List Component (diff-based render)
import { store, ACTION, MSG_STATE } from '../store.js';
import { CONFIG } from '../config.js';
import { dataSource } from '../data-source.js';
const el = id => document.getElementById(id);
let _prevIds = [];

const API_URL = 'http://localhost:5000';

export function mountMessageList() {
    store.subscribe('messages', render);
    store.subscribe('selectedMessageId', render);
    store.subscribe('focusMode', render);
    store.subscribe('searchQuery', render);

    // Auto-analyze un-analyzed messages
    setTimeout(autoAnalyzeMessages, 1000);
}

const konuIcon = {
    is_proje: '💼', finans: '💰', alisveris: '🛒', teknoloji: '💻',
    pazarlama: '📢', kisisel: '👤', egitim: '🎓', seyahat: '✈️',
    hukuk_resmi: '⚖️', saglik: '🏥', sosyal_bildirim: '🔔', spor_eglence: '⚽'
};

async function autoAnalyzeMessages() {
    const s = store.getState();
    const unanalyzed = s.messages.filter(m => !m.aiAnalyzed);
    if (!unanalyzed.length) return;

    try {
        const res = await fetch(`${API_URL}/analyze-batch`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ messages: unanalyzed.map(m => ({ id: m.id, text: m.body || m.preview || '' })) })
        });

        if (res.ok) {
            const data = await res.json();
            const payload = {};
            for (const [id, analysis] of Object.entries(data)) {
                payload[id] = {
                    aiAnalyzed: true,
                    aiLabel: analysis.konu?.label,
                    aiSpam: analysis.spam?.score,
                    aiSmartReplies: analysis.smart_replies
                };
            }
            store.dispatch({ type: ACTION.UPDATE_MESSAGES_AI, payload });
        }
    } catch (err) {
        console.warn("AI Batch Analysis failed:", err);
    }
}

function render() {
    const list = el('message-list');
    if (!list) return;
    const msgs = store.getVisibleMessages();
    const s = store.getState();

    // Include ai properties in diff check
    const newIds = msgs.map(m => `${m.id}_${m.pinned}_${m.read}_${m.important}_${m.aiAnalyzed}`);

    if (JSON.stringify(newIds) === JSON.stringify(_prevIds) && !_selChanged(s)) return;
    _prevIds = newIds;

    list.innerHTML = msgs.length ? msgs.map(m => {
        const state = store.getMsgState(m.id);
        const sel = s.selectedMessageId === m.id ? 'selected' : '';
        const unread = !m.read ? 'unread' : '';
        const badges = [];

        const acc = s.accounts.find(a => a.id === m.accountId);
        if (s.selectedAccountId === 'unified' && acc) {
            badges.push(`<span class="badge" style="background:${acc.color}20;color:${acc.color};border:1px solid ${acc.color}50">${acc.displayName || acc.email.split('@')[0]}</span>`);
        }

        if (state === MSG_STATE.PINNED) badges.push('<span class="badge pin">📌</span>');
        if (m.important) badges.push('<span class="badge important">⭐</span>');
        if (m.isNewsletter) badges.push('<span class="badge newsletter">📰</span>');
        if (m.hasAttachment) badges.push('<span class="badge attachment">📎</span>');

        // AI Badges
        if (m.aiLabel && konuIcon[m.aiLabel]) {
            badges.push(`<span class="badge ai-topic" style="background:var(--bg-tertiary);color:var(--text-primary)">${konuIcon[m.aiLabel]} ${m.aiLabel}</span>`);
        }
        if (m.aiSpam && m.aiSpam >= 7) {
            badges.push(`<span class="badge ai-spam" style="background:#ef444430;color:#ef4444" title="Spam Güvenlik Skoru (10=Kötü)">🛡️ ${m.aiSpam}/10 Spam Riski</span>`);
        } else if (m.aiSpam && m.aiSpam <= 3) {
            badges.push(`<span class="badge ai-safe" style="background:#10b98130;color:#10b981" title="Güvenli Gönderici">🛡️ ${m.aiSpam}/10 Güvenli</span>`);
        }

        const time = formatTime(m.date);
        const readingTime = Math.max(1, Math.ceil((m.body?.length || 0) / 1000));

        return `<div class="msg-row ${sel} ${unread}" data-id="${m.id}">
            <div class="msg-sender">${m.from}${badges.join('')}</div>
            <div class="msg-subject">${m.subject}</div>
            <div class="msg-preview">${m.preview || ''}</div>
            <div class="msg-meta"><span class="msg-time">${time}</span><span class="msg-reading">~${readingTime}dk</span></div>
            <div class="msg-actions">
                <button class="act-btn" data-act="pin" title="${state === MSG_STATE.PINNED ? 'Sabitlemeyi kaldır' : 'Sabitle'}">${state === MSG_STATE.PINNED ? '📌' : '📍'}</button>
                <button class="act-btn" data-act="important" title="Önemli">${m.important ? '⭐' : '☆'}</button>
                <button class="act-btn" data-act="snooze" title="Ertele">⏰</button>
                <button class="act-btn" data-act="delete" title="Sil">🗑️</button>
            </div>
        </div>`;
    }).join('') : '<div class="empty-state"><div class="empty-icon">📭</div><div>Mesaj yok</div></div>';

    // Event delegation
    list.onclick = e => {
        const row = e.target.closest('.msg-row');
        if (!row) return;
        const id = row.dataset.id;
        const act = e.target.closest('.act-btn');
        const s = store.getState();
        const msg = s.messages.find(m => m.id === id);
        if (!msg) return;

        if (act) {
            e.stopPropagation();
            const a = act.dataset.act;
            if (a === 'pin') {
                const st = store.getMsgState(id);
                store.dispatch({ type: st === MSG_STATE.PINNED ? ACTION.UNPIN_MESSAGE : ACTION.PIN_MESSAGE, payload: id });
            }
            else if (a === 'important') {
                store.dispatch({ type: ACTION.MARK_IMPORTANT, payload: id });
                dataSource.updateFlags(msg.accountId, msg.folder, msg.uid, { flagged: !msg.important });
            }
            else if (a === 'snooze') {
                document.querySelectorAll('.snooze-dropdown').forEach(e => e.remove());

                const dd = document.createElement('div');
                dd.className = 'snooze-dropdown';
                dd.style.position = 'absolute';
                dd.style.background = 'var(--bg-primary)';
                dd.style.border = '1px solid var(--border)';
                dd.style.borderRadius = '8px';
                dd.style.padding = '8px 0';
                dd.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
                dd.style.zIndex = '1000';
                dd.style.minWidth = '150px';
                
                const rect = act.getBoundingClientRect();
                dd.style.top = (rect.bottom + window.scrollY + 5) + 'px';
                dd.style.left = (rect.left + window.scrollX - 50) + 'px';

                CONFIG.snoozeOptions.forEach(opt => {
                    const btn = document.createElement('div');
                    btn.innerText = opt.label;
                    btn.style.padding = '8px 16px';
                    btn.style.cursor = 'pointer';
                    btn.style.fontSize = '13px';
                    btn.onmouseover = () => btn.style.background = 'var(--bg-secondary)';
                    btn.onmouseout = () => btn.style.background = 'transparent';
                    btn.onclick = (ev) => {
                        ev.stopPropagation();
                        dd.remove();
                        const until = Date.now() + opt.ms;
                        store.dispatch({ type: ACTION.SNOOZE_MESSAGE, payload: { id, until } });
                        dataSource.snoozeMessage(msg.accountId, msg.folder, msg.uid, new Date(until).toISOString());
                    };
                    dd.appendChild(btn);
                });

                const customBtn = document.createElement('div');
                customBtn.innerHTML = 'Özel Tarih Seç... <input type="datetime-local" style="width:100%; margin-top:6px; display:none; background:var(--bg-secondary); border:1px solid var(--border); color:var(--text-primary); border-radius:4px; padding:4px;">';
                customBtn.style.padding = '8px 16px';
                customBtn.style.cursor = 'pointer';
                customBtn.style.fontSize = '13px';
                customBtn.style.borderTop = '1px solid var(--border)';
                
                const inp = customBtn.querySelector('input');
                customBtn.onclick = (ev) => {
                    ev.stopPropagation();
                    if(inp.style.display === 'none') {
                        inp.style.display = 'block';
                        inp.focus();
                        inp.showPicker && inp.showPicker();
                    }
                };
                inp.onchange = (ev) => {
                    ev.stopPropagation();
                    const d = new Date(inp.value);
                    if(isNaN(d.getTime())) return;
                    dd.remove();
                    store.dispatch({ type: ACTION.SNOOZE_MESSAGE, payload: { id, until: d.getTime() } });
                    dataSource.snoozeMessage(msg.accountId, msg.folder, msg.uid, d.toISOString());
                };
                dd.appendChild(customBtn);

                document.body.appendChild(dd);
                
                setTimeout(() => {
                    const closeFn = (ev) => {
                        if (!dd.contains(ev.target)) {
                            dd.remove();
                            document.removeEventListener('click', closeFn);
                        }
                    };
                    document.addEventListener('click', closeFn);
                }, 10);
            }
            else if (a === 'delete') {
                if (confirm('Silmek istediğinize emin misiniz?')) {
                    store.dispatch({ type: ACTION.DELETE_MESSAGE, payload: id });
                    dataSource.updateFlags(msg.accountId, msg.folder, msg.uid, { deleted: true, seen: true });
                }
            }
            return;
        }
        store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: id });
        if (!msg.read) {
            store.dispatch({ type: ACTION.MARK_READ, payload: id });
            dataSource.updateFlags(msg.accountId, msg.folder, msg.uid, { seen: true });
        }
    };
}
let _prevSel = null;
function _selChanged(s) { const c = _prevSel !== s.selectedMessageId; _prevSel = s.selectedMessageId; return c; }
function formatTime(d) {
    const dt = new Date(d); const now = new Date(); const diff = now - dt;
    if (diff < 86400000) return dt.toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit' });
    if (diff < 604800000) return dt.toLocaleDateString('tr-TR', { weekday: 'short' });
    return dt.toLocaleDateString('tr-TR', { day: 'numeric', month: 'short' });
}
