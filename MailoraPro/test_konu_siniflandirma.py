from transformers import pipeline
import colorama
from colorama import Fore, Style

colorama.init(autoreset=True)

# Eğittiğimiz Final Model
MODEL_YOLU = "./Modeller/Konu_Modeli_v1"

print(Fore.CYAN + "🧠 KONU MODELİ YÜKLENİYOR... (Ryzen 9800X3D Eseri)")

try:
    # Modeli Yükle
    classifier = pipeline("text-classification", model=MODEL_YOLU, tokenizer=MODEL_YOLU)
    print(Fore.GREEN + "✅ Model Başarıyla Yüklendi!\n")
except Exception as e:
    print(Fore.RED + f"❌ HATA: Model yüklenemedi. Klasör boş olabilir.\nDetay: {e}")
    exit()

# Zorlu Test Cümleleri
testler = [
    "Fenerbahçe uzatmalarda bulduğu golle maçı 2-1 kazandı.",  # SPOR
    "Merkez Bankası faiz kararını açıkladı, dolar sert düştü.", # EKONOMİ
    "Meclis yeni yasayı onayladı, muhalefet tepki gösterdi.",   # SİYASET
    "Apple yeni iPhone 16 modelinde yapay zeka kullanacak.",    # TEKNOLOJİ
    "Grip salgınına karşı uzmanlardan maske uyarısı geldi.",    # SAĞLIK
    "Borsa İstanbul güne rekor yükselişle başladı."             # EKONOMİ (Zor)
]

print("-" * 60)
print(f"{'HABER METNİ':<50} | {'TAHMİN':<15} | {'GÜVEN'}")
print("-" * 60)

for metin in testler:
    sonuc = classifier(metin)[0]
    etiket = sonuc['label']
    puan = sonuc['score'] * 100
    
    # Renklendirme
    renk = Fore.WHITE
    if etiket == "spor": renk = Fore.GREEN
    elif etiket == "ekonomi": renk = Fore.YELLOW
    elif etiket == "siyaset": renk = Fore.RED
    elif etiket == "teknoloji": renk = Fore.BLUE
    elif etiket == "saglik": renk = Fore.MAGENTA
    
    # Ekrana Bas
    print(f"{metin[:45]}... | {renk}{etiket.upper():<15}{Style.RESET_ALL} | %{puan:.2f}")

print("-" * 60)