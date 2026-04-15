import torch
import pandas as pd
from sklearn.model_selection import train_test_split
from transformers import BertTokenizer, BertForSequenceClassification, Trainer, TrainingArguments
from datasets import load_dataset, Dataset
import colorama
from colorama import Fore, Style

colorama.init(autoreset=True)

# --- AYARLAR ---
MODEL_ADI = "savasy/bert-base-turkish-sentiment-cased"
CIKIS_KLASORU = "./Modeller/Duygu_Modeli_v1"
MAX_UZUNLUK = 128
BATCH_SIZE = 32  # CPU için ideal

label2id = {"Negatif": 0, "Nötr": 1, "Pozitif": 2}
id2label = {0: "Negatif", 1: "Nötr", 2: "Pozitif"}

print(Fore.CYAN + "="*60)
print(Fore.CYAN + "🚀 DUYGU MODELİ EĞİTİMİ (CPU MODU - RYZEN 9800X3D)")
print(Fore.CYAN + "="*60 + "\n")

# --- 1. VERİ İNDİRME ---
print(Fore.YELLOW + "📥 Veri seti indiriliyor (Winvoker)...")
try:
    # İnternetten hazır veri seti çekiyoruz
    ds = load_dataset("winvoker/turkish-sentiment-analysis-dataset")
    df = ds['train'].to_pandas()
    
    # Etiketleri sayıya çevir
    mapping = {"Negative": 0, "Notr": 1, "Positive": 2}
    df['label'] = df['label'].map(mapping)
    
    # Temizlik
    df = df.dropna()
    print(Fore.GREEN + f"✅ Veri Hazır: {len(df)} satır.")

except Exception as e:
    print(Fore.RED + f"❌ Veri hatası: {e}")
    exit()

# --- 2. HAZIRLIK ---
train_df, test_df = train_test_split(df, test_size=0.1, random_state=42)

tokenizer = BertTokenizer.from_pretrained(MODEL_ADI)
def tokenize_function(examples):
    return tokenizer(examples["text"], padding="max_length", truncation=True, max_length=MAX_UZUNLUK)

train_ds = Dataset.from_pandas(train_df).map(tokenize_function, batched=True)
test_ds = Dataset.from_pandas(test_df).map(tokenize_function, batched=True)

print(Fore.YELLOW + "⚙️  Model CPU Modunda Hazırlanıyor...")
model = BertForSequenceClassification.from_pretrained(
    MODEL_ADI, num_labels=3, id2label=id2label, label2id=label2id, ignore_mismatched_sizes=True
)

training_args = TrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    save_strategy="epoch",
    learning_rate=2e-5,
    per_device_train_batch_size=BATCH_SIZE,
    per_device_eval_batch_size=BATCH_SIZE,
    num_train_epochs=2, # Duygu için 2 epoch yeterli
    weight_decay=0.01,
    use_cpu=True,       # 🔥 CPU Zorlaması
    logging_steps=100
)

def compute_metrics(pred):
    labels = pred.label_ids
    preds = pred.predictions.argmax(-1)
    from sklearn.metrics import accuracy_score
    acc = accuracy_score(labels, preds)
    return {'accuracy': acc}

trainer = Trainer(
    model=model, args=training_args, train_dataset=train_ds, eval_dataset=test_ds, compute_metrics=compute_metrics
)

# --- 3. BAŞLAT ---
print(Fore.GREEN + "\n🔥 EĞİTİM BAŞLIYOR...")
trainer.train()

print(Fore.GREEN + f"\n🎉 DUYGU MODELİ HAZIR: {CIKIS_KLASORU}")
trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)