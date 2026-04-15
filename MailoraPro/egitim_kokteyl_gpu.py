import torch
from datasets import Dataset
from transformers import BertTokenizer, BertForSequenceClassification, Trainer, TrainingArguments
from sklearn.model_selection import train_test_split
import pandas as pd
import colorama
from colorama import Fore, Style
import os

colorama.init(autoreset=True)

# --- 1. GPU KONTROLÜ ---
device = "cuda" if torch.cuda.is_available() else "cpu"
print(Fore.GREEN + f"✅ RTX 5080 HAZIR VE NAZIR")

# --- 2. AYARLAR ---
MODEL_ADI = "savasy/bert-base-turkish-sentiment-cased"
CIKIS_KLASORU = "./Modeller/Duygu_Modeli_Final"
BATCH_SIZE = 64 # RTX 5080'in sınırlarını zorlayalım

# --- 3. YEREL VERİLERİ BİRLEŞTİRME (OFFLINE) ---
print(Fore.YELLOW + "📂 Yerel dosyalar harmanlanıyor...")

# 1. Aşı Verisi (Senin oluşturduğun ironi/argo seti)
ozel_df = pd.read_csv("./VeriSetleri/ozel_veri.csv")
ozel_df = pd.concat([ozel_df] * 100, ignore_index=True) # Aşı etkisini 100 kat artır

# 2. Haber Verisi (Duygusuz/Nötr metinler için)
haber_yolu = "./VeriSetleri/70000+_turkish_news/turkish_news_70000.csv"
if os.path.exists(haber_yolu):
    df_haber = pd.read_csv(haber_yolu)
    # Sadece metin kısmını al ve etiketini 1 (Nötr) yap
    text_col = 'main_text' if 'main_text' in df_haber.columns else 'text'
    df_haber = df_haber[[text_col]].rename(columns={text_col: 'text'})
    df_haber['label'] = 1 
    df_haber = df_haber.sample(10000) # 10 bin nötr haber yeterli
    
    # İki veriyi birleştir
    df_final = pd.concat([df_haber, ozel_df], ignore_index=True)
    print(Fore.GREEN + f"✅ Toplam {len(df_final)} satırlık veri seti oluşturuldu.")
else:
    print(Fore.RED + "❌ HATA: Haber dosyası bulunamadı! Yolu kontrol et."); exit()

df_final = df_final.dropna().sample(frac=1).reset_index(drop=True)
train_df, test_df = train_test_split(df_final, test_size=0.1, random_state=42)

# --- 4. TOKENIZATION ---
print(Fore.CYAN + "⚙️  Tokenization işlemi başlıyor...")
tokenizer = BertTokenizer.from_pretrained(MODEL_ADI)
def tokenize_fn(x): return tokenizer(x["text"], padding="max_length", truncation=True, max_length=128)

train_ds = Dataset.from_pandas(train_df).map(tokenize_fn, batched=True)
test_ds = Dataset.from_pandas(test_df).map(tokenize_fn, batched=True)

# --- 5. MODEL VE EĞİTİM (RTX 5080 MODU) ---
model = BertForSequenceClassification.from_pretrained(MODEL_ADI, num_labels=3, ignore_mismatched_sizes=True).to(device)

args = TrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    save_strategy="epoch",
    learning_rate=2e-5,
    per_device_train_batch_size=BATCH_SIZE if device == "cuda" else 16,
    num_train_epochs=3, 
    bf16=(device == "cuda"), # GPU varsa bf16, yoksa fp32
    use_cpu=(device != "cuda"),
    logging_steps=10,
    report_to="none"
)

trainer = Trainer(model=model, args=args, train_dataset=train_ds, eval_dataset=test_ds)

print(Fore.MAGENTA + "\n🚀 EĞİTİM BAŞLIYOR (OFFLINE)...")
trainer.train()

trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)
print(Fore.GREEN + "🏁 BİTTİ! Modeller/Duygu_Modeli_Final klasörünü kontrol et.")