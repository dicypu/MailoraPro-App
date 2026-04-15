"""
Mailora AI — Konu Modeli v3 Eğitimi (12 Kategori, Dengeli Veri Seti, GPU ZORUNLU)
RTX 5080 ile bf16 modunda çalışır.

Kategoriler:
 0: İş/Proje    1: Finans     2: Alışveriş   3: Teknoloji
 4: Pazarlama   5: Kişisel    6: Eğitim      7: Seyahat
 8: Hukuk/Resmi 9: Sağlık    10: Sosyal     11: Spor/Eğlence

Çalıştır: python egitim_konu_v3.py
"""

import torch
import pandas as pd
from sklearn.model_selection import train_test_split
from transformers import BertTokenizer, BertForSequenceClassification, Trainer, TrainingArguments
from datasets import Dataset
from sklearn.metrics import accuracy_score, classification_report
import colorama
from colorama import Fore, Style

colorama.init(autoreset=True)

# ============ GPU KONTROLÜ (ZORUNLU) ============
if not torch.cuda.is_available():
    print(Fore.RED + "❌ GPU BULUNAMADI! Bu script sadece GPU ile çalışır.")
    print(Fore.RED + "PyTorch CUDA sürümünü yükleyin:")
    print(Fore.YELLOW + "pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu124")
    exit(1)

device = "cuda"
gpu_name = torch.cuda.get_device_name(0)
print(Fore.GREEN + f"✅ GPU: {gpu_name}")
print(Fore.GREEN + f"✅ VRAM: {torch.cuda.get_device_properties(0).total_memory / 1e9:.1f} GB")

# ============ AYARLAR ============
MODEL_ADI = "dbmdz/bert-base-turkish-cased"
CIKIS_KLASORU = "./Modeller/Konu_Modeli_v3"
VERI_DOSYASI = "./VeriSetleri/email_konu_v3_balanced.csv"
MAX_UZUNLUK = 128
BATCH_SIZE = 64  # RTX 5080 için optimal
NUM_EPOCHS = 5

KATEGORILER = [
    "is_proje", "finans", "alisveris", "teknoloji",
    "pazarlama", "kisisel", "egitim", "seyahat",
    "hukuk_resmi", "saglik", "sosyal_bildirim", "spor_eglence"
]

label2id = {label: i for i, label in enumerate(KATEGORILER)}
id2label = {i: label for label, i in label2id.items()}

print(Fore.CYAN + "=" * 60)
print(Fore.CYAN + f"🚀 KONU MODELİ v3 EĞİTİMİ — {len(KATEGORILER)} KATEGORİ")
print(Fore.CYAN + f"📊 GPU: {gpu_name} | Batch: {BATCH_SIZE} | Epochs: {NUM_EPOCHS}")
print(Fore.CYAN + "=" * 60)

# ============ VERİ YÜKLEME ============
print(Fore.YELLOW + f"\n📥 Veri yükleniyor: {VERI_DOSYASI}")

if not pd.io.common.file_exists(VERI_DOSYASI):
    print(Fore.RED + f"❌ Veri dosyası bulunamadı: {VERI_DOSYASI}")
    print(Fore.RED + "Önce veri_zenginlestir.py çalıştırın!")
    exit(1)

df = pd.read_csv(VERI_DOSYASI)
df = df.dropna()
df['text'] = df['text'].astype(str)
df['label'] = df['label'].astype(int)

print(Fore.GREEN + f"✅ Toplam veri: {len(df)} satır")
print("\nKategori dağılımı:")
for i, kat in enumerate(KATEGORILER):
    count = len(df[df['label'] == i])
    print(f"  {i:2d}. {kat:20s} → {count}")

# ============ TRAIN/TEST SPLIT ============
train_df, test_df = train_test_split(df, test_size=0.15, random_state=42, stratify=df['label'])
print(Fore.YELLOW + f"\nTrain: {len(train_df)} | Test: {len(test_df)}")

# ============ TOKENIZATION ============
print(Fore.CYAN + "⚙️  Tokenization başlıyor...")
tokenizer = BertTokenizer.from_pretrained(MODEL_ADI)

def tokenize_fn(examples):
    return tokenizer(examples["text"], padding="max_length", truncation=True, max_length=MAX_UZUNLUK)

train_ds = Dataset.from_pandas(train_df[['text', 'label']]).map(tokenize_fn, batched=True)
test_ds = Dataset.from_pandas(test_df[['text', 'label']]).map(tokenize_fn, batched=True)

# ============ MODEL ============
print(Fore.YELLOW + "🧠 Model yükleniyor...")
model = BertForSequenceClassification.from_pretrained(
    MODEL_ADI,
    num_labels=len(KATEGORILER),
    id2label=id2label,
    label2id=label2id,
    ignore_mismatched_sizes=True
).to(device)

# ============ EĞİTİM AYARLARI ============
args = TrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    save_strategy="epoch",
    learning_rate=2e-5,
    per_device_train_batch_size=BATCH_SIZE,
    per_device_eval_batch_size=BATCH_SIZE,
    num_train_epochs=NUM_EPOCHS,
    weight_decay=0.01,
    bf16=True,  # RTX 5080 bf16 donanım desteği
    logging_steps=10,
    load_best_model_at_end=True,
    metric_for_best_model="accuracy",
    report_to="none",
    warmup_ratio=0.1,
    gradient_accumulation_steps=1,
)

def compute_metrics(pred):
    labels = pred.label_ids
    preds = pred.predictions.argmax(-1)
    acc = accuracy_score(labels, preds)
    return {'accuracy': acc}

trainer = Trainer(
    model=model,
    args=args,
    train_dataset=train_ds,
    eval_dataset=test_ds,
    compute_metrics=compute_metrics,
)

# ============ EĞİTİM ============
print(Fore.MAGENTA + f"\n🔥 EĞİTİM BAŞLIYOR — {NUM_EPOCHS} EPOCH, GPU: {gpu_name}")
trainer.train()

# ============ DEĞERLENDİRME ============
print(Fore.YELLOW + "\n📊 Final değerlendirmesi...")
results = trainer.evaluate()
print(Fore.GREEN + f"✅ Doğruluk: {results['eval_accuracy']:.4f}")

# Detaylı rapor
preds = trainer.predict(test_ds)
y_pred = preds.predictions.argmax(-1)
y_true = preds.label_ids
print(Fore.CYAN + "\nDetaylı Sınıflandırma Raporu:")
print(classification_report(y_true, y_pred, target_names=KATEGORILER))

# ============ KAYIT ============
print(Fore.GREEN + f"\n💾 Model kaydediliyor: {CIKIS_KLASORU}")
trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)

print(Fore.GREEN + "=" * 60)
print(Fore.GREEN + "🎉 KONU MODELİ v3 HAZIR!")
print(Fore.GREEN + f"📁 {CIKIS_KLASORU}")
print(Fore.GREEN + f"📊 Doğruluk: {results['eval_accuracy']:.2%}")
print(Fore.GREEN + "=" * 60)
