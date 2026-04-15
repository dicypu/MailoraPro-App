"""
Mailora AI — Spam Tespit Modeli Eğitimi (GPU ZORUNLU)
Binary classifier: 0 = Normal (Ham), 1 = Spam

HuggingFace'den Turkish spam email veri seti indirip eğitir.
Ek olarak sentetik phishing ve güvenli e-posta örnekleri ekler.

Çalıştır: python egitim_spam.py
"""

import torch
import pandas as pd
from sklearn.model_selection import train_test_split
from transformers import BertTokenizer, BertForSequenceClassification, Trainer, TrainingArguments
from datasets import Dataset, load_dataset
from sklearn.metrics import accuracy_score, classification_report
import colorama
from colorama import Fore

colorama.init(autoreset=True)

# ============ GPU KONTROLÜ ============
if not torch.cuda.is_available():
    print(Fore.RED + "❌ GPU BULUNAMADI! Bu script sadece GPU ile çalışır.")
    print(Fore.YELLOW + "pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu124")
    exit(1)

device = "cuda"
print(Fore.GREEN + f"✅ GPU: {torch.cuda.get_device_name(0)}")

# ============ AYARLAR ============
MODEL_ADI = "dbmdz/bert-base-turkish-cased"
CIKIS_KLASORU = "./Modeller/Spam_Modeli_v1"
BATCH_SIZE = 64

print(Fore.CYAN + "=" * 60)
print(Fore.CYAN + "🛡️  SPAM TESPİT MODELİ EĞİTİMİ (GPU)")
print(Fore.CYAN + "=" * 60)

# ============ VERİ TOPLAMA ============
all_texts = []
all_labels = []

# 1. HuggingFace Turkish Spam Dataset
print(Fore.YELLOW + "\n📥 HuggingFace spam veri seti indiriliyor...")
try:
    ds = load_dataset("anilguven/turkish_spam_email")
    df_hf = ds['train'].to_pandas() if 'train' in ds else pd.DataFrame()
    
    if len(df_hf) > 0:
        # Kolon isimlerini bul
        text_cols = [c for c in df_hf.columns if any(k in c.lower() for k in ['text', 'mail', 'content', 'message', 'body'])]
        label_cols = [c for c in df_hf.columns if any(k in c.lower() for k in ['label', 'class', 'spam', 'category'])]
        
        if text_cols and label_cols:
            tc = text_cols[0]
            lc = label_cols[0]
            for _, row in df_hf.iterrows():
                t = str(row[tc]).strip()
                l = row[lc]
                if isinstance(l, str):
                    l = 1 if 'spam' in l.lower() else 0
                if len(t) > 20:
                    all_texts.append(t[:512])
                    all_labels.append(int(l))
            print(Fore.GREEN + f"  ✅ HuggingFace verisi: {len(all_texts)} satır")
        else:
            print(Fore.YELLOW + f"  ⚠️ Kolon yapısı: {df_hf.columns.tolist()}")
    
except Exception as e:
    print(Fore.YELLOW + f"  ⚠️ HuggingFace indirilemedi: {e}")

# 2. Sentetik Spam E-postaları
print(Fore.YELLOW + "📝 Sentetik spam verileri ekleniyor...")

