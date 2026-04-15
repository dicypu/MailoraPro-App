import os
import torch
import pandas as pd
from sklearn.model_selection import train_test_split
from transformers import BertTokenizer, BertForSequenceClassification, Trainer, TrainingArguments
from datasets import Dataset
import colorama
from colorama import Fore, Style

colorama.init(autoreset=True)

# --- AYARLAR ---
# DİKKAT: Buraya senin klasöründeki o EN SON checkpoint numarasını yaz!
# Sen checkpoint-7154 dedin ama klasörde daha yükseği varsa onu yaz.
SON_CHECKPOINT_YOLU = "./Modeller/Konu_Modeli_v1/checkpoint-7154" 
CIKIS_KLASORU = "./Modeller/Konu_Modeli_v1"

# --- KONTROL ---
if not os.path.exists(SON_CHECKPOINT_YOLU):
    print(Fore.RED + f"❌ HATA: '{SON_CHECKPOINT_YOLU}' bulunamadı!")
    print(Fore.RED + "Lütfen koddaki 'SON_CHECKPOINT_YOLU' kısmına doğru klasör adını yaz.")
    exit()

print(Fore.CYAN + "="*60)
print(Fore.CYAN + "🚀 EĞİTİM KURTARMA MODU: KALDIĞIMIZ YERDEN DEVAM EDİYORUZ")
print(Fore.CYAN + f"📂 Hedef Checkpoint: {SON_CHECKPOINT_YOLU}")
print(Fore.CYAN + "="*60 + "\n")

# --- STANDART AYARLAR ---
MODEL_ADI = "dbmdz/bert-base-turkish-cased"
MAX_UZUNLUK = 128
BATCH_SIZE = 16 # CPU için ideal

HEDEF_KATEGORILER = ["ekonomi", "spor", "siyaset", "teknoloji", "saglik"]
label2id = {label: i for i, label in enumerate(HEDEF_KATEGORILER)}
id2label = {i: label for label, i in label2id.items()}

# --- VERİLERİ TEKRAR OKU (Hafızaya Alması Lazım) ---
print(Fore.YELLOW + "📥 Veriler tekrar yükleniyor (Eğitim için şart)...")
all_texts = []
all_labels = []

YEREL_KLASORLER = ["VeriSetleri/TTC-3600_Orj", "VeriSetleri/Kemik_42k", "VeriSetleri/news"]
CSV_DOSYASI = "VeriSetleri/interpress.csv"

# Klasör Okuma
for ana_klasor in YEREL_KLASORLER:
    if os.path.exists(ana_klasor):
        for kat in HEDEF_KATEGORILER:
            olasi_yollar = [os.path.join(ana_klasor, kat), os.path.join(ana_klasor, kat.capitalize()), os.path.join(ana_klasor, kat.upper()), os.path.join(ana_klasor, "magazin") if kat == "teknoloji" else "yok"]
            for yol in olasi_yollar:
                if os.path.exists(yol) and os.path.isdir(yol):
                    for dosya in os.listdir(yol):
                        if dosya.endswith(".txt"):
                            try:
                                with open(os.path.join(yol, dosya), "r", encoding="utf-8", errors="ignore") as f:
                                    t = f.read().strip().replace("\n", " ")
                                    if len(t)>20: all_texts.append(t); all_labels.append(label2id[kat])
                            except: pass

# CSV Okuma
if os.path.exists(CSV_DOSYASI):
    try:
        df_csv = pd.read_csv(CSV_DOSYASI)
        df_csv.columns = [c.lower() for c in df_csv.columns]
        txt_col = 'content' if 'content' in df_csv.columns else 'text'
        cat_col = 'category' if 'category' in df_csv.columns else 'label'
        if txt_col in df_csv.columns and cat_col in df_csv.columns:
            for _, row in df_csv.iterrows():
                t = str(row[txt_col]); c = str(row[cat_col]).lower()
                lab = None
                if "ekonomi" in c: lab="ekonomi"
                elif "spor" in c: lab="spor"
                elif "siyaset" in c: lab="siyaset"
                elif "teknoloji" in c or "bilim" in c: lab="teknoloji"
                elif "sağlık" in c or "saglik" in c: lab="saglik"
                if lab and len(t)>20: all_texts.append(t); all_labels.append(label2id[lab])
    except: pass

print(Fore.GREEN + f"✅ Veri Hazır: {len(all_texts)} adet.")

# Dataset Hazırlığı
df = pd.DataFrame({"text": all_texts, "label": all_labels})
train_df, test_df = train_test_split(df, test_size=0.1, random_state=42)
tokenizer = BertTokenizer.from_pretrained(MODEL_ADI)
def tokenize_function(examples): return tokenizer(examples["text"], padding="max_length", truncation=True, max_length=MAX_UZUNLUK)
train_ds = Dataset.from_pandas(train_df).map(tokenize_function, batched=True)
test_ds = Dataset.from_pandas(test_df).map(tokenize_function, batched=True)

# --- KRİTİK NOKTA: MODELİ CHECKPOINT'TEN YÜKLE ---
print(Fore.YELLOW + "⏳ Yarım kalan model yükleniyor...")
model = BertForSequenceClassification.from_pretrained(
    SON_CHECKPOINT_YOLU, # <-- Buraya dikkat! Sıfırdan değil, checkpointten yüklüyoruz.
    num_labels=len(HEDEF_KATEGORILER),
    id2label=id2label,
    label2id=label2id
)

training_args = TrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    save_strategy="epoch",
    learning_rate=2e-5,
    per_device_train_batch_size=BATCH_SIZE,
    per_device_eval_batch_size=BATCH_SIZE,
    num_train_epochs=3, 
    weight_decay=0.01,
    load_best_model_at_end=True,
    use_cpu=True, # CPU Modu
    logging_steps=50
)

def compute_metrics(pred):
    labels = pred.label_ids
    preds = pred.predictions.argmax(-1)
    from sklearn.metrics import accuracy_score
    acc = accuracy_score(labels, preds)
    return {'accuracy': acc}

trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=train_ds,
    eval_dataset=test_ds,
    compute_metrics=compute_metrics,
)

print(Fore.MAGENTA + "\n🔥🔥 EĞİTİM KALDIĞI YERDEN (%60'tan) DEVAM EDİYOR... 🔥🔥")
# resume_from_checkpoint=True diyerek eğitimin tarihçesini de yüklüyoruz
trainer.train(resume_from_checkpoint=True) 

print(Fore.GREEN + "\n🎉 EĞİTİM %100 TAMAMLANDI!")
trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)