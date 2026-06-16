// Mailora v2 — Message Preview Component
import { store, ACTION } from '../store.js';
import { CONFIG } from '../config.js';
const el = id => document.getElementById(id);
export function mountPreview() {
    store.subscribe('selectedMessageId', render);
    store.subscribe('messages', render);
}
function render() {
    const c = el('preview-pane');
    if (!c) return;
    const s = store.getState();
    const msg = s.messages.find(m => m.id === s.selectedMessageId);
    if (!msg) { c.innerHTML = '<div class="empty-state"><div class="empty-icon">📬</div><div>Bir e-posta seçin</div></div>'; return; }
    const readMin = Math.max(1, Math.ceil((msg.body?.length || 0) / 1000));
    c.innerHTML = `
        <div class="preview-header">
            <div class="preview-from">
                <div class="avatar" style="background:var(--gradient-primary)">${msg.from[0]}</div>
                <div><div class="preview-sender">${msg.from}</div><div class="preview-email">${msg.email||''}</div></div>
                <div class="preview-time">${new Date(msg.date).toLocaleString('tr-TR')}</div>
            </div>
            <h2 class="preview-subject">${msg.subject}</h2>
            <div class="preview-badges">
                <span class="reading-badge">⏱️ ~${readMin} dk okuma</span>
                ${msg.labels?.map(l=>'<span class="label-badge">'+l+'</span>').join('')||''}
            </div>
        </div>
        <div class="preview-toolbar">
            <button class="tool-btn" id="btn-reply">↩️ Yanıtla</button>
            <button class="tool-btn" id="btn-forward">➡️ İlet</button>
            <button class="tool-btn" id="btn-ner" title="Kişi, Para ve Yer isimlerini ayıkla">🔍 Varlık Çıkar</button>
            <button class="tool-btn" id="btn-ai">🤖 AI Özet</button>
            <button class="tool-btn" id="btn-translate">🌍 Çevir</button>
            ${msg.isNewsletter?'<button class="tool-btn newsletter-btn" id="btn-unsub">📰 Abonelikten Çık</button>':''}
        </div>
        <div id="ai-summary" class="ai-box" style="display:none"></div>
        <div id="translate-box" class="translate-box" style="display:none"></div>
        <div class="preview-body">${msg.body||msg.preview||''}</div>
        ${msg.hasAttachment?'<div class="preview-attachments"><h4>📎 Ekler</h4><div class="att-list"><div class="att-item">📄 rapor.pdf <span class="att-size">2.4 MB</span></div></div></div>':''}
    `;
    // Quick reply - Dynamic from AI if available
    c.insertAdjacentHTML('beforeend', `<div class="quick-replies" id="dynamic-replies-container" style="margin-top:12px">
        <div style="font-size:11px;color:var(--text-muted);display:flex;align-items:center;gap:6px"><div class="spinner" style="width:10px;height:10px;border-width:2px"></div> ✨ Akıllı yanıt üretiliyor...</div>
    </div>`);
    // Feature bindings
    el('btn-ner')?.addEventListener('click', () => showNER(msg));
    el('btn-ai')?.addEventListener('click', () => showAI(msg));
    el('btn-translate')?.addEventListener('click', () => showTranslate(msg));
    el('btn-unsub')?.addEventListener('click', () => unsubNewsletter(msg));
    el('btn-reply')?.addEventListener('click', () => { store.dispatch({type:ACTION.TOGGLE_COMPOSE}); });
    
    // Fetch Generative MT5 Smart Replies
    fetch(`${AI_API}/smart-reply`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ text: msg.body?.replace(/<[^>]*>/g,'') || msg.preview || '' })
    }).then(res => res.json()).then(data => {
        const qrContainer = el('dynamic-replies-container');
        if (qrContainer && data.replies) {
            qrContainer.innerHTML = `<span style="font-size:11px;color:var(--text-muted);margin-right:8px">✨ MT5 Yanıtları:</span>` +
                data.replies.map(r => `<button class="qr-btn" data-qr="${r}">${r}</button>`).join('');
            qrContainer.querySelectorAll('.qr-btn').forEach(b => b.onclick = () => alert('Gönderildi: ' + b.dataset.qr));
        }
    }).catch(e => console.error("Smart reply failed", e));
}
const AI_API = 'http://localhost:5000';

async function showAI(msg) {
    const box = el('ai-summary');
    if (!box) return;
    if (box.style.display !== 'none') { box.style.display = 'none'; return; }

    const text = msg.body?.replace(/<[^>]*>/g,'') || msg.preview || '';
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

        const duyguRenk = { Negatif:'#ef4444', Nötr:'#94a3b8', Pozitif:'#10b981', Bilinmiyor:'#64748b' };
        const konuIcon = { 
            is_proje:'💼', finans:'💰', alisveris:'🛒', teknoloji:'💻', 
            pazarlama:'📢', kisisel:'👤', egitim:'🎓', seyahat:'✈️', 
            hukuk_resmi:'⚖️', saglik:'🏥', sosyal_bildirim:'🔔', spor_eglence:'⚽' 
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
                        <span style="color:${duyguRenk[d.label]||'#fff'};font-weight:700;margin-left:8px">${d.label||'?'} (${d.confidence||0}%)</span>
                    </div>
                    ${duyguBars}
                </div>
                <div style="flex:1;min-width:200px">
                    <div style="font-weight:600;margin-bottom:8px">${konuIcon[k.label]||'📌'} Konu Tahmini
                        <span style="color:var(--accent-blue);font-weight:700;margin-left:8px">${k.label||'?'} (${k.confidence||0}%)</span>
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
        } catch(e) {
            console.error("Özetleme hatası", e);
        }
    } catch (err) {
        // API çalışmıyorsa fallback: basit özet
        const sentences = text.split(/[.!?]/).filter(s=>s.trim().length>10);
        const summary = sentences.slice(0,2).join('. ').trim() + '.';
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
            const textToTranslate = msg.body?.replace(/<[^>]*>/g,'') || msg.preview || '';
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
        const textToAnalyze = msg.body?.replace(/<[^>]*>/g,'') || msg.preview || '';
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
