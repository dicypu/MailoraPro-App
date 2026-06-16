const mockData = [
    {
        id: 12, sender: 'Global Tech Partners', email: 'sarah@globaltech.com', accName: 'Work', accColor: '#3b82f6', folder: 'inbox',
        subject: 'Partnership Agreement Updates & Next Steps', time: '11:45', readTime: '~2dk',
        preview: 'Hello team, please find the revised partnership agreement attached. We have updated the clauses regarding data privacy...',
        body: `<p>Hello team,</p>
               <p>Please find the revised partnership agreement for the upcoming quarter attached. We have updated the clauses regarding data privacy and cross-border data transfer, as discussed during our last meeting.</p>
               <p>We are very excited to move forward with the Mailora Hub integration. Our engineering team believes this will drastically improve our internal communication flow.</p>
               <p>Could you please review these changes and share your feedback by Friday?</p>
               <p>Best regards,<br><strong>Sarah Connor</strong><br>Director of Partnerships</p>`,
        pinned: true, important: true, read: false,
        aiTopic: 'İş', aiIcon: '💼', aiSpam: null,
        hasAttachment: true
    },
    {
        id: 1, sender: 'Ahmet Yılmaz', email: 'ahmet@company.com', accName: 'Work', accColor: '#3b82f6', folder: 'inbox',
        subject: 'Q3 Finansal Raporları ve Proje Analizleri', time: '10:45', readTime: '~3dk',
        preview: 'Merhaba ekip, son çeyreğe ait finansal raporları ve proje analizlerini ekte bulabilirsiniz. Toplantıda görüşmek üzere...',
        body: `<p>Merhaba ekip,</p><p>Son çeyreğe ait finansal raporları ve proje durum analizlerini ekteki tabloda bulabilirsiniz.</p><p>Özellikle <strong>Rust ve Axum</strong> mimarisine geçiş sürecindeki performans kazanımlarımız oldukça tatmin edici görünüyor. IMAP senkronizasyon sürelerinde %40 oranında bir iyileşme raporlanmış durumda. Bunun yanında yeni geliştirilen <strong>RBAC</strong> sisteminin güvenlik analiz sonuçları da oldukça başarılı.</p><p>Detayları yarınki haftalık toplantıda konuşacağız.</p><p>Kolay gelsin,</p><p><strong>Ahmet Yılmaz</strong><br>Danışman</p>`,
        pinned: false, important: true, read: true,
        aiTopic: 'Eğitim', aiIcon: '🎓', aiSpam: null,
        hasAttachment: true
    },
    {
        id: 14, sender: 'Muhasebe Departmanı', email: 'muhasebe@company.com', accName: 'Finance', accColor: '#f59e0b', folder: 'inbox',
        subject: 'Ekim Ayı Fatura Kesimleri', time: 'Pzt', readTime: '~1dk',
        preview: 'Ekim ayına ait fatura kesimleri tamamlanmıştır. Lütfen ekteki dosyadan kontrollerinizi sağlayın.',
        body: `<p>Ekim ayına ait tüm departman faturalarının kesim işlemleri tamamlanmıştır. Hata olmaması adına ekteki raporu incelemenizi rica ederiz.</p>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Finans', aiIcon: '💰', aiSpam: null,
        hasAttachment: true
    },
    {
        id: 2, sender: 'Mailora Team', email: 'updates@mailora.local', accName: 'Dev', accColor: '#10b981', folder: 'inbox',
        subject: 'Mailora Hub V2.0 Güncelleme Detayları', time: '09:12', readTime: '~1dk',
        preview: 'Yeni sürüm başarıyla yayına alındı. IMAP asenkron senkronizasyonu ve Tablolar modülü artık çok daha hızlı çalışıyor.',
        body: `<div style="text-align:center; padding: 20px; background:var(--bg-secondary); border-radius: 8px;">
            <h1 style="color:var(--accent-blue)">Mailora Hub V2.0 Yayında! 🎉</h1>
            <p>Sevgili geliştirici, yeni sürümdeki yenilikler:</p>
            <ul style="text-align:left; display:inline-block; margin:20px auto;">
                <li>Asenkron IMAP senkronizasyonu eklendi.</li>
                <li>Tablolar modülü için WebSocket desteği eklendi.</li>
                <li>Vanilla JS arayüzünde %60 hız artışı sağlandı.</li>
            </ul>
            <br><button style="padding:10px 20px; background:var(--accent-blue); color:white; border:none; border-radius:5px; margin-top:15px; cursor:pointer;">Sürüm Notlarını Oku</button>
        </div>`,
        pinned: true, important: false, read: false,
        aiTopic: 'Teknoloji', aiIcon: '💻', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 15, sender: 'Spotify', email: 'no-reply@spotify.com', accName: 'Personal', accColor: '#ef4444', folder: 'inbox',
        subject: 'Haftalık Keşif Listesi Hazır!', time: 'Paz', readTime: '~1dk',
        preview: 'Sana özel hazırladığımız Haftalık Keşif listesi yayında. Hemen dinlemeye başla.',
        body: `<p>Sana özel hazırladığımız yepyeni şarkılardan oluşan Haftalık Keşif listesi yayında!</p>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Eğlence', aiIcon: '🎧', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 3, sender: 'DevOps Alerts', email: 'alerts@devops.local', accName: 'Dev', accColor: '#10b981', folder: 'inbox',
        subject: 'Yeni Sunucu Kurulum Yönergeleri (Rust & Axum)', time: 'Dün', readTime: '~5dk',
        preview: 'Geliştirme ortamı için yeni Rust ve SQLite sunucularının ayağa kaldırılma adımları Wiki sayfasına eklendi. Lütfen inceleyin.',
        body: `<p>Sistem Yöneticisi,</p>
            <p>Aşağıdaki komutları kullanarak yeni sunucuları ayağa kaldırabilirsiniz:</p>
            <pre style="background:var(--bg-tertiary); padding:10px; border-radius:4px; border:1px solid var(--border);"><code>cargo run --release\nsqlite3 database.db < schema.sql</code></pre>
            <p>Daha fazla bilgi için Wiki'ye göz atın.</p>`,
        pinned: false, important: false, read: true,
        aiTopic: null, aiIcon: null, aiSpam: {score: 0, text: 'Güvenli', color: '#10b981'},
        hasAttachment: false
    },
    {
        id: 7, sender: 'Fatma Yılmaz', email: 'fatma@company.com', accName: 'Work', accColor: '#3b82f6', folder: 'inbox',
        subject: 'Ofis Malzemeleri Siparişi', time: '20 Eki', readTime: '~1dk',
        preview: 'Yeni ofis malzemeleri listesi ektedir. Lütfen onaylayın.',
        body: `<p>Merhaba,</p><p>İhtiyaç duyulan malzemeler listesi ektedir. Onayınızdan sonra sipariş geçilecektir.</p>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Alışveriş', aiIcon: '🛒', aiSpam: null,
        hasAttachment: true
    },
    {
        id: 13, sender: 'Prof. Dr. İlhan', email: 'ilhan@university.edu', accName: 'School', accColor: '#8b5cf6', folder: 'inbox',
        subject: 'Vize Sınavı Hakkında Bilgilendirme', time: 'Dün', readTime: '~2dk',
        preview: 'Değerli öğrenciler, vize sınavımız haftaya perşembe saat 14:00\'te yapılacaktır. Sınavda ilk 4 haftanın konularından...',
        body: `<p>Değerli öğrenciler,</p><p>Vize sınavımız haftaya perşembe saat 14:00'te online platform üzerinden yapılacaktır. Sınavda ilk 4 haftanın konularından sorumlu olacaksınız.</p><p>Başarılar dilerim.</p>`,
        pinned: false, important: true, read: false,
        aiTopic: 'Eğitim', aiIcon: '🎓', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 4, sender: 'Weekly Sync', email: 'sync@company.com', accName: 'Work', accColor: '#3b82f6', folder: 'inbox',
        subject: 'Haftalık Ekip Toplantısı Notları', time: 'Pzt', readTime: '~2dk',
        preview: 'Dünkü toplantıda aldığımız kararlar ve RBAC yetkilendirme modülünün son durumu hakkında kısa bir özet geçiyorum...',
        body: `<p>Toplantı Özeti:</p><ul><li>RBAC modülü admin paneline entegre edildi.</li><li>Müşteri geri bildirimleri değerlendirildi.</li><li>UI güncellemeleri tamamlandı.</li></ul>`,
        pinned: false, important: false, read: true,
        aiTopic: 'İş', aiIcon: '💼', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 6, sender: 'GitHub', email: 'noreply@github.com', accName: 'Dev', accColor: '#10b981', folder: 'inbox',
        subject: '[mailora-hub] Pull request #42: Feature/RBAC', time: '22 Eki', readTime: '~4dk',
        preview: 'A new pull request has been opened by dev-user. "Implemented Role-Based Access Control logic for admin endpoints".',
        body: `<div style="border:1px solid var(--border); padding: 15px; border-radius: 5px;">
            <h3><span style="color:#10b981">Open</span> Pull Request #42: Feature/RBAC</h3>
            <p>Implemented Role-Based Access Control logic for admin endpoints using custom Axum extractors.</p>
            <button style="padding:5px 10px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:3px; cursor:pointer; color:var(--text-primary);">View PR</button>
        </div>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Teknoloji', aiIcon: '💻', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 16, sender: 'Müşteri Hizmetleri', email: 'destek@shopping.com', accName: 'Personal', accColor: '#ef4444', folder: 'inbox',
        subject: 'Siparişiniz Kargoya Verildi - #TR982347', time: 'Cmt', readTime: '~1dk',
        preview: 'Siparişiniz MNG kargoya teslim edilmiştir. Kargo takip numarası ile sürecini izleyebilirsiniz.',
        body: `<p>Siparişiniz başarıyla kargoya verilmiştir.</p><p>Takip No: MNG-12345678</p>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Alışveriş', aiIcon: '🛒', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 8, sender: 'Canan Kaya', email: 'canan@company.com', accName: 'Work', accColor: '#3b82f6', folder: 'inbox',
        subject: 'Müşteri Görüşmesi: Proje X', time: '18 Eki', readTime: '~3dk',
        preview: 'Proje X için bugün yaptığımız görüşmenin notlarını paylaşıyorum. Müşteri arayüz tasarımını beğendi ancak...',
        body: `<p>Proje X Görüşme Notları:</p><p>Müşteri arayüz tasarımını çok beğendi. Ancak "Tablolar" kısmında bazı ek özellikler talep ediyorlar. İlgili eklentileri haftaya kadar hazırlamamız gerekiyor.</p>`,
        pinned: false, important: true, read: true,
        aiTopic: 'İş', aiIcon: '💼', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 5, sender: 'Unknown Sender', email: 'spam@freestuff.com', accName: 'Personal', accColor: '#ef4444', folder: 'spam',
        subject: 'Win a Free Cloud Server! Limited Time Offer!', time: '23 Eki', readTime: '~1dk',
        preview: 'Click here to claim your free lifetime cloud server instance today. No credit card required! Don\'t miss out on this...',
        body: `<h2 style="color:red">CONGRATULATIONS!</h2><p>You have been selected to win a free cloud server. <a href="#">Click here to claim</a>.</p>`,
        pinned: false, important: false, read: true,
        aiTopic: null, aiIcon: null, aiSpam: {score: 9, text: 'Spam Riski', color: '#ef4444'},
        hasAttachment: false
    },
    {
        id: 9, sender: 'Ben', email: 'info@company.com', accName: 'Work', accColor: '#3b82f6', folder: 'sent',
        subject: 'Yeni Tasarım Onayı Hakkında', time: '17 Eki', readTime: '~1dk',
        preview: 'Gönderdiğiniz son tasarımları inceledim. Login sayfası çok güzel olmuş ancak tablo kısmında revizyon gerekiyor.',
        body: `<p>Selamlar,</p><p>Son gönderdiğiniz tasarımları inceledim. Login sayfası çok güzel olmuş ancak tablo kısmında revizyon gerekiyor. Lütfen yarınki toplantıya kadar güncellemeleri tamamlayın.</p>`,
        pinned: false, important: false, read: true,
        aiTopic: 'İş', aiIcon: '💼', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 10, sender: 'Ben', email: 'dev@mailora.local', accName: 'Dev', accColor: '#10b981', folder: 'drafts',
        subject: 'Veritabanı Taşıma Planı', time: '15 Eki', readTime: '~2dk',
        preview: 'PostgreSQL geçiş planını bu haftasonu başlatmayı düşünüyorum. Aşağıdaki adımları...',
        body: `<p>PostgreSQL geçiş planını bu haftasonu başlatmayı düşünüyorum. Aşağıdaki adımları takip edeceğiz:</p><ul><li>Yedekleme</li><li>Downtime duyurusu</li><li>Migrasyon scripti...</li></ul>`,
        pinned: false, important: false, read: true,
        aiTopic: 'Teknoloji', aiIcon: '💻', aiSpam: null,
        hasAttachment: false
    },
    {
        id: 11, sender: 'Promo Mails', email: 'promo@fake-store.com', accName: 'Personal', accColor: '#ef4444', folder: 'trash',
        subject: 'Last chance for 90% discount!!!', time: '10 Eki', readTime: '~1dk',
        preview: 'Hurry up! The discount ends in 5 minutes.',
        body: `<p>Buy now or regret later!</p>`,
        pinned: false, important: false, read: true,
        aiTopic: null, aiIcon: null, aiSpam: {score: 8, text: 'Şüpheli', color: '#ef4444'},
        hasAttachment: false
    }
];

