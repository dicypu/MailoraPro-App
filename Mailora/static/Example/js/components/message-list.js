// Mailora v2 — Message List Component (diff-based render)
import { store, ACTION, MSG_STATE } from '../store.js';
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
    is_proje:'💼', finans:'💰', alisveris:'🛒', teknoloji:'💻', 
    pazarlama:'📢', kisisel:'👤', egitim:'🎓', seyahat:'✈️', 
    hukuk_resmi:'⚖️', saglik:'🏥', sosyal_bildirim:'🔔', spor_eglence:'⚽' 
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
    } catch(err) {
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
            <div class="msg-preview">${m.preview||''}</div>
            <div class="msg-meta"><span class="msg-time">${time}</span><span class="msg-reading">~${readingTime}dk</span></div>
            <div class="msg-actions">
                <button class="act-btn" data-act="pin" title="${state===MSG_STATE.PINNED?'Sabitlemeyi kaldır':'Sabitle'}">${state===MSG_STATE.PINNED?'📌':'📍'}</button>
                <button class="act-btn" data-act="important" title="Önemli">${m.important?'⭐':'☆'}</button>
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
        if (act) {
            e.stopPropagation();
            const a = act.dataset.act;
            if (a === 'pin') { const st = store.getMsgState(id); store.dispatch({ type: st===MSG_STATE.PINNED ? ACTION.UNPIN_MESSAGE : ACTION.PIN_MESSAGE, payload: id }); }
            else if (a === 'important') store.dispatch({ type: ACTION.MARK_IMPORTANT, payload: id });
            else if (a === 'snooze') store.dispatch({ type: ACTION.SNOOZE_MESSAGE, payload: { id, until: Date.now()+3600000 } });
            else if (a === 'delete') store.dispatch({ type: ACTION.DELETE_MESSAGE, payload: id });
            return;
        }
        store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: id });
        store.dispatch({ type: ACTION.MARK_READ, payload: id });
    };
}
let _prevSel = null;
function _selChanged(s) { const c = _prevSel !== s.selectedMessageId; _prevSel = s.selectedMessageId; return c; }
function formatTime(d) { const dt = new Date(d); const now = new Date(); const diff = now - dt;
    if (diff < 86400000) return dt.toLocaleTimeString('tr-TR',{hour:'2-digit',minute:'2-digit'});
    if (diff < 604800000) return dt.toLocaleDateString('tr-TR',{weekday:'short'});
    return dt.toLocaleDateString('tr-TR',{day:'numeric',month:'short'}); }