spam_samples = [
    "TEBRİKLER! 1.000.000 TL kazandınız! Ödülünüzü almak için hemen tıklayın.",
    "Hesabınız askıya alındı! 24 saat içinde şifrenizi güncellemezseniz kapatılacak.",
    "Bankadan acil mesaj: Kredi kartınız kopyalanmış olabilir. Doğrulamak için tıklayın.",
    "İnanılmaz fırsat! %90 indirimli iPhone sadece bugün! Stoklar tükeniyor!",
    "Sayın kullanıcı, vergi iadeniz hazır. TC kimlik numaranızı girerek alın.",
    "Lotoya katılmadan kazandınız! Bilgilerinizi göndererek ödülü alın.",
    "Acil: Hesabınızda şüpheli işlem tespit edildi. Güvenlik doğrulaması yapın.",
    "Ücretsiz tatil kazandınız! Detaylar için hemen formu doldurun.",
    "Şifreniz sıfırlandı. Bu siz değilseniz hemen buraya tıklayın.",
    "Devletten COVID yardım ödemesi! TC numaranızla başvurun.",
    "WhatsApp Gold sürümüne yükseltin! Premium özellikler ücretsiz.",
    "Son şans! Kredi borcunuz siliniyor! Sadece bugüne özel kampanya.",
    "Banka doğrulama: Kart bilgilerinizi güncellemeniz gerekmektedir.",
    "Dikkat! IP adresiniz yasaklanmak üzere. Hemen bilgilerinizi doğrulayın.",
    "Hemen zengin olun! Evden günde 5000 TL kazanma yöntemi.",
    "PTT kargonuz bekliyor. Gümrük ücreti ödemeniz gerekiyor: 89 TL.",
    "Netflix hesabınız donduruldu! Ödeme bilginizi güncelleyin.",
    "Kripto para ile milyoner olun! Garantili yatırım fırsatı kaçırmayın.",
    "Apple hesabınız kilitlendı. Güvenlik sorusunu cevaplayarak açın.",
    "Arkadaşın sana bir hediye gönderdi! Görmek için tıkla.",
    "Acil durum: E-posta kotanız doldu. Hesabınız silinecek.",
    "Özel davet: Gizli yatırım kulübüne katılın ve her ay 10.000 TL kazanın.",
    "SGK borcunuz siliniyor! Son gün bugün, başvurmak için tıklayın.",
    "Şüpheli giriş tespit edildi. Hesabınızı korumak için doğrulama yapın.",
    "E-devlet şifreniz değiştirildi. Bu işlemi siz yapmadıysanız bildirin.",
    "PayPal hesabınız sınırlandırıldı. Kimlik doğrulaması yapın.",
    "Türk Telekom'dan mesaj: Faturanız ödenmedi. Hattınız kapatılacak.",
    "Ücretsiz diploma! Online eğitimle sertifika alın, ücret yok.",
    "Hesabınıza 50.000 TL yatırıldı! Kontrol etmek için giriş yapın.",
    "Son 3 saat! 1 TL'ye telefon kampanyası. Kaçırmayın!",
    "E-posta güvenlik uyarısı: Şifreniz başkları tarafından bilinmektedir.",
    "Milyonluk miras! Uzak akrabanızdan size miras kaldı. İletişime geçin.",
    "Hemen başvur, anında onay! Koşulsuz kredi kampanyası.",
    "Dikkat: İnternet kotanız %99 doldu. Paket yükseltmek için tıklayın.",
    "Instagram takipçi kasma hilesi! 10.000 takipçi ücretsiz.",
    "Acil tıbbi yardım fonu: Bağış yaparak hayat kurtarın. Banka bilgileri:",
    "Konut kredisi faiz oranları sıfırlandı! Başvuru için bilgilerinizi girin.",
    "Amazon siparişiniz iade edildi. Tutarı almak için kart bilgisi girin.",
    "Trafik cezası bilgilendirmesi: 2.500 TL ceza kesildi. İtiraz için tıklayın.",
    "Vodafone: Hediye 50 GB internet kazandınız! Aktivasyon için tıklayın.",
]

ham_samples = [
    "Toplantı yarın saat 14:00'te gerçekleşecektir. Katılımınızı bekliyoruz.",
    "Ekte fatura detaylarını bulabilirsiniz. İyi günler dilerim.",
    "Proje ile ilgili son gelişmeleri paylaşmak istiyorum.",
    "Haftalık rapor ekte sunulmuştur. Geri bildirimlerinizi bekliyorum.",
    "Siparişiniz hazırlanmıştır. Kargo takip numaranız: TR1234567890.",
    "Doğum gününüz kutlu olsun! Size güzel bir yıl diliyorum.",
    "Randevunuz onaylanmıştır. Dr. Ayşe Yılmaz, 15 Nisan saat 14:30.",
    "Bu haftaki ders programı güncellenmiştir. Yeni saatleri kontrol ediniz.",
    "Uçuş bilgileriniz: TK1234, İstanbul → Ankara, 15 Nisan.",
    "Sözleşme taslağı incelemenize sunulmuştur.",
    "GitHub: Pull request #42 merged successfully.",
    "Hesap ekstreniz hazırlanmıştır. İnternet bankacılığınızdan görüntüleyebilirsiniz.",
    "Kafede buluşalım mı? Seni özledim, çok görüşemedik.",
    "Spor salonu üyeliğiniz yenilenmiştir.",
    "Yeni sezon koleksiyonu mağazalarda. %20 indirimle keşfedin.",
    "Kan tahlili sonuçlarınız hazırdır. Normal sınırlarda.",
    "Otel rezervasyonunuz onaylandı. Check-in: 15 Nisan.",
    "Kira ödemesi bu ay yapılmıştır. Dekont ektedir.",
    "LinkedIn: 5 kişi profilinize baktı bu hafta.",
    "Yeni blog yazımız yayınlandı: Verimli çalışma teknikleri.",
    "Sprint retrospektifi: Bu sprintte neler iyi gitti tartışacağız.",
    "Araç muayene randevunuz: 20 Nisan, saat 10:00.",
    "Sigorta poliçeniz yenilendi. Yeni teminat detayları ektedir.",
    "Çocuğun okul karnesi çok güzel geldi. Tebrikler!",
    "Basın açıklaması: Şirketimiz yeni ofisini açtı.",
    "Database migration başarıyla tamamlandı.",
    "Kargonuz teslim edildi. Değerlendirme yaparak puan kazanın.",
    "Ay sonu kapanış için muhasebe belgelerinizi hazırlayınız.",
    "Yoga dersi bu hafta Salı günü iptal edilmiştir.",
    "Akademik danışmanınız ile görüşme: Çarşamba 14:00.",
    "Server bakımı tamamlandı. Tüm sistemler normal çalışıyor.",
    "Aile buluşması bu Pazar öğle yemeğinde. Katılır mısınız?",
    "İş başvurunuz değerlendirilmektedir. Sonuç 2 hafta içinde bildirilecek.",
    "Yeni güncelleme yayınlandı: v3.2.1 performans iyileştirmeleri.",
    "Kiracı memnuniyet anketi: Görüşleriniz bizim için önemli.",
    "Tenis kortu rezervasyonunuz onaylandı: Cumartesi 16:00.",
    "E-fatura düzenlenmiştir. Fatura numarası: ML-2026-04567.",
    "Çalışma saatleri güncellendi: 09:00-17:30 olarak belirlenmiştir.",
    "Yeni çalışan oryantasyonu Pazartesi başlıyor.",
    "Film tavsiyesi: Dün izledim, bayıldım. Sana da tavsiye ederim.",
]