let state = {
    selectedMessageId: null,
    folderCollapsed: false,
    selectedAccount: null,
    selectedFolder: 'inbox',
    focusMode: false,
    analyticsOpen: false,
    searchQuery: ''
};

window.togglePin = function(id) {
    const msg = mockData.find(m => m.id === id);
    if(msg) msg.pinned = !msg.pinned;
    renderList();
};

window.toggleImportant = function(id) {
    const msg = mockData.find(m => m.id === id);
    if(msg) msg.important = !msg.important;
    renderList();
};

window.deleteMsg = function(id) {
    const idx = mockData.findIndex(m => m.id === id);
    if(idx > -1) {
        mockData.splice(idx, 1);
        if(state.selectedMessageId === id && mockData.length > 0) state.selectedMessageId = mockData[0].id;
        renderList();
        renderPreview();
    }
};

window.selectMsg = function(id) {
    state.selectedMessageId = id;
    const msg = mockData.find(m => m.id === id);
    if(msg && !msg.read) msg.read = true;
    renderList();
    renderPreview();
};

window.toggleFolder = function() {
    state.folderCollapsed = !state.folderCollapsed;
    const fl = document.getElementById('folder-list');
    const icon = document.getElementById('folder-toggle-icon');
    if (state.folderCollapsed) {
        fl.style.display = 'none';
        icon.style.transform = 'rotate(-90deg)';
    } else {
        fl.style.display = 'block';
        icon.style.transform = 'rotate(0deg)';
    }
};

