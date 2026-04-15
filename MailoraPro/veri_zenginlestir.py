import pandas as pd
import random
import os

print("=" * 60)
print("🚀 MAILORA AI - VERİ ZENGİNLEŞTİRİCİ v3 (AUGMENTATION)")
print("=" * 60)

# Okunacak Orijinal CSV
input_csv = "./VeriSetleri/email_konu_v2.csv"
output_csv = "./VeriSetleri/email_konu_v3_balanced.csv"

try:
    df_main = pd.read_csv(input_csv)
    print(f"✅ Orijinal veri seti yüklendi: {len(df_main)} satır")
except Exception as e:
    print(f"❌ Orijinal veri bulunamadı! {e}")
    exit()

KATEGORILER = {
    0: "is_proje", 1: "finans", 2: "alisveris", 3: "teknoloji",
    4: "pazarlama", 5: "kisisel", 6: "egitim", 7: "seyahat",
    8: "hukuk_resmi", 9: "saglik", 10: "sosyal_bildirim", 11: "spor_eglence"
}

# ================= TEMPLATE TABANLI VERİ ÜRETİMİ (Weak Classes) =================
# Zayıf sınıfların (2, 5, 6, 7, 10 vb.) dengelenmesi için
augmented_data = []

def generate_alisveris(count=400):
    urunler = ["telefon", "bilgisayar", "kulaklık", "ayakkabı", "tişört", "mont", "çanta", "kitap", "oyun konsolu", "saat"]
    durumlar = ["kargoya verildi", "teslim edildi", "iade talebi alındı", "iptal edildi", "dağıtıma çıktı", "hazırlanıyor"]
    firmalar = ["Trendyol", "Hepsiburada", "Amazon", "N11", "Yemeksepeti", "Getir", "Boyner"]
    for _ in range(count):
        u = random.choice(urunler)
        d = random.choice(durumlar)
        f = random.choice(firmalar)
        t = f"Sayın Müşterimiz, {f} üzerinden verdiğiniz {u} siparişiniz {d}. Takip numaranız: {random.randint(1000000, 9999999)}."
        augmented_data.append({"text": t, "label": 2})

def generate_kisisel(count=400):
    kisiler = ["Anneciğim", "Babacığım", "Kardeşim", "Ali", "Ayşe", "Mehmet", "Canım", "Dostum"]
    konular = ["hafta sonu yemeğe gidelim mi?", "doğum günün kutlu olsun, nice senelere!", "nasılsın, görüşmeyeli uzun zaman oldu?", 
               "yeni evin hayırlı olsun, çok sevindim.", "sürpriz için çok teşekkür ederim, harikaydı.", "düğün davetiyeni aldım, orada olacağım.",
               "hasta olduğunu duydum, çok geçmiş olsun.", "sınavı kazanmışsın tebrik ederim!", "akşam kahve içmeye ne dersin?"]
    for _ in range(count):
        k = random.choice(kisiler)
        c = random.choice(konular)
        t = f"{k}, {c} {random.choice(['Sevgiler.', 'Öpüyorum.', 'Görüşmek üzere.', 'Kendine iyi bak.'])}"
        augmented_data.append({"text": t, "label": 5})

def generate_egitim(count=400):
    icerikler = [
        "Final sınavı sonuçları açıklanmıştır.", "Ders kaydı için son tarih 5 Ekim.", "Ödevinizi sisteme yüklemeyi unutmayınız.",
        "Yeni akademik yıl akademik takvimi yayınlandı.", "Erasmus başvuru sonuçları için lütfen portalı ziyaret edin.",
        "Kütüphane kitap iade süreniz dolmuştur.", "Uzaktan eğitim linki e-kampüs sistemine eklendi.",
        "Vize mazeret sınavı programı belli oldu.", "Seminer katılım sertifikanız ektedir.", "Burs başvuru evraklarınızı tamamlayın."
    ]
    subeler = ["Bölüm Sekreterliği", "Öğrenci İşleri", "Rektörlük", "Danışman", "Enstitü"]
    for _ in range(count):
        t = f"Sayın Öğrenci, {random.choice(icerikler)} Bilgi için {random.choice(subeler)} ile iletişime geçebilirsiniz."
        augmented_data.append({"text": t, "label": 6})

def generate_seyahat(count=400):
    sehirler = ["İstanbul", "Ankara", "İzmir", "Londra", "Paris", "Berlin", "New York", "Roma", "Dubai", "Antalya"]
    islem = ["uçuşunuz", "otel rezervasyonunuz", "araç kiralamanız", "tur paketiniz", "vize işleminiz"]
    firmalar = ["THY", "Pegasus", "Booking.com", "Airbnb", "Avis", "Etstur"]
    for _ in range(count):
        s = random.choice(sehirler)
        i = random.choice(islem)
        f = random.choice(firmalar)
        t = f"{f} Bilgilendirmesi: {s} yönüne {i} basariyla onaylanmistir. Rezervasyon PNR: {random.randint(10000, 99999)}."
        augmented_data.append({"text": t, "label": 7})

def generate_sosyal(count=400):
    platformlar = ["Facebook", "Instagram", "Twitter", "LinkedIn", "TikTok", "YouTube", "Discord", "Twitch"]
    aksiyonlar = ["fotoğrafını beğendi.", "seni takip etmeye başladı.", "gönderine yorum yaptı.", 
                  "yeni bir hikaye paylaştı.", "sana mesaj gönderdi.", "canlı yayına başladı."]
    for _ in range(count):
        p = random.choice(platformlar)
        a = random.choice(aksiyonlar)
        t = f"{p}: Kullanıcı{random.randint(100,999)} {a} Görmek için hemen uygulamaya dön."
        augmented_data.append({"text": t, "label": 10})

def generate_pazarlama(count=400):
    kelimeler = ["indirim", "fırsat", "kampanya", "son şans", "bedava", "yeni sezon", "hediye", "çekiliş"]
    oranlar = ["%10", "%20", "%50", "%70"]
    for _ in range(count):
        t = f"📣 Kaçırılmayacak {random.choice(kelimeler)}! Sadece bugüne özel tüm ürünlerde {random.choice(oranlar)} indirim. Hemen alışverişe başla."
        augmented_data.append({"text": t, "label": 4})

# Üretimi Başlat
generate_alisveris(400)
generate_kisisel(400)
generate_egitim(400)
generate_seyahat(400)
generate_sosyal(400)
generate_pazarlama(300)

df_augmented = pd.DataFrame(augmented_data)
print(f"🎯 Zayıf sınıflar için özel sentetik şablon verisi üretildi: {len(df_augmented)} satır")

df_final = pd.concat([df_main, df_augmented], ignore_index=True)
df_final = df_final.sample(frac=1, random_state=42).reset_index(drop=True)

df_final.to_csv(output_csv, index=False)
print(f"🚀 Veri seti zenginleştirildi ve birleştirildi!")
print(f"📂 Kayıt edilen dosya: {output_csv}")
print(f"Toplam Veri Sayısı: {len(df_final)}")
