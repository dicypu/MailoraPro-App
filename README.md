## 🎉 Mailora AI — Akıllı E-Posta İstemcisi

Mailora, yerel donanımda çalışan yapay zeka modelleriyle desteklenen **akıllı bir e-posta yönetim platformudur.** Konu sınıflandırma, spam tespiti, çeviri, özetleme, varlık çıkarımı ve akıllı yanıt üretimi gibi özellikleri tamamen **yerel ve çevrimdışı** olarak sunar.

---

### 🧠 1. Yapay Zeka Özellikleri

| Özellik | Model | Doğruluk / Bilgi |
|---|---|---|
| 📌 Konu Sınıflandırma (12 Kategori) | `dbmdz/bert-base-turkish-cased` | **%96.3** |
| 🛡️ Anti-Spam | Binary Classification | **%87** |
| 📝 Üretken Özetleme | `mrm8488/bert2bert_shared-turkish-summarization` | Encoder-Decoder |
| 🌍 Çevrimdışı Çeviri (EN↔TR) | `Helsinki-NLP/opus-tatoeba-en-tr`, `opus-mt-tr-en` | MarianMT |
| 🔍 Varlık Çıkarımı (NER) | `akdeniz27/bert-base-turkish-cased-ner` | Kişi, Kurum, Yer, Tarih, Para |
| ✨ Akıllı Yanıt Üretici | `google/mt5-small` (Fine-tuned) | Beam Search, 3 farklı yanıt |

---

### 🖥️ 2. Uygulama Sayfaları

| Sayfa | Dosya | Açıklama |
|---|---|---|
| 📥 Inbox | `index.html` | Ana e-posta listesi, AI Özet, Çeviri, NER, Akıllı Yanıt butonları |
| 📅 Takvim | `calendar.html` | Ay/Hafta görünümlü takvim, etkinlik oluşturma, renk kodları |
| 📊 Tablo (Sheets) | `sheets.html` | Google Sheets benzeri tablo, formül desteği (SUM/AVG/MAX/MIN), CSV export |
| 🛡️ Admin Panel | `admin.html` | Dashboard istatistikleri, sistem logları, kullanıcı yönetimi, AI raporları |

---

### ⚙️ 3. Mimari: Lazy Load Optimizasyonu

Tüm AI modelleri **Lazy Load** mimarisi ile çalışır:
1. Kullanıcı butona basana kadar GPU/RAM kullanımı **sıfırdır**.
2. Butona basıldığı anda model belleğe yüklenir (1-2 saniye).
3. İşlem tamamlanınca `torch.cuda.empty_cache()` ile VRAM anında boşaltılır.

---

### 💻 4. Kurulum ve Çalıştırma

#### Gereksinimler
- Python 3.10+
- CUDA destekli GPU (önerilir, opsiyonel)

#### Adım 1: Bağımlılıkları Kurun
```bash
cd MailoraPro
pip install transformers torch fastapi uvicorn datasets pandas scikit-learn sacremoses sentencepiece colorama
```

#### Adım 2: AI API Sunucusunu Başlatın (Terminal 1)
```bash
cd MailoraPro
python api_server.py
```
> Sunucu `http://localhost:5000` adresinden hizmet vermeye başlar.
> ⚠️ **Bu sunucu açık olmadan AI özellikleri (Çeviri, NER, Özet, Akıllı Yanıt) çalışmaz!**

#### Adım 3: Arayüz Sunucusunu Başlatın (Terminal 2 — Ayrı bir pencere açın)
```bash
cd Mailora
python -m http.server 8888
```

#### Adım 4: Tarayıcıda Açın
```
http://localhost:8888/static/index.html
```

Diğer sayfalar:
- Takvim: `http://localhost:8888/static/calendar.html`
- Tablo: `http://localhost:8888/static/sheets.html`
- Admin: `http://localhost:8888/static/admin.html`

---

### 📂 5. Proje Yapısı

```
MailoraPro Ecosystem/
├── Mailora/                    # Frontend (HTML/CSS/JS)
│   └── static/
│       ├── index.html          # Ana e-posta arayüzü
│       ├── calendar.html       # Takvim sayfası
│       ├── sheets.html         # Tablo (Sheets) sayfası
│       ├── admin.html          # Admin raporlama paneli
│       ├── css/premium.css     # Tasarım sistemi
│       └── js/                 # Bileşenler ve store
│
├── MailoraPro/                 # Backend (Python / AI)
│   ├── api_server.py           # FastAPI sunucusu (tüm AI endpointleri)
│   ├── egitim_konu_v3.py       # Konu modeli eğitim scripti
│   ├── egitim_yanit.py         # Smart Reply modeli eğitim scripti
│   ├── egitim_duygu.py         # Duygu analizi modeli eğitim scripti
│   ├── egitim_spam.py          # Spam modeli eğitim scripti
│   ├── veri_olustur.py         # Veri seti oluşturucu
│   ├── veri_zenginlestir.py    # Data augmentation scripti
│   └── Modeller/               # Eğitilmiş model ağırlıkları (.gitignore)
│
└── .gitignore
```

---

### 🔌 6. API Endpointleri

| Endpoint | Metod | Açıklama |
|---|---|---|
| `/analyze` | POST | Duygu + Konu analizi |
| `/translate` | POST | EN↔TR çeviri (MarianMT) |
| `/summarize` | POST | Üretken özetleme |
| `/extract-entities` | POST | NER (Kişi, Kurum, Yer) |
| `/smart-reply` | POST | 3 adet akıllı yanıt üretimi |
| `/health` | GET | Sunucu durumu kontrolü |