const AI_API = 'http://localhost:5000';

window.showAI = async function() {
    const box = document.getElementById('ai-summary');
    if (!box) return;
    if (box.style.display !== 'none') { box.style.display = 'none'; return; }
    box.style.display = 'block';
    
    if(!document.getElementById('mock-style')) {
        document.head.insertAdjacentHTML('beforeend', '<style id="mock-style">@keyframes spin { 100% { transform: rotate(360deg); } }</style>');
    }
    
    box.innerHTML = `<div style="display:flex;align-items:center;gap:8px"><div class="spinner" style="width:14px;height:14px;border:2px solid var(--accent-blue);border-top-color:transparent;border-radius:50%;animation:spin 1s linear infinite"></div> <span style="font-size:13px;">MailoraPro Modelleri metni analiz ediyor...</span></div>`;
    
    const m = mockData.find(x => x.id === state.selectedMessageId);
    const textToAnalyze = m ? m.body.replace(/<[^>]*>?/gm, '') : '';

    try {
        const res = await fetch(`${AI_API}/analyze`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ text: textToAnalyze })
        });
        const data = await res.json();
        
        const duyguRenk = { 'Pozitif': '#10b981', 'Nötr': '#f59e0b', 'Negatif': '#ef4444' };
        const konuIcon = {
            is_proje: '💼', finans: '💰', alisveris: '🛒', teknoloji: '💻',
            pazarlama: '📈', kisisel: '👤', egitim: '🎓', seyahat: '✈️',
            hukuk_resmi: '⚖️', saglik: '🏥', sosyal_bildirim: '🔔', spor_eglence: '⚽'
        };

        const d = data.duygu || {};
        const k = data.konu || {};

        const duyguBars = d.scores ? Object.entries(d.scores).map(([label, score]) => {
            const color = duyguRenk[label] || '#64748b';
            return `<div style="display:flex;align-items:center;gap:6px;font-size:11px;margin-bottom:4px;">
                <span style="width:55px;color:${color}">${label}</span>
                <div style="flex:1;height:6px;background:var(--bg-tertiary);border-radius:3px;overflow:hidden">
                    <div style="width:${score}%;height:100%;background:${color};border-radius:3px;"></div>
                </div>
                <span style="width:40px;text-align:right;color:var(--text-muted)">${score}%</span>
            </div>`;
        }).join('') : '';

        const konuBars = k.scores ? Object.entries(k.scores).map(([label, score]) => {
            const icon = konuIcon[label] || '📌';
            return `<div style="display:flex;align-items:center;gap:6px;font-size:11px;margin-bottom:4px;">
                <span style="width:75px">${icon} ${label}</span>
                <div style="flex:1;height:6px;background:var(--bg-tertiary);border-radius:3px;overflow:hidden">
                    <div style="width:${score}%;height:100%;background:var(--accent-blue);border-radius:3px;"></div>
                </div>
                <span style="width:40px;text-align:right;color:var(--text-muted)">${score}%</span>
            </div>`;
        }).join('') : '';

        box.innerHTML = `
            <div style="display:flex;gap:24px;flex-wrap:wrap">
                <div style="flex:1;min-width:200px">
                    <div style="font-weight:600;margin-bottom:8px;font-size:13px;">🎭 Duygu Analizi
                        <span style="color:${duyguRenk[d.label] || '#fff'};font-weight:700;margin-left:8px">${d.label || '?'} (${d.confidence || 0}%)</span>
                    </div>
                    ${duyguBars}
                </div>
                <div style="flex:1;min-width:200px">
                    <div style="font-weight:600;margin-bottom:8px;font-size:13px;">${konuIcon[k.label] || '📌'} Konu Tahmini
                        <span style="color:var(--accent-blue);font-weight:700;margin-left:8px">${k.label || '?'} (${k.confidence || 0}%)</span>
                    </div>
                    ${konuBars}
                </div>
            </div>
            <div style="margin-top:16px;padding:12px;background:var(--bg-primary);border-radius:6px;border:1px solid var(--border)">
                <strong style="font-size:13px;">📝 Üretken Özet (MT5-Small)</strong>
                <div id="ai-mock-summary-content" style="margin-top:8px;font-style:italic;font-size:13px;"><div class="spinner" style="width:12px;height:12px;border:2px solid var(--accent-blue);border-top-color:transparent;border-radius:50%;animation:spin 1s linear infinite;display:inline-block;"></div> Özet çıkarılıyor...</div>
            </div>
        `;

        try {
            const sumRes = await fetch(`${AI_API}/summarize`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ text: textToAnalyze })
            });
            const sumData = await sumRes.json();
            document.getElementById('ai-mock-summary-content').innerHTML = sumData.error ? '<span style="color:red">Hata: ' + sumData.error + '</span>' : sumData.summary;
        } catch(err) {
            document.getElementById('ai-mock-summary-content').innerHTML = '<span style="color:red">Özet alınamadı.</span>';
        }

    } catch (err) {
        box.innerHTML = `<span style="color:red">Sunucuya bağlanılamadı: ${err.message}. (Python API çalışıyor mu?)</span>`;
    }
};

