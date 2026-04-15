import pandas as pd
import random
import os

print("=" * 60)
print("🚀 MAILORA AI - YANIT ÜRETİCİ VERİ SETİ OLUŞTURUCU")
print("=" * 60)

pairs = []

# Kalıplar: Gelen E-posta -> Potansiyel Yanıt
sablons = [
    # Alışveriş / Sipariş
    (["siparişiniz kargoya verildi", "paketiniz yola çıktı", "kargonuz dağıtıma çıktı"], 
     ["Bilgilendirme için teşekkür ederim.", "Teşekkürler, kargoyu bekliyorum.", "Harika, teşekkürler."]),
    (["iade talebiniz", "iadeniz onaylandı"], 
     ["Teşekkür ederim.", "Tutarın kartıma yansımasını bekliyorum.", "Bilgi için sağ olun."]),
    
    # İş / Toplantı
    (["toplantı yarın", "toplantı saat 14:00", "görüşme planlayalım"], 
     ["Anlaşıldı, toplantıda görüşmek üzere.", "Randevu takvimime ekliyorum.", "Teşekkürler, orada olacağım."]),
    (["rapor ektedir", "sunumu inceleyebilirsiniz", "dosya ektedir"], 
     ["Dosyayı aldım, inceleyip dönüş yapacağım.", "Teşekkürler, en kısa sürede göz atacağım.", "Aldım, teşekkürler."]),
     
    # Finans / Ödeme
    (["faturanız kesilmiştir", "ödeme hatırlatması", "hesabınıza para transferi"], 
     ["Faturayı teslim aldım.", "Ödemeyi gün içinde gerçekleştireceğim.", "Bilgilendirme için teşekkürler."]),
     
    # Teknik / Yazılım
    (["sunucu bakımı", "güncelleme yapılacak", "sistem kesintisi"], 
     ["Bilgilendirme için teşekkürler.", "Anlaşıldı.", "Önlem alıyoruz, teşekkürler."]),
     
    # Kişisel / Sosyal
    (["doğum günün kutlu olsun", "nice senelere", "iyi ki doğdun"], 
     ["Çok teşekkür ederim!", "Hatırlaman beni çok mutlu etti.", "Çok naziksin, teşekkürler! 😊"]),
    (["geçmiş olsun", "hastaymışsın"], 
     ["Çok teşekkür ederim, daha iyiyim.", "Sağ ol, dinleniyorum.", "Teşekkürler, geçiyor umarım."])
]

for inputs, outputs in sablons:
    for i in inputs:
        for _ in range(50):  # Veriyi çoğalt
            t_in = f"Merhaba, {i}. İyi günler dileriz." if random.choice([True, False]) else i
            t_out = random.choice(outputs)
            pairs.append({"text": f"yanıtla: {t_in}", "target": t_out})

# Birchok rasgele mailler de "Anlaşıldı, teşekkürler" ile yanıtlanabilir.
for _ in range(500):
    pairs.append({
        "text": f"yanıtla: {random.choice(['Bilgilendirme mailidir.', 'Aylık bülten.', 'Sipariş detayınız ektedir.'])}", 
        "target": random.choice(["Teşekkürler.", "Anlaşıldı.", "Bilgi için teşekkür ederim."])
    })

df = pd.DataFrame(pairs)
df = df.sample(frac=1, random_state=42).reset_index(drop=True)
os.makedirs("VeriSetleri", exist_ok=True)
df.to_csv("VeriSetleri/smart_reply_data.csv", index=False)
print(f"✅ Akıllı yanıt veri seti (Seq2Seq) hazır! Toplam: {len(df)} satır.")
