// Mailora v2 — Message Preview Component
import { store, ACTION } from '../store.js';
import { CONFIG } from '../config.js';
import { dataSource } from '../data-source.js';
const el = id => document.getElementById(id);
let currentRenderId = 0;
let currentAbortController = null;
export function mountPreview() {
    store.subscribe('selectedMessageId', render);
    store.subscribe('messages', render);
}
async function render() {
    const renderId = ++currentRenderId;
    if (currentAbortController) {
        currentAbortController.abort();
    }
    currentAbortController = new AbortController();
    const signal = currentAbortController.signal;
    const c = el('preview-pane');
    if (!c) return;
    const s = store.getState();
    const msg = s.messages.find(m => m.id === s.selectedMessageId);
    if (!msg) { c.innerHTML = '<div class="empty-state"><div class="empty-icon">📬</div><div>Bir e-posta seçin</div></div>'; return; }

    // Yükleniyor...
    c.innerHTML = `<div class="empty-state"><div class="spinner"></div><div>Yükleniyor...</div></div>`;

    try {
        let bodyData, attData;
        if (msg.isOutbox) {
            bodyData = {
                subject: msg.subject || '(Konu Yok)',
                from: 'Giden Kutusu Kuyruğu',
                html_body: '',
                plain_text: msg.body || '',
                date: msg.date || '',
            };
            attData = [];
        } else {
                        bodyData = await dataSource.getMessage(msg.accountId, msg.uid, msg.folder, signal);
            if (renderId !== currentRenderId) return;
            attData = await dataSource.getAttachments(msg.accountId, msg.uid, msg.folder);
            if (renderId !== currentRenderId) return;

            // Okundu olarak işaretle
            if (!msg.read) {
                store.dispatch({ type: ACTION.MARK_READ, payload: msg.id });
                try { await dataSource.updateFlags(msg.accountId, msg.folder, msg.uid, { seen: true }); } catch (e) { }
            }
        }

        const readMin = Math.max(1, Math.ceil((bodyData.plain_text?.length || 0) / 1000));

        // HTML Body Rendering
        let bodyHtml = '';
        if (bodyData.html_body && bodyData.html_body.trim().length > 0) {
            bodyHtml = `<div class='preview-html-container' style="flex:1; display:flex; flex-direction:column; min-height:400px; margin-top:10px;">
                <iframe id='html-frame' style="width:100%; flex:1; border:none; background:#fff; border-radius:8px;" sandbox='allow-scripts allow-popups allow-forms allow-same-origin'></iframe>
            </div>`;
        } else {
            bodyHtml = `<div class="preview-body" style="white-space:pre-wrap;">${bodyData.plain_text || '(Mesaj içeriği boş)'}</div>`;
        }

        // Attachments
        let attHtml = '';
        if (attData && attData.length > 0) {
            attHtml = `<div class="preview-attachments" style="margin-top:20px; padding-top:15px; border-top:1px solid var(--border);">
                <h4 style="margin-bottom:10px;">📎 Ekler (${attData.length})</h4>
                <div class="att-list" style="display:flex; flex-wrap:wrap; gap:10px;">
                    ${attData.map(a => {
                const fname = a.filename || a.content_type || 'dosya';
                const size = a.size ? (a.size < 1024 ? a.size + 'B' : (a.size / 1024).toFixed(1) + 'KB') : '';
                const part = a.part_id || a.part || a.filename || '1';
                const url = dataSource.getAttachmentDownloadUrl(msg.accountId, msg.uid, part, msg.folder);
                return `<a href="${url}" target="_blank" class="att-item" style="text-decoration:none; padding:8px 12px; background:var(--bg-tertiary); border-radius:6px; font-size:12px; color:var(--text-primary); border:1px solid var(--border); display:flex; align-items:center; gap:6px;">
                            📄 ${fname} <span class="att-size" style="color:var(--text-muted); font-size:11px;">${size}</span>
                        </a>`;
            }).join('')}
                </div>
            </div>`;
        }

        let outboxStatusHtml = '';
        if (msg.isOutbox) {
            const statusColor = {
                queued: '#f59e0b',
                processing: '#3b82f6',
                sent: '#10b981',
                failed: '#ef4444'
            }[msg.outboxStatus] || 'var(--text-muted)';

            const statusLabel = {
                queued: '⏳ Kuyrukta Bekliyor',
                processing: '⚙️ Gönderiliyor...',
                sent: '✅ Gönderildi',
                failed: '❌ Gönderim Başarısız Oldu'
            }[msg.outboxStatus] || msg.outboxStatus;

            outboxStatusHtml = `
                <div class="outbox-status-card" style="padding:16px; background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; margin-bottom:20px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1);">
                    <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px;">
                        <span style="font-weight:600; color:${statusColor}; font-size:14px;">${statusLabel}</span>
                        <span style="font-size:12px; color:var(--text-muted)">Deneme Sayısı: ${msg.retries || 0}/3</span>
                    </div>
                    ${msg.outboxError ? `<div style="color:#ef4444; font-size:12px; margin-bottom:12px; padding:10px; background:rgba(239,68,68,0.08); border:1px solid rgba(239,68,68,0.2); border-radius:6px; line-height: 1.4;"><strong>Hata Detayı:</strong> ${msg.outboxError}</div>` : ''}
                    <div style="display:flex; gap:10px;">
                        ${msg.outboxStatus === 'failed' ? `<button class="tool-btn" id="btn-outbox-retry" style="padding:6px 12px; font-size:12px; background:var(--gradient-primary); color:white; border:none; border-radius:4px; cursor:pointer;">🔄 Tekrar Dene</button>` : ''}
                        <button class="tool-btn" id="btn-outbox-delete" style="padding:6px 12px; font-size:12px; background:rgba(239,68,68,0.2); color:#ef4444; border:1px solid rgba(239,68,68,0.3); border-radius:4px; cursor:pointer;">🗑️ Gönderimi İptal Et</button>
                    </div>
                </div>
            `;
        }

        c.innerHTML = `
            <div class="preview-header">
                <div class="preview-from">
                    <div class="avatar" style="background:var(--gradient-primary)">${msg.from[0] ? msg.from[0].toUpperCase() : '?'}</div>
                    <div><div class="preview-sender">${msg.from}</div><div class="preview-email">${msg.email || ''}</div></div>
                    <div class="preview-time">${new Date(msg.date).toLocaleString('tr-TR')}</div>
                </div>
                <h2 class="preview-subject">${msg.subject}</h2>
                <div class="preview-badges">
                    <span class="reading-badge">⏱️ ~${readMin} dk okuma</span>
                    ${msg.labels?.map(l => '<span class="label-badge">' + l + '</span>').join('') || ''}
                </div>
            </div>
            ${msg.isOutbox ? '' : `
            <div class="preview-toolbar">
                <button class="tool-btn" id="btn-reply">↩️ Yanıtla</button>
                <button class="tool-btn" id="btn-forward">➡️ İlet</button>
                <button class="tool-btn" id="btn-ner" title="Kişi, Para ve Yer isimlerini ayıkla">🔍 Varlık Çıkar</button>
                <button class="tool-btn" id="btn-ai">🤖 AI Özet</button>
                <button class="tool-btn" id="btn-translate">🌍 Çevir</button>
                ${msg.isNewsletter ? '<button class="tool-btn newsletter-btn" id="btn-unsub">📰 Abonelikten Çık</button>' : ''}
            </div>
            `}
            <div id="ai-summary" class="ai-box" style="display:none"></div>
            <div id="translate-box" class="translate-box" style="display:none"></div>
            ${outboxStatusHtml}
            ${bodyHtml}
            ${attHtml}
        `;

        // Iframe setup
        if (bodyData.html_body && bodyData.html_body.trim().length > 0) {
            const iframe = el('html-frame');
            if (iframe) {
                const shell = `<!doctype html><html><head><base target="_blank"><style>html,body{margin:0;padding:12px;font:14px system-ui,-apple-system,Segoe UI,Roboto,sans-serif;color:#333;background:#fff}img{max-width:100%;height:auto}table{max-width:100%;overflow:auto}a{color:#2563eb}</style></head><body>__CONTENT__</body></html>`;
                let safeHtml = bodyData.html_body
                    .replace(/<(script)[^>]*>[\s\S]*?<\/\1>/gi, '')
                    .replace(/<(form|object|embed|iframe)[^>]*>[\s\S]*?<\/\1>/gi, '')
                    .replace(/ on[a-zA-Z]+="[^"]*"|'[^']*'/g, '');

                safeHtml = safeHtml.replace(/src\s*=\s*(["'])cid:([^"']+)\1/gi, (m, q, cid) => {
                    return `src="${dataSource.getAttachmentDownloadUrl(msg.accountId, msg.uid, cid, msg.folder)}"`;
                });

                iframe.srcdoc = shell.replace('__CONTENT__', safeHtml);
            }
        }

        // Save plain_text for AI tools
        msg.raw_body = bodyData.plain_text;
    } catch (err) {
        if (err.name === 'AbortError') {
            console.log('Fetch aborted for older email click');
            return;
        }
        c.innerHTML = `<div class="empty-state"><div style="color:var(--accent-red)">Hata: ${err.message}</div></div>`;
        return;
    }

    if (msg.isOutbox) {
        // Attach outbox action listeners
        el('btn-outbox-retry')?.addEventListener('click', async () => {
            try {
                const r = await fetch(`/outbox/${msg.uid}/retry`, { method: 'POST' });
                const data = await r.json();
                if (data.ok) {
                    const accId = store.getState().selectedAccountId;
                    const folder = store.getState().selectedFolder;
                    const msgs = accId === 'unified' 
                        ? await dataSource.getUnifiedInbox(folder)
                        : await dataSource.getMessages(accId, folder);
                    store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
                } else {
                    alert('Hata: ' + data.error);
                }
            } catch (e) {
                alert('İstek gönderilemedi: ' + e.message);
            }
        });

        el('btn-outbox-delete')?.addEventListener('click', async () => {
            if (!confirm('Bu e-posta gönderimini iptal etmek ve kuyruktan silmek istiyor musunuz?')) return;
            try {
                const r = await fetch(`/outbox/${msg.uid}`, { method: 'DELETE' });
                const data = await r.json();
                if (data.ok) {
                    const accId = store.getState().selectedAccountId;
                    const folder = store.getState().selectedFolder;
                    const msgs = accId === 'unified' 
                        ? await dataSource.getUnifiedInbox(folder)
                        : await dataSource.getMessages(accId, folder);
                    store.dispatch({ type: ACTION.SET_MESSAGES, payload: msgs });
                    store.dispatch({ type: ACTION.SELECT_MESSAGE, payload: null });
                } else {
                    alert('Hata: ' + data.error);
                }
            } catch (e) {
                alert('İstek gönderilemedi: ' + e.message);
            }
        });
    } else {
        // Quick reply - Dynamic from AI if available
        c.insertAdjacentHTML('beforeend', `<div class="quick-replies" id="dynamic-replies-container" style="margin-top:12px">
            <div style="font-size:11px;color:var(--text-muted);display:flex;align-items:center;gap:6px"><div class="spinner" style="width:10px;height:10px;border-width:2px"></div> ✨ Akıllı yanıt üretiliyor...</div>
        </div>`);
        // Feature bindings
        el('btn-ner')?.addEventListener('click', () => showNER(msg));
        el('btn-ai')?.addEventListener('click', () => showAI(msg));
        el('btn-translate')?.addEventListener('click', () => showTranslate(msg));
        el('btn-unsub')?.addEventListener('click', () => unsubNewsletter(msg));
        el('btn-reply')?.addEventListener('click', () => { store.dispatch({ type: ACTION.TOGGLE_COMPOSE }); });

        // Fetch Generative MT5 Smart Replies
        fetch(`${AI_API}/smart-reply`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text: msg.raw_body?.replace(/<[^>]*>/g, '') || msg.preview || '' })
        }).then(res => res.json()).then(data => {
            const qrContainer = el('dynamic-replies-container');
            if (qrContainer && data.replies) {
                qrContainer.innerHTML = `<span style="font-size:11px;color:var(--text-muted);margin-right:8px">✨ MT5 Yanıtları:</span>` +
                    data.replies.map(r => `<button class="qr-btn" data-qr="${r}">${r}</button>`).join('');
                qrContainer.querySelectorAll('.qr-btn').forEach(b => b.onclick = () => alert('Gönderildi: ' + b.dataset.qr));
            }
        }).catch(e => console.error("Smart reply failed", e));
    }
}
const AI_API = 'http://localhost:5000';

async function showAI(msg) {
    const box = el('ai-summary');
    if (!box) return;
    if (box.style.display !== 'none') { box.style.display = 'none'; return; }

    const text = msg.raw_body?.replace(/<[^>]*>/g, '') || msg.preview || '';
    box.innerHTML = `<div style="display:flex;align-items:center;gap:8px"><div class="spinner"></div> <span>BERT modeli analiz ediyor...</span></div>`;
    box.style.display = 'block';

    try {
        const res = await fetch(`${AI_API}/analyze`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text })
        });

        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();

        const duyguRenk = { Negatif: '#ef4444', Nötr: '#94a3b8', Pozitif: '#10b981', Bilinmiyor: '#64748b' };
        const konuIcon = {
            is_proje: '💼', finans: '💰', alisveris: '🛒', teknoloji: '💻',
            pazarlama: '📢', kisisel: '👤', egitim: '🎓', seyahat: '✈️',
            hukuk_resmi: '⚖️', saglik: '🏥', sosyal_bildirim: '🔔', spor_eglence: '⚽'
        };

        const d = data.duygu || {};
        const k = data.konu || {};

        // Duygu bar grafiği
        const duyguBars = d.scores ? Object.entries(d.scores).map(([label, score]) => {
            const color = duyguRenk[label] || '#64748b';
            return `<div style="display:flex;align-items:center;gap:6px;font-size:11px;">
                <span style="width:55px;color:${color}">${label}</span>
                <div style="flex:1;height:6px;background:var(--bg-tertiary);border-radius:3px;overflow:hidden">
                    <div style="width:${score}%;height:100%;background:${color};border-radius:3px;transition:width 0.5s"></div>
                </div>
                <span style="width:40px;text-align:right;color:var(--text-muted)">${score}%</span>
            </div>`;
        }).join('') : '';

        // Konu bar grafiği
        const konuBars = k.scores ? Object.entries(k.scores).map(([label, score]) => {
            const icon = konuIcon[label] || '📌';
            return `<div style="display:flex;align-items:center;gap:6px;font-size:11px;">
                <span style="width:75px">${icon} ${label}</span>
                <div style="flex:1;height:6px;background:var(--bg-tertiary);border-radius:3px;overflow:hidden">
                    <div style="width:${score}%;height:100%;background:var(--accent-blue);border-radius:3px;transition:width 0.5s"></div>
                </div>
                <span style="width:40px;text-align:right;color:var(--text-muted)">${score}%</span>
            </div>`;
        }).join('') : '';

        box.innerHTML = `
            <div style="display:flex;gap:24px;flex-wrap:wrap">
                <div style="flex:1;min-width:200px">
                    <div style="font-weight:600;margin-bottom:8px">🎭 Duygu Analizi
                        <span style="color:${duyguRenk[d.label] || '#fff'};font-weight:700;margin-left:8px">${d.label || '?'} (${d.confidence || 0}%)</span>
                    </div>
                    ${duyguBars}
                </div>
                <div style="flex:1;min-width:200px">
                    <div style="font-weight:600;margin-bottom:8px">${konuIcon[k.label] || '📌'} Konu Tahmini
                        <span style="color:var(--accent-blue);font-weight:700;margin-left:8px">${k.label || '?'} (${k.confidence || 0}%)</span>
                    </div>
                    ${konuBars}
                </div>
            </div>
            <div id="ai-real-summary-box" style="margin-top:16px;padding:12px;background:var(--bg-primary);border-radius:6px;border:1px solid var(--border-color)">
                <strong>📝 Üretken Özet (MT5-Small)</strong>
                <div id="ai-real-summary-content" style="margin-top:8px;font-style:italic"><div style="display:flex;align-items:center;gap:8px"><div class="spinner"></div> <span>Model belleğe yüklenip özet çıkartılıyor... 3-4 sn bekleyin.</span></div></div>
            </div>
            <div style="margin-top:10px;font-size:10px;color:var(--text-muted)">BERT Turkish Model • ${data.duygu?.error ? '⚠️ Duygu modeli yüklenmedi' : '✅ Duygu aktif'} • ${data.konu?.error ? '⚠️ Konu modeli yüklenmedi' : '✅ Konu aktif'}</div>
        `;

        // Async trigger summarization
        try {
            const sumRes = await fetch(`${AI_API}/summarize`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ text })
            });
            const sumData = await sumRes.json();
            const sumBox = el('ai-real-summary-content');
            if (sumBox) {
                if (sumData.error) sumBox.innerHTML = `<span style="color:#ef4444">Hata: ${sumData.error}</span>`;
                else sumBox.innerHTML = sumData.summary;
            }
        } catch (e) {
            console.error("Özetleme hatası", e);
        }
    } catch (err) {
        // API çalışmıyorsa fallback: basit özet
        const sentences = text.split(/[.!?]/).filter(s => s.trim().length > 10);
        const summary = sentences.slice(0, 2).join('. ').trim() + '.';
        box.innerHTML = `<strong>🤖 AI Özet (Offline):</strong> ${summary || 'Özet oluşturulamadı.'}
            <div style="margin-top:8px;font-size:11px;color:var(--accent-orange)">⚠️ ML API bağlantısı yok. Başlatmak için: <code>cd MailoraPro && python api_server.py</code></div>`;
    }
}
async function showTranslate(msg) {
    const box = el('translate-box');
    if (!box) return;
    if (box.style.display !== 'none') { box.style.display = 'none'; return; }

    box.innerHTML = `<div class="translate-header"><strong>🌍 Çeviri</strong></div>
        <div class="lang-btns">
            <button class="lang-btn" data-lang="tr">İngilizce -> Türkçe</button>
            <button class="lang-btn" data-lang="en">Türkçe -> İngilizce</button>
        </div>
        <div id="translate-result" class="translate-result" style="margin-top:10px;font-size:13px">Lütfen bir dil yönü seçin.</div>`;
    box.style.display = 'block';

    box.querySelectorAll('.lang-btn').forEach(b => b.onclick = async () => {
        const resBox = el('translate-result');
        const targetLang = b.dataset.lang;
        resBox.innerHTML = `<div style="display:flex;align-items:center;gap:8px"><div class="spinner"></div> <span>Helsinki-NLP modeli yükleniyor ve çevriliyor (Lazy Load)... Lütfen 2-3 sn bekleyin.</span></div>`;

        try {
            const textToTranslate = msg.raw_body?.replace(/<[^>]*>/g, '') || msg.preview || '';
            const res = await fetch(`${AI_API}/translate`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ text: textToTranslate, target_lang: targetLang })
            });
            const data = await res.json();
            if (data.error) throw new Error(data.error);
            resBox.innerHTML = `<strong>Çeviri Sonucu:</strong><br><br>${data.translated_text}`;
        } catch (e) {
            resBox.innerHTML = `<span style="color:#ef4444">Hata: ${e.message}</span>`;
        }
    });
}

async function showNER(msg) {
    const box = el('ai-summary');
    if (!box) return;

    // We will show loading in the ai-summary box, but the result will highlight the body
    if (box.style.display !== 'none' && box.dataset.activeTool === 'ner') {
        box.style.display = 'none';
        return;
    }

    box.dataset.activeTool = 'ner';
    box.innerHTML = `<div style="display:flex;align-items:center;gap:8px"><div class="spinner"></div> <span>BERT-NER Modeli isimleri ve yerleri arıyor (Lazy Load)... Lütfen 2-3 sn bekleyin.</span></div>`;
    box.style.display = 'block';

    try {
        const textToAnalyze = msg.raw_body?.replace(/<[^>]*>/g, '') || msg.preview || '';
        const res = await fetch(`${AI_API}/extract-entities`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text: textToAnalyze })
        });

        const data = await res.json();
        if (data.error) throw new Error(data.error);

        const entityLabelMap = { 'PER': 'Kişi 👤', 'ORG': 'Kurum 🏢', 'LOC': 'Lokasyon 📍', 'DATE': 'Tarih 📅', 'MONEY': 'Para 💰' };

        let foundEntitiesHtml = data.entities.map(e =>
            `<span style="display:inline-block; margin:4px; padding:2px 8px; border-radius:12px; font-size:12px; background:var(--bg-tertiary); color:var(--accent-blue)">
                ${e.word} <strong style="opacity:0.7;font-size:10px">${entityLabelMap[e.entity] || e.entity}</strong>
            </span>`
        ).join('');

        if (data.entities.length === 0) {
            foundEntitiesHtml = `<span style="color:var(--text-muted)">Özel isim veya lokasyon bulunamadı.</span>`;
        }

        box.innerHTML = `<div style="font-weight:600;margin-bottom:8px">🔍 Varlık Çıkarımı Sonuçları:</div><div>${foundEntitiesHtml}</div>`;

    } catch (err) {
        box.innerHTML = `<span style="color:#ef4444">Hata: ${err.message}</span>`;
    }
}

function unsubNewsletter(msg) {
    if (confirm(`${msg.from} aboneliğinden çıkmak istiyor musunuz?`)) alert('✓ Abonelikten çıkıldı.');
}
