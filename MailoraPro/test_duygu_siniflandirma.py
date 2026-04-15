import torch
from transformers import BertTokenizer, BertForSequenceClassification
import colorama
from colorama import Fore, Style

colorama.init(autoreset=True)

MODEL_YOLU = "./Modeller/Duygu_Modeli_Final"

print(Fore.CYAN + "Duygu Modeli Yükleniyor (Offline Mod)...")
tokenizer = BertTokenizer.from_pretrained(MODEL_YOLU, local_files_only=True)
model = BertForSequenceClassification.from_pretrained(MODEL_YOLU, local_files_only=True)

device = "cuda" if torch.cuda.is_available() else "cpu"
model.to(device)

test_cumleleri = [
    "Harika, siparişim yine yanlış geldi!",          # Beklenen: Negatif (İroni)
    "Oyun çok fena sarıyor, başından kalkamadım.",  # Beklenen: Pozitif (Argo/Mecaz)
    "Ankara, Türkiye'nin başkentidir.",             # Beklenen: Nötr (Bilgi)
    "Hayatımda izlediğim en berbat filmdi.",        # Beklenen: Negatif (Doğrudan)
    "Mükemmel paketleme, ürün paramparça olmuş."    # Beklenen: Negatif (İroni)
]

print(Fore.YELLOW + "Tahminler başlıyor...\n")
print("-" * 70)

etiket_haritasi = {0: "Negatif", 1: "Nötr", 2: "Pozitif"}

for metin in test_cumleleri:
    inputs = tokenizer(metin, return_tensors="pt", truncation=True, padding=True, max_length=128).to(device)
    
    with torch.no_grad():
        outputs = model(**inputs)
        
    tahmin_id = torch.argmax(outputs.logits, dim=-1).item()
    tahmin_metni = etiket_haritasi[tahmin_id]
    
    renk = Fore.RED if tahmin_id == 0 else (Fore.GREEN if tahmin_id == 2 else Fore.WHITE)
    
    print(f"Metin  : {metin}")
    print(f"Tahmin : {renk}{tahmin_metni}{Style.RESET_ALL}\n")
    print("-" * 70)