window.showTranslate = async function() {
    const box = document.getElementById('translate-box');
    if (!box) return;
    
    if (box.style.display === 'block' && !box.innerHTML.includes('spinner')) { 
        box.style.display = 'none'; 
        return; 
    }
    
    box.style.display = 'block';
    box.innerHTML = `<div style="display:flex;align-items:center;gap:8px"><div class="spinner" style="width:14px;height:14px;border:2px solid var(--accent-blue);border-top-color:transparent;border-radius:50%;animation:spin 1s linear infinite"></div> <span style="font-size:13px;">Helsinki-NLP modeli çeviriyor...</span></div>`;
    
    const m = mockData.find(x => x.id === state.selectedMessageId);
    const textToTranslate = m ? m.body.replace(/<[^>]*>?/gm, '') : '';

    const dirSelect = document.getElementById('translate-dir');
    const targetLang = dirSelect ? dirSelect.value : 'TR';

    // BURAYA DEEPL API ANAHTARINIZI GİRİN
    const DEEPL_API_KEY = 'cce5eaab-78ec-41d0-b7ef-b066ace5b0a5:fx';

    if (DEEPL_API_KEY === 'YOUR_DEEPL_API_KEY') {
        box.innerHTML = `<span style="color:var(--accent-red)">DeepL API anahtarı girilmedi. Lütfen mock-app.js içerisindeki DEEPL_API_KEY değişkenini güncelleyin.</span>`;
        return;
    }

    try {
        // Pro hesabı kullanıyorsanız 'api-free.deepl.com' yerine 'api.deepl.com' yazın.
        const res = await fetch('https://api-free.deepl.com/v2/translate', {
            method: 'POST',
            headers: { 
                'Authorization': `DeepL-Auth-Key ${DEEPL_API_KEY}`,
                'Content-Type': 'application/json' 
            },
            body: JSON.stringify({ 
                text: [textToTranslate], 
                target_lang: targetLang 
            })
        });
        const data = await res.json();
        
        if (data.translations && data.translations.length > 0) {
            box.innerHTML = `<strong style="font-size:13px;">🌍 Çeviri Sonucu (Yerel Model):</strong><br><br><div style="font-size:13px;">${data.translations[0].text}</div>`;
        } else {
            box.innerHTML = `<span style="color:var(--accent-red)">Hata: ${data.message || 'Çeviri alınamadı'}</span>`;
        }
    } catch (err) {
        box.innerHTML = `<span style="color:var(--accent-red)">Bağlantı hatası: ${err.message}.</span>`;
    }
};

function renderList() {
    let filteredData = mockData;
    
    // Filter by folder first
    filteredData = filteredData.filter(m => m.folder === state.selectedFolder);
    
    if (state.selectedAccount) {
        filteredData = filteredData.filter(m => m.accName === state.selectedAccount);
    }
    if (state.focusMode) {
        filteredData = filteredData.filter(m => m.important || m.pinned);
    }
    if (state.searchQuery) {
        const q = state.searchQuery.toLowerCase();
        filteredData = filteredData.filter(m => m.subject.toLowerCase().includes(q) || m.sender.toLowerCase().includes(q));
    }

    const html = filteredData.map(m => {
        const badges = [];
        badges.push(`<span class="badge" style="background:${m.accColor}20;color:${m.accColor};border:1px solid ${m.accColor}50">${m.accName}</span>`);
        if(m.pinned) badges.push('<span class="badge pin">📌</span>');
        if(m.important) badges.push('<span class="badge important">⭐</span>');
        if(m.hasAttachment) badges.push('<span class="badge attachment">📎</span>');
        if(m.aiTopic) badges.push(`<span class="badge ai-topic" style="background:var(--bg-tertiary);color:var(--text-primary)">${m.aiIcon} ${m.aiTopic}</span>`);
        if(m.aiSpam) badges.push(`<span class="badge ai-safe" style="background:${m.aiSpam.color}30;color:${m.aiSpam.color}">🛡️ ${m.aiSpam.score}/10 ${m.aiSpam.text}</span>`);

        return `<div class="msg-row ${state.selectedMessageId === m.id ? 'selected' : ''} ${!m.read ? 'unread' : ''}" onclick="selectMsg(${m.id})">
            <div class="msg-sender">${m.sender} ${badges.join('')}</div>
            <div class="msg-subject">${m.subject}</div>
            <div class="msg-preview">${m.preview}</div>
            <div class="msg-meta"><span class="msg-time">${m.time}</span><span class="msg-reading">${m.readTime}</span></div>
            <div class="msg-actions">
                <button class="act-btn" onclick="event.stopPropagation(); togglePin(${m.id})">${m.pinned ? '📌' : '📍'}</button>
                <button class="act-btn" onclick="event.stopPropagation(); toggleImportant(${m.id})">${m.important ? '⭐' : '☆'}</button>
                <button class="act-btn" onclick="event.stopPropagation()">⏰</button>
                <button class="act-btn" onclick="event.stopPropagation(); deleteMsg(${m.id})">🗑️</button>
            </div>
        </div>`;
    }).join('');
    document.getElementById('message-list').innerHTML = html || '<div class="empty-state">Mesaj yok</div>';
}

function renderPreview() {
    const m = mockData.find(x => x.id === state.selectedMessageId);
    if(!m) {
        document.getElementById('preview-pane').innerHTML = '<div class="empty-state"><div class="empty-icon">📭</div><div>Seçili e-posta yok</div></div>';
        return;
    }

    const aiActions = `
        <div class="preview-toolbar" style="margin-top: 15px; padding-top: 15px; border-top: 1px solid var(--border); display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:10px;">
            <button class="tool-btn" onclick="showAI()" style="padding: 6px 12px; font-size:13px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:4px; cursor:pointer; color:var(--text-primary);">🤖 AI Özet</button>
            <div style="display:flex; align-items:center; gap:5px;">
                <select id="translate-dir" style="padding: 5px 8px; font-size:12px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:4px; color:var(--text-primary); cursor:pointer;">
                    <option value="TR">İngilizce > Türkçe</option>
                    <option value="EN-US">Türkçe > İngilizce</option>
                </select>
                <button class="tool-btn" onclick="showTranslate()" style="padding: 6px 12px; font-size:13px; background:var(--accent-blue); border:none; border-radius:4px; cursor:pointer; color:white;">🌍 Çevir</button>
            </div>
        </div>
        <div id="ai-summary" class="ai-box" style="display:none; margin-top:15px; padding:15px; background:var(--bg-secondary); border-radius:6px; border:1px solid var(--border);"></div>
        <div id="translate-box" class="translate-box" style="display:none; margin-top:15px; padding:15px; background:var(--bg-secondary); border-radius:6px; border:1px solid var(--border);"></div>
    `;

    document.getElementById('preview-pane').innerHTML = `
        <div class="preview-header">
            <h2 class="preview-subject">${m.subject}</h2>
            <div class="preview-meta">
                <div class="preview-avatar" style="background:${m.accColor}; color:white; display:flex; align-items:center; justify-content:center; border-radius:50%; width:40px; height:40px; font-weight:bold; font-size:16px;">${m.sender.charAt(0)}</div>
                <div class="preview-sender-info">
                    <div class="preview-from"><strong>${m.sender}</strong> &lt;${m.email}&gt;</div>
                    <div class="preview-to">Kime: <strong>Tüm Ekip</strong> &lt;info@company.com&gt;</div>
                </div>
                <div class="preview-time">${m.time}</div>
            </div>
        </div>
        <div class="preview-body" style="padding: 20px; font-size: 14px; line-height: 1.6; color: var(--text-primary);">
            ${m.body}
            ${aiActions}
        </div>
    `;
}

document.addEventListener("DOMContentLoaded", () => {
    window.selectAccount = function(accName) {
        state.selectedAccount = accName;
        document.querySelectorAll('.account-item').forEach(el => el.classList.remove('active'));
        
        let titleSuffix = '';
        if(state.selectedFolder === 'inbox') titleSuffix = 'Inbox';
        else if(state.selectedFolder === 'sent') titleSuffix = 'Sent';
        else if(state.selectedFolder === 'drafts') titleSuffix = 'Drafts';
        else if(state.selectedFolder === 'spam') titleSuffix = 'Spam';
        else if(state.selectedFolder === 'trash') titleSuffix = 'Trash';

        if (accName === null) {
            document.querySelectorAll('.account-item')[0].classList.add('active');
            document.getElementById('list-title').textContent = 'Unified ' + titleSuffix;
        } else {
            const items = document.querySelectorAll('.account-item');
            for(let i=1; i<items.length; i++) {
                if(items[i].textContent.includes(accName)) { items[i].classList.add('active'); break; }
            }
            document.getElementById('list-title').textContent = accName + ' ' + titleSuffix;
        }
        renderList();
        renderPreview();
    };

    window.selectFolder = function(folderName, element) {
        state.selectedFolder = folderName;
        document.querySelectorAll('.folder-item').forEach(el => el.classList.remove('active'));
        if(element) element.classList.add('active');
        
        state.selectedMessageId = null;
        
        selectAccount(state.selectedAccount); // Re-trigger title change and render
    };

    // Accounts
    document.getElementById('account-list').innerHTML = `
        <div class="account-item active" style="border-bottom: 1px solid var(--border); padding-bottom: 12px; margin-bottom: 12px; cursor:pointer;" onclick="selectAccount(null)">
            <div class="account-dot" style="background:var(--text-muted)"></div>
            <span class="account-name" style="font-weight:600">Tüm Hesaplar</span>
        </div>
        <div class="account-item" style="cursor:pointer;" onclick="selectAccount('Work')"><div class="account-dot" style="background:#3b82f6"></div><span class="account-name">Work (info@company.com)</span></div>
        <div class="account-item" style="cursor:pointer;" onclick="selectAccount('Dev')"><div class="account-dot" style="background:#10b981"></div><span class="account-name">Dev (dev@mailora.local)</span></div>
        <div class="account-item" style="cursor:pointer;" onclick="selectAccount('Personal')"><div class="account-dot" style="background:#ef4444"></div><span class="account-name">Personal (me@gmail.com)</span></div>
        <div class="account-item" style="cursor:pointer;" onclick="selectAccount('School')"><div class="account-dot" style="background:#8b5cf6"></div><span class="account-name">School (student@university.edu)</span></div>
        <div class="account-item" style="cursor:pointer;" onclick="selectAccount('Finance')"><div class="account-dot" style="background:#f59e0b"></div><span class="account-name">Finance (finance@company.com)</span></div>
    `;

    // Folders Section Header (Collapsible)
    const folderList = document.getElementById('folder-list');
    if (folderList && folderList.previousElementSibling) {
        folderList.previousElementSibling.outerHTML = `
            <div class="section-label" id="folder-toggle" style="cursor: pointer; display: flex; justify-content: space-between; align-items: center;" onclick="toggleFolder()">
                <span>Klasörler</span>
                <span id="folder-toggle-icon" style="transition: transform 0.2s;">▼</span>
            </div>
        `;
    }

    // Folders List
    document.getElementById('folder-list').innerHTML = `
        <div class="folder-item active" onclick="selectFolder('inbox', this)" style="cursor:pointer"><span>📥</span><span>Inbox</span></div>
        <div class="folder-item" onclick="selectFolder('sent', this)" style="cursor:pointer"><span>📤</span><span>Sent</span></div>
        <div class="folder-item" onclick="selectFolder('drafts', this)" style="cursor:pointer"><span>📝</span><span>Drafts</span></div>
        <div class="folder-item" onclick="selectFolder('spam', this)" style="cursor:pointer"><span>⚠️</span><span>Spam</span></div>
        <div class="folder-item" onclick="selectFolder('trash', this)" style="cursor:pointer"><span>🗑️</span><span>Trash</span></div>
    `;

    renderList();
    renderPreview();
    
    // Wire up dummy analytics and focus functions used in index.html
    window.mailora = {
        search: (val) => { state.searchQuery = val; renderList(); },
        compose: () => { document.getElementById('compose-modal').style.display = 'flex'; },
        closeCompose: () => { document.getElementById('compose-modal').style.display = 'none'; },
        handleFileInput: () => { alert("Dosya ekleme simülasyonu"); },
        sendEmail: () => { alert("E-posta gönderildi!"); document.getElementById('compose-modal').style.display = 'none'; },
        toggleFocus: () => {
            state.focusMode = !state.focusMode;
            document.getElementById('focus-btn').textContent = state.focusMode ? '🎯 Focus: ON' : '🎯';
            renderList();
        },
        toggleAnalytics: () => {
            const drawer = document.getElementById('analytics-drawer');
            if (drawer) {
                state.analyticsOpen = !state.analyticsOpen;
                if(state.analyticsOpen) {
                    drawer.classList.add('open');
                    document.getElementById('stat-total').textContent = mockData.length * 42;
                    document.getElementById('stat-sent').textContent = 14;
                    document.getElementById('stat-received').textContent = mockData.length * 42 - 14;
                    document.getElementById('stat-response').textContent = '14dk';
                    document.getElementById('stat-top').innerHTML = '<div style="margin-bottom:5px">1. Work (%45)</div><div style="margin-bottom:5px">2. Dev (%30)</div><div>3. Personal (%25)</div>';
                } else {
                    drawer.classList.remove('open');
                }
            }
        }
    };
});
