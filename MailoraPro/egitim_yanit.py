"""
Mailora AI — Akıllı Yanıt Üretici (MT5 Seq2Seq) Eğitimi
"""

import torch
import pandas as pd
from sklearn.model_selection import train_test_split
from transformers import AutoTokenizer, MT5ForConditionalGeneration, Seq2SeqTrainer, Seq2SeqTrainingArguments, DataCollatorForSeq2Seq
from datasets import Dataset
import colorama
from colorama import Fore

colorama.init(autoreset=True)

if not torch.cuda.is_available():
    print(Fore.RED + "❌ GPU BULUNAMADI!")
    exit(1)

device = "cuda"
gpu_name = torch.cuda.get_device_name(0)
print(Fore.GREEN + f"✅ GPU: {gpu_name}")

# ============ AYARLAR ============
# base model olarak google/mt5-small veya yerel özetleyici kullanılabilir
MODEL_ADI = "google/mt5-small"
CIKIS_KLASORU = "./Modeller/Smart_Reply_Model"
VERI_DOSYASI = "./VeriSetleri/smart_reply_data.csv"
MAX_LENGTH_IN = 64
MAX_LENGTH_OUT = 32
BATCH_SIZE = 16  # seq2seq hafıza yoğundur
NUM_EPOCHS = 3

print(Fore.CYAN + "=" * 60)
print(Fore.CYAN + "🚀 AKILLI YANIT MODELİ (SEQ2SEQ) EĞİTİMİ")
print(Fore.CYAN + "=" * 60)

df = pd.read_csv(VERI_DOSYASI)
df = df.dropna()

train_df, test_df = train_test_split(df, test_size=0.1, random_state=42)

tokenizer = AutoTokenizer.from_pretrained(MODEL_ADI)

def preprocess_function(examples):
    inputs = [str(ex) for ex in examples["text"]]
    targets = [str(ex) for ex in examples["target"]]
    
    model_inputs = tokenizer(inputs, max_length=MAX_LENGTH_IN, truncation=True)
    labels = tokenizer(targets, max_length=MAX_LENGTH_OUT, truncation=True)
    
    model_inputs["labels"] = labels["input_ids"]
    return model_inputs

train_ds = Dataset.from_pandas(train_df).map(preprocess_function, batched=True)
test_ds = Dataset.from_pandas(test_df).map(preprocess_function, batched=True)

model = MT5ForConditionalGeneration.from_pretrained(MODEL_ADI).to(device)
data_collator = DataCollatorForSeq2Seq(tokenizer, model=model)

args = Seq2SeqTrainingArguments(
    output_dir=CIKIS_KLASORU,
    eval_strategy="epoch",
    learning_rate=3e-4,  # seq2seq için biraz yüksek lr iyidir
    per_device_train_batch_size=BATCH_SIZE,
    per_device_eval_batch_size=BATCH_SIZE,
    weight_decay=0.01,
    save_total_limit=1,
    num_train_epochs=NUM_EPOCHS,
    predict_with_generate=True,
    bf16=True, # RTX 5080
)

trainer = Seq2SeqTrainer(
    model=model,
    args=args,
    train_dataset=train_ds,
    eval_dataset=test_ds,
    tokenizer=tokenizer,
    data_collator=data_collator,
)

print(Fore.MAGENTA + f"\n🔥 EĞİTİM BAŞLIYOR — {NUM_EPOCHS} EPOCH")
trainer.train()

print(Fore.GREEN + f"\n💾 Model kaydediliyor: {CIKIS_KLASORU}")
trainer.save_model(CIKIS_KLASORU)
tokenizer.save_pretrained(CIKIS_KLASORU)

print(Fore.GREEN + "=" * 60)
print(Fore.GREEN + "🎉 AKILLI YANIT MODELİ HAZIR!")
print(Fore.GREEN + "=" * 60)