for t in spam_samples:
    all_texts.append(t)
    all_labels.append(1)  # Spam

for t in ham_samples:
    all_texts.append(t)
    all_labels.append(0)  # Ham

print(Fore.GREEN + f"  ✅ Sentetik veri eklendi: {len(spam_samples)} spam + {len(ham_samples)} ham")

# ============ VERİ SETİ OLUŞTUR ============
df = pd.DataFrame({"text": all_texts, "label": all_labels})
df = df.dropna().sample(frac=1, random_state=42).reset_index(drop=True)

print(Fore.GREEN + f"\n📊 Toplam veri: {len(df)}")
print(f"  Ham (Normal): {len(df[df['label']==0])}")
print(f"  Spam:         {len(df[df['label']==1])}")

# Kaydet
df.to_csv("./VeriSetleri/spam_veri.csv", index=False, encoding="utf-8")

# ============ TRAIN/TEST SPLIT ============
train_df, test_df = train_test_split(df, test_size=0.15, random_state=42, stratify=df['label'])

# ============ TOKENIZATION ============
print(Fore.CYAN + "⚙️  Tokenization başlıyor...")
tokenizer = BertTokenizer.from_pretrained(MODEL_ADI)
def tokenize_fn(examples):
    return tokenizer(examples["text"], padding="max_length", truncation=True, max_length=128)

train_ds = Dataset.from_pandas(train_df[['text','label']]).map(tokenize_fn, batched=True)
test_ds = Dataset.from_pandas(test_df[['text','label']]).map(tokenize_fn, batched=True)

# ============ MODEL ============
model = BertForSequenceClassification.from_pretrained(
    MODEL_ADI, num_labels=2,
    id2label={0: "ham", 1: "spam"},
    label2id={"ham": 0, "spam": 1},
    ignore_mismatched_sizes=True
).to(device)

# ============ EĞİTİM ============
args = TrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    save_strategy="epoch",
    learning_rate=3e-5,
    per_device_train_batch_size=BATCH_SIZE,
    num_train_epochs=5,
    bf16=True,
    logging_steps=10,
    load_best_model_at_end=True,
    metric_for_best_model="accuracy",
    report_to="none",
    warmup_ratio=0.1,
)

def compute_metrics(pred):
    labels = pred.label_ids
    preds = pred.predictions.argmax(-1)
    return {'accuracy': accuracy_score(labels, preds)}

trainer = Trainer(model=model, args=args, train_dataset=train_ds, eval_dataset=test_ds, compute_metrics=compute_metrics)

print(Fore.MAGENTA + "\n🔥 SPAM MODELİ EĞİTİMİ BAŞLIYOR...")
trainer.train()

# ============ DEĞERLENDİRME ============
results = trainer.evaluate()
print(Fore.GREEN + f"\n✅ Doğruluk: {results['eval_accuracy']:.4f}")

preds = trainer.predict(test_ds)
print(Fore.CYAN + "\nDetaylı Rapor:")
print(classification_report(preds.label_ids, preds.predictions.argmax(-1), target_names=["Ham", "Spam"]))

# ============ KAYIT ============
trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)

print(Fore.GREEN + "=" * 60)
print(Fore.GREEN + f"🛡️  SPAM MODELİ HAZIR: {CIKIS_KLASORU}")
print(Fore.GREEN + f"📊 Doğruluk: {results['eval_accuracy']:.2%}")
print(Fore.GREEN + "=" * 60)
