"""
Mailora AI — 12 Kategori E-posta Veri Seti Oluşturucu
Her kategori için ~100 gerçekçi Türkçe e-posta metni üretir.
HuggingFace veri setlerini indirir ve birleştirir.

Çıktı: VeriSetleri/email_konu_v2.csv  (text, label)
"""

import pandas as pd
import os
import random

print("=" * 60)
print("📧 MAILORA E-POSTA VERİ SETİ OLUŞTURUCU v2")
print("=" * 60)

# 12 Kategori
KATEGORILER = [
    "is_proje",       # 0 - 💼
    "finans",         # 1 - 💰
    "alisveris",      # 2 - 🛒
    "teknoloji",      # 3 - 💻
    "pazarlama",      # 4 - 📢
    "kisisel",        # 5 - 👤
    "egitim",         # 6 - 🎓
    "seyahat",        # 7 - ✈️
    "hukuk_resmi",    # 8 - ⚖️
    "saglik",         # 9 - 🏥
    "sosyal_bildirim",# 10 - 🔔
    "spor_eglence",   # 11 - ⚽
]

# ============ SENTETİK E-POSTA VERİLERİ ============

data_is_proje = [
    "Merhaba ekibimiz, bu haftaki sprint toplantısı Cuma saat 14:00'te yapılacak. Gündem maddelerini önceden paylaşmanızı rica ederim.",
    "Proje ile ilgili son gelişmeleri paylaşmak istiyorum. Yeni sprint planı hazırlandı ve ekip toplantısı için tarih belirlememiz gerekiyor.",
    "Q3 raporunu incelemeniz için ekte paylaşıyorum. Geri bildirimlerinizi Pazartesi'ye kadar bekliyorum.",
    "Müşteri toplantısı yarın saat 10:00'da yapılacak. Sunum dosyasını gözden geçirmenizi rica ederim.",
    "Yeni feature branch açtım, code review'a hazır. PR linkini aşağıda bulabilirsiniz.",
    "Haftalık ilerleme raporu: Backend API'leri tamamlandı, frontend entegrasyonu devam ediyor.",
    "Deadline hatırlatması: Proje teslim tarihi 15 Nisan. Kalan görevleri kontrol edelim.",
    "Bugün yapılan standup toplantısının özeti: 3 task tamamlandı, 2 task devam ediyor.",
    "Yeni çalışan oryantasyonu için eğitim dokümanlarını hazırladım. İncelemenizi rica ederim.",
    "Müşteriden gelen değişiklik talepleri doğrultusunda scope güncellemesi yapıldı.",
    "Performans değerlendirme dönemi yaklaşıyor. Hedeflerinizi sisteme girmenizi rica ederim.",
    "Server migration planı hazır. Taşıma işlemi hafta sonu gerçekleştirilecek.",
    "Takım retrospektifi: Bu sprintte neler iyi gitti, neler iyileştirilebilir?",
    "KPI raporlarını yönetim kuruluna sunmak üzere hazırladım.",
    "Yeni iş ortağı ile NDA imzalandı. Proje detaylarını paylaşabiliriz.",
    "İş birliği toplantısı notu: Karşılıklı beklentiler belirlendi ve timeline onaylandı.",
    "Haftalık task dağılımı mail olarak paylaşılmıştır. Lütfen kendi görevlerinizi kontrol edin.",
    "Proje bütçesi revize edildi. Yeni bütçe tablosunu ekte bulabilirsiniz.",
    "Release v2.1 planlaması tamamlandı. QA testleri Perşembe başlayacak.",
    "İş süreçleri iyileştirme önerilerinizi bekliyorum. Toplantı Çarşamba saat 11:00.",
    "Departman toplantısı bu Salı saat 15:00'te yapılacaktır. Katılımınızı bekliyorum.",
    "Projeye yeni bir developer atandı. Onboarding sürecini başlatıyorum.",
    "Aylık ekip performans özeti: Hedeflerin %87'si tutturuldu.",
    "Müşteri memnuniyet anketi sonuçları hazır. Detayları toplantıda paylaşacağım.",
    "Configuration management için yeni prosedür dokümanı oluşturuldu.",
    "İş planı güncellendi, yeni milestonelar belirlendi. Onayınızı bekliyorum.",
    "Acil toplantı: Sistem kesintisi yaşandı, postmortem analiz yapılacak.",
    "Yeni CRM entegrasyonu için teknik gereksinimler belirlendi.",
    "Yarınki demo için hazırlıklarımızı tamamlamamız gerekiyor.",
    "Proje koordinasyon toplantısı: Her ekipten temsilci bekleniyor.",
    "Stajyer başvuruları değerlendirildi, mülakat tarihleri belirlendi.",
    "Yıllık strateji planı sunumu Cuma günü yapılacak.",
    "IT altyapı yenileme projesi kapsamında network düzenlemesi planlanıyor.",
    "Takım motivasyon etkinliği: Kahvaltı organizasyonu bu Cumartesi.",
    "Satış hedefleri güncellendi. Yeni hedefler mail eki olarak sunulmuştur.",
    "Yazılım lisans yenileme süreci başlatıldı. Onaylarınızı bekliyorum.",
    "Veritabanı optimizasyon çalışması tamamlandı. Performans %40 arttı.",
    "Yeni ofis düzeni hakkında görüşlerinizi almak istiyoruz.",
    "Şirket politikası güncellemesi: Uzaktan çalışma kuralları revize edildi.",
    "Sprint planning toplantısı: Product Owner öncelikleri belirleyecek.",
    "Tedarikçi görüşmeleri tamamlandı. En uygun teklifleri karşılaştırdım.",
    "DevOps pipeline CI/CD otomasyonu kuruldu. Deploy süreleri kısaldı.",
    "Veri yedekleme politikası güncellendi. Tüm sistemler yedekleniyor.",
    "Şirket genelinde güvenlik eğitimi zorunlu hale getirildi.",
    "Proje ilerleme raporu: %72 tamamlandı, kritik path sorunsuz ilerliyor.",
    "Son kullanıcı eğitimi takvimi paylaşıldı. Katılımcı listesini onaylayın.",
    "Ürün yol haritası 2026 Q3-Q4 planlaması için giriş toplantısı.",
    "Organizasyon şeması güncellendi. Yeni yapıyı intranet'te bulabilirsiniz.",
    "Bütçe kesintisi nedeniyle bazı projelerin önceliği değiştirildi.",
    "Hackathon duyurusu: 2 günlük inovasyon etkinliği düzenleniyor.",
]

data_finans = [
    "Sayın müşterimiz, Nisan ayı hesap ekstreniz hazırlanmıştır. Detaylar için internet bankacılığınızı kontrol ediniz.",
    "Elektrik faturanızın son ödeme tarihi 20 Nisan'dır. Otomatik ödeme talimatı vermek ister misiniz?",
    "Kredi kartı borcunuz 4.250,00 TL olarak kesinleşmiştir. Son ödeme tarihi: 15 Nisan.",
    "Hesabınıza 12.500,00 TL tutarında EFT gelmiştir. Gönderen: ABC Şirketi.",
    "Maaş ödemesi hesabınıza yatırılmıştır. Tutar: 28.750,00 TL.",
    "Vergi beyanname dönemi yaklaşıyor. Son gün 30 Nisan.",
    "Yatırım hesabınızdaki fonların aylık getiri raporu ekte sunulmuştur.",
    "Sigorta poliçenizin yenileme zamanı geldi. Yeni teklif için bizi arayın.",
    "Kira ödemesi bu ay henüz yapılmamıştır. Hatırlatma olarak bilgilendiriyoruz.",
    "Dönem sonu kapanış işlemleri için muhasebe belgelerinizi hazırlayınız.",
    "Banka hesap numaranıza yeni bir virüs koruması tanımlanmıştır.",
    "KDV iade işleminiz onaylanmıştır. Tutar hesabınıza 3 iş günü içinde aktarılacaktır.",
    "Kredi başvurunuz ön onay almıştır. Evrak tamamlama için şubeye uğrayınız.",
    "Fatura tutarınız: 1.890,00 TL. Online ödeme için link ektedir.",
    "Yıllık gelir vergisi beyannamesi hatırlatması. Son tarih 31 Mart.",
    "Portföy özetiniz: Hisse %3.2 artış, tahvil %1.4 artış, altın %0.8 düşüş.",
    "Hesap bakiyeniz minimum limitin altına düşmüştür. Lütfen yükleme yapınız.",
    "Gayrimenkul vergisi 1. taksit son ödeme tarihi: 31 Mayıs.",
    "Şirket bütçe planlaması için departman harcama raporlarını bekliyoruz.",
    "Nakit akış tablosu ve bilanço özeti ekte paylaşılmıştır.",
    "Borsa günlük özet: BIST100 %1.2 yükseldi, dolar 38.45 TL.",
    "E-fatura sistemi zorunluluğu hakkında bilgilendirme.",
    "İşletme gider raporu aylık bazda hazırlanmıştır.",
    "Emeklilik fonunuz için yıllık performans raporu ektedir.",
    "Ödemeniz alınmıştır. İşlem numarası: TXN-2026-04-12345.",
    "Hesap güvenliği: Son 24 saatte 3 başarısız giriş denemesi tespit edildi.",
    "Kurumsal kredi kartı limit artışı talebiniz onaylanmıştır.",
    "SGK prim borcu ödeme hatırlatması. Son gün: 30 Nisan.",
    "E-Devlet üzerinden vergi borcu sorgulaması yapabilirsiniz.",
    "Banka promosyon: 3 ay boyunca havale ve EFT ücretsiz.",
    "Yurt dışı para transferi işleminiz tamamlanmıştır.",
    "Mevduat faiz oranları güncellendi. Yeni oranlar için tıklayın.",
    "Kredi kartı puan kampanyası: 50 TL ve üzeri alışverişlerde 2x puan.",
    "İşletme hesabı açma başvurunuz onaylanmıştır.",
    "Dijital cüzdan uygulaması güncellendi. Yeni özellikler eklendi.",
    "Aylık aidat ödemeniz için hatırlatma: 500 TL, son gün 10 Nisan.",
    "Yatırım danışmanınızdan rapor: Portföy çeşitlendirme önerileri.",
    "Fatura iptal talebi işleme alınmıştır. Sonuç 2 iş günü içinde bildirilecek.",
    "Masraf raporu onay bekliyor. Toplam: 2.340 TL. Lütfen onaylayın.",
    "Bankacılık işlem limitleriniz güncellendi.",
]

data_alisveris = [
    "Siparişiniz kargoya verildi! Takip numarası: TR1234567890. Tahmini teslimat: 3-5 iş günü.",
    "Sipariş #304-1234567 kargoya verilmiştir. Tahmini teslimat tarihi: 12 Nisan.",
    "İade talebiniz onaylanmıştır. Ürünü 7 gün içinde kargoya vermenizi rica ederiz.",
    "Sepetinizde unuttuğunuz ürünler var! Stoklar tükenmeden sipariş verin.",
    "Siparişiniz teslim edildi. Değerlendirme yaparak 50 puan kazanın.",
    "Yeni sezon ürünleri mağazamıza eklendi. İndirimli fiyatlarla keşfedin.",
    "Kargonuz dağıtıma çıktı. Bugün saat 12:00-18:00 arası teslim edilecek.",
    "Ürün değişim talebiniz işleme alınmıştır. Yeni ürün 2 gün içinde kargoya verilecek.",
    "Garanti süresi dolan ürününüz için uzatılmış garanti seçeneği sunuyoruz.",
    "Hediye kartınız aktif edilmiştir. Bakiye: 250 TL. Son kullanma: 31 Aralık.",
    "Fiyat düşüş alarmı: Takip ettiğiniz ürünün fiyatı 200 TL düştü!",
    "Sipariş özetiniz: 3 ürün, toplam 1.450 TL. Ödeme başarıyla alındı.",
    "Kargo firması değişikliği: Siparişiniz artık Yurtiçi Kargo ile gönderilecek.",
    "Müşteri puanlarınız: 2.450 puan. 500 puan ile indirim kuponu elde edin.",
    "Ürün yorumunuz yayınlandı. Katkınız için teşekkürler!",
    "Mağazamız yenilendi! Yeni arayüzle alışveriş deneyiminizi keşfedin.",
    "Kupon kodunuz: BAHAR20 — Tüm ürünlerde %20 indirim, son gün yarın.",
    "Taksit seçenekleri güncellendi. 12 aya varan taksit imkanı!",
    "Kargonuz şubeye ulaştı. 3 gün içinde teslim alınmazsa iade edilecektir.",
    "Satıcıya mesajınız iletildi. 24 saat içinde yanıt verilecektir.",
    "Favori listenize eklediğiniz ürün tekrar stoka girdi!",
    "Alışveriş kredisi başvurunuz onaylandı. Limit: 5.000 TL.",
    "Paketiniz gümrükte beklemektedir. Takip numarası ile sorgulayabilirsiniz.",
    "Ücretsiz kargo kampanyası başladı! 200 TL üzeri siparişlerde geçerli.",
    "Ürün karşılaştırma: Sepetinizdeki ürünlerle benzer alternatifleri görün.",
    "Sipariş iptal talebiniz alınmıştır. İade işlemi 3-5 iş günü sürecektir.",
    "Yeni mağaza açıldı! Açılışa özel indirimler sizi bekliyor.",
    "Sadakat programına katıldınız! Her alışverişte puan kazanın.",
    "Kargo hasarlı geldi bildirimi alınmıştır. Fotoğrafları yükleyin.",
    "Ödeme hatırlatması: Kapıda ödeme siparişiniz yarın teslim edilecek.",
]

data_teknoloji = [
    "Yeni güncelleme yayınlandı: v3.2.1 — Performans iyileştirmeleri ve hata düzeltmeleri.",
    "GitHub Actions: Build başarıyla tamamlandı. Tüm testler geçti.",
    "Server CPU kullanımı %95'e ulaştı. Acil ölçeklendirme gerekiyor.",
    "API rate limit aşıldı. Günlük 10.000 istek limitiniz doldu.",
    "Docker container'ı yeniden başlatıldı. Uptime sıfırlandı.",
    "SSL sertifikası 15 gün içinde sona eriyor. Yenileme işlemini başlatın.",
    "Database migration başarıyla tamamlandı. Yeni tablolar oluşturuldu.",
    "CI/CD pipeline hatası: Test suite'te 3 başarısız test var.",
    "Yeni teknoloji stack değerlendirmesi: React vs Vue karşılaştırması.",
    "Cloud maliyetleri aylık raporunuz hazır. Toplam: 2.340 USD.",
    "Güvenlik yaması yayınlandı. Tüm sunuculara uygulanması gerekiyor.",
    "Yeni API endpoint'i deploy edildi: /api/v2/analytics",
    "Load balancer konfigürasyonu güncellendi. Traffic dağılımı optimize edildi.",
    "Kubernetes cluster auto-scaling devreye girdi. 3 yeni pod oluşturuldu.",
    "Code coverage raporu: %78 — Hedef: %85. Unit test eksikleri listelendi.",
    "Microservice architecture migration planı onay bekliyor.",
    "Redis cache hit oranı: %94. Cache stratejisi başarılı çalışıyor.",
    "Monitoring alert: Disk kullanımı %90'ı aştı. Temizleme gerekli.",
    "A/B test sonuçları: Yeni tasarım %12 daha yüksek conversion sağladı.",
    "Tech blog yazısı yayınlandı: Machine Learning ile e-posta sınıflandırma.",
    "NPM paketi güncellendi. Breaking change var, migration guide ektedir.",
    "Veri tabanı yedekleme işlemi başarıyla tamamlandı.",
    "WebSocket bağlantı hatası düzeltildi. Canlı bildirimler tekrar aktif.",
    "Yeni programlama dili Rust hakkında ekip içi sunum yapılacak.",
    "Sunucu bakım penceresi: Pazar 02:00-06:00 arası planlı kesinti.",
    "GraphQL şeması yeniden tasarlandı. Dokümantasyon güncellendi.",
    "Yapay zeka modelinin doğruluk oranı %94'e yükseltildi.",
    "Mobile app yeni sürümü App Store'da yayınlandı.",
    "Elasticsearch index optimizasyonu tamamlandı. Arama hızı 3x arttı.",
    "Open-source projeye katkı: Pull request kabul edildi.",
]

data_pazarlama = [
    "🎉 Bahar Kampanyası başladı! Tüm ürünlerde %50'ye varan indirim!",
    "Son 24 saat! Premium üyelikte %30 indirim fırsatını kaçırmayın.",
    "Haftalık bültenimize hoş geldiniz. Bu haftanın en çok okunan yazıları.",
    "Yeni koleksiyonumuz çıktı! İlk alışverişe özel %15 indirim kodu: YENI15",
    "Sadece bugün: 1 alana 1 bedava kampanyası! Stoklar sınırlı.",
    "E-posta aboneliğiniz onaylandı. Her hafta en iyi fırsatlar kapınızda.",
    "Doğum gününüz kutlu olsun! Size özel %25 indirim hediyemiz.",
    "Yaz indirimleri erken başladı! Seçili ürünlerde büyük fırsatlar.",
    "Arkadaşını getir kampanyası: Her davet için 100 TL kazanın.",
    "Flash Sale: 2 saatlik süper indirimler başlıyor!",
    "Newsletter: E-ticaret dünyasından son haberler ve trendler.",
    "Ücretsiz webinar: Dijital pazarlama stratejileri 2026.",
    "Kara Cuma erken erişim: VIP müşterilerimize özel fırsatlar.",
    "Yeni blog yazımız: 10 adımda müşteri memnuniyetini artırın.",
    "Anket katılım daveti: Görüşleriniz bizim için değerli. 5 dakikanızı ayırın.",
    "Promosyon kodu: WELCOME50 ile ilk siparişinizde 50 TL indirim.",
    "Sezon sonu büyük indirimler! Son fiyatlar üzerinden ekstra %20.",
    "Instagram'da bizi takip edin, çekilişe katılma şansı yakalayın!",
    "E-kitap hediye: Başarılı girişimcilerin hikayeleri — ücretsiz indirin.",
    "Müşteri sadakat programı güncellendi. Yeni avantajları keşfedin.",
    "Yılbaşı öncesi last minute fırsatları kaçırmayın!",
    "Podcast bölümümüz yayında: Yapay zeka ve geleceğin iş dünyası.",
    "Sosyal medya kampanyası sonuçları: 50K etkileşim, 5K yeni takipçi.",
    "Influencer iş birliği teklifi: Ürünlerimizi tanıtmak ister misiniz?",
    "SMS kampanyası raporu: %34 açılma oranı, %8 dönüşüm.",
    "Duyuru: Yeni mobil uygulamamız yayında! İndirene özel hediyeler.",
    "Landing page A/B test sonuçları raporlandı.",
    "Content marketing takvimi güncellendi. Nisan ayı konuları belirlendi.",
    "Marka işbirliği teklifi: Co-branding kampanyası fırsatı.",
    "Reklam bütçesi optimizasyonu: Google Ads ROI %45 arttı.",
]

data_kisisel = [
    "Merhaba canım, bu akşam yemeğe çıkmak ister misin? Yeni açılan restoran güzel görünüyor.",
    "Doğum günün kutlu olsun! Sana sağlıklı ve mutlu bir yıl diliyorum. 🎂",
    "Annecim, hafta sonu sizi ziyarete gelmek istiyoruz. Müsait misiniz?",
    "Tatil fotoğraflarını gördüm, harika görünüyorlar! Nasıl geçti anlatır mısın?",
    "Yarın akşam maç var, birlikte izleyelim mi? Ben bir şeyler hazırlarım.",
    "Uzun süredir görüşemedik. Bir kahve içmeye ne dersin?",
    "Taşınma işleri nasıl gidiyor? Yardıma ihtiyacın olursa haber ver.",
    "Çocukların okul resitali bu Perşembe. Katılabilir misin?",
    "Geçmiş olsun, nasıl hissediyorsun? İhtiyacın olan bir şey var mı?",
    "Bayram planların ne, bu sene köye gidecek misiniz?",
    "Düğün davetiyesini aldın mı? 15 Haziran'da, İstanbul'da.",
    "Kedinin veteriner randevusu yarın saat 10:00'da, unutma!",
    "Fotoğraf albümünü buldum, çocukluğumuzdan bir sürü resim var!",
    "Yeni ev çok güzel olmuş, tebrikler! Ev hediyesi için ne istersiniz?",
    "Bu hafta sonu piknik yapalım mı? Hava güneşli olacakmış.",
    "Sınav sonuçları açıklanmış, tebrikler! Çok başarılı bir sonuç.",
    "Tarif istemiştim, anneannenin börek tarifini gönderir misin?",
    "Yeni işin nasıl gidiyor? Alıştın mı?",
    "Eski fotoğrafları dijitalleştirdim, link paylaşıyorum.",
    "Anne baba evliliğin 30. yılı için sürpriz planlıyorum. Yardımcı olur musun?",
    "Randevunu saat 3'e aldırdım. Hastanenin adresini gönderiyorum.",
    "Köpek maması bitti, dönüşte alabilir misin?",
    "Yarınki doğum günü partisi için balon ve pasta hazır.",
    "Kuzenin askere gidiyor. Uğurlama töreni Pazar günü.",
    "Yoga dersine başladım, seninle gelmek ister misin?",
    "Çocuğun karnesi çok iyi geldi, gurur duydum!",
    "Film önerisi: Dün izledim, bayıldım. Sana da tavsiye ederim.",
    "Komşu teyze hasta, çorba götürsek iyi olur.",
    "Eski okul arkadaşları buluşması planlıyoruz. Katılır mısın?",
    "Yeni bebeğiniz için tebrikler! Ziyarete gelebilir miyim?",
]

data_egitim = [
    "Ödev hatırlatması: Lineer cebir ödevi 15 Nisan'a kadar teslim edilmelidir.",
    "Final sınav programı açıklandı. Detaylar için öğrenci portalını kontrol ediniz.",
    "Online ders materyalleri güncellendi. Yeni sunumlar erişime açılmıştır.",
    "Burs başvuru sonuçları açıklandı. Tebrikler, başvurunuz kabul edilmiştir!",
    "Staj başvurusu son tarihi: 20 Nisan. Başvuru formunu doldurmayı unutmayın.",
    "Yeni dönem ders kaydı başlıyor. Kontenjan sınırlı olduğu için erken kayıt yapın.",
    "Seminer duyurusu: Yapay Zeka ve Gelecek — Prof. Dr. Ahmet Yılmaz, 18 Nisan.",
    "Mezuniyet töreni tarihi ve yeri belirlendi. Davetiyeler hazırlanıyor.",
    "Kütüphane çalışma saatleri sınav döneminde uzatılmıştır: 08:00-24:00.",
    "Erasmus+ değişim programı başvuruları için son tarih 1 Mayıs.",
    "Ders notu paylaşımı: Veri yapıları ve algoritmalar — Hafta 8.",
    "Akademik danışman görüşme saatleri: Pazartesi ve Çarşamba 14:00-16:00.",
    "Proje ödevleri için grup oluşturma son tarihi: 10 Nisan.",
    "Yüksek lisans tez savunma tarihiniz belirlenmiştir: 25 Mayıs.",
    "Online sertifika programı tamamlandı. Sertifikanız ekte sunulmuştur.",
    "Workshop: Python ile Veri Analizi — Cumartesi 10:00-16:00.",
    "Kampüs etkinliği: Kariyer günleri 22-23 Nisan tarihlerinde düzenlenecek.",
    "Ders programı değişikliği: Matematik dersi Salı'dan Perşembe'ye alındı.",
    "Öğrenci kulübü toplantısı bu Cuma 17:00'de gerçekleşecek.",
    "Akademik makale yayınlandı: Turkish NLP benchmark çalışması.",
    "Lab deneyi raporu teslim tarihi: Bu Cuma saat 23:59.",
    "Yaz okulu kayıtları başladı. Erken kayıt indirimi var.",
    "Devamsızlık sınırına yaklaşıyorsunuz. Lütfen derslere katılın.",
    "Üniversite kütüphanesinde yeni kitaplar eklendi. Katalog güncellendi.",
    "Eğitim bursu kazandınız! Detaylar için burs ofisine başvurun.",
    "Mezunlar buluşması: Bu yıl 10. yıl kutlaması düzenleniyor.",
    "Çevrimiçi sınav kuralları güncellendi. Kamera zorunlu hale geldi.",
    "Öğretim üyesi ataması: Dr. Elif Kara bölümümüze katıldı.",
    "Bitirme projesi ara raporu teslim tarihi: 5 Mayıs.",
    "Eğitim platformu güncellendi. Yeni videolar eklendi.",
]

data_seyahat = [
    "Uçuş bilgileriniz: TK1234, İstanbul → Ankara, 15 Nisan saat 09:30.",
    "Otel rezervasyonunuz onaylandı. Check-in: 15 Nisan, Check-out: 18 Nisan.",
    "Araç kiralama onayı: 15-18 Nisan, Ankara Havalimanı teslim/iade.",
    "Vize başvurunuz onaylanmıştır. Pasaportunuzu konsolosluktan teslim alabilirsiniz.",
    "Seyahat sigortanız aktif edilmiştir. Poliçe detayları ekte sunulmuştur.",
    "Uçuş gecikmesi bildirimi: TK1234, yeni kalkış saati 11:00.",
    "Hotel değerlendirmesi yapın ve 500 bonus puan kazanın!",
    "Transferiniz ayarlandı. Şoför havalimanında sizi bekleyecek.",
    "Tur programı güncellendi. Yeni duraklar eklendi.",
    "Pasaport yenileme hatırlatması. Pasaportunuzun süresi 6 ay içinde doluyor.",
    "Cruise seferi bilgilendirmesi: Ege ve Akdeniz rotası, 7 gece.",
    "Bagaj hakkınız: 1 kabin + 1 check-in (23 kg). Fazla bagaj ücreti uygulanır.",
    "Otel spa rezervasyonu onaylandı. Saat: 15:00-17:00.",
    "Gümrük beyannamesi hatırlatması. Limit üzeri eşya bildirimi zorunludur.",
    "Acil durum kontakt bilgilerinizi güncelleyin.",
    "Tatil paketi fırsatı: Antalya 5 gece her şey dahil 8.500 TL.",
    "E-biletiniz hazır. Boarding pass QR kodunu ekte bulabilirsiniz.",
    "Seyahat planınız güncellendi. Yeni itinerary ektedir.",
    "Havaalanı lounge erişiminiz aktif edildi. Geçiş kodu: 4567.",
    "Döviz kuru bilgilendirmesi: 1 EUR = 41.20 TL. Seyahat bütçenizi planlayın.",
    "Yerel rehber atandı. İletişim bilgileri ektedir.",
    "Kayak tatili paketi: Uludağ 3 gece, lift kartı dahil.",
    "Otobüs bileti onayı: İstanbul-İzmir, 16 Nisan 22:00.",
    "Müze ve tarihi mekan giriş biletleriniz online alındı.",
    "Seyahat sağlık önerileri: Aşı gereksinimleri ve sağlık bilgileri.",
    "Avrupa rail pass aktive edildi. 30 günlük sınırsız tren yolculuğu.",
    "Camping rezervasyonu: Sapanca Gölü, 3 gece çadır alanı.",
    "Uçak bileti fiyat alarmı: İstanbul-London £89'a düştü!",
    "Yolculuk süresince Wi-Fi erişimi 5 EUR/saat üzerinden sunulmaktadır.",
    "Tatil konutu ev sahibi mesajı: Anahtar teslim detayları.",
]

data_hukuk = [
    "Sözleşme taslağı incelemenize sunulmuştur. 5 iş günü içinde geri dönüş bekliyoruz.",
    "Mahkeme duruşma tarihiniz belirlenmiştir: 20 Nisan 2026, saat 10:00.",
    "Yasal bildirim: Kira sözleşmenizin yenileme dönemi yaklaşmaktadır.",
    "Vekaletname hazırlanmıştır. İmza için notere gitmeniz gerekmektedir.",
    "Miras işlemleri için gerekli belgeler listesi ekte sunulmuştur.",
    "KVKK kapsamında kişisel veri işleme onayınız alınmıştır.",
    "İş sözleşmesi güncellenmesi hakkında bilgilendirme.",
    "Ticaret sicil kaydı onaylanmıştır. Resmi gazete ilanı yapılacaktır.",
    "Arabuluculuk başvurunuz kabul edilmiştir. İlk oturum: 25 Nisan.",
    "Tapu devir işlemi için randevu alınmıştır. Tarih: 18 Nisan.",
    "Vergi davası hakkında mahkeme kararı bildirilmiştir.",
    "Gizlilik sözleşmesi (NDA) imzalanması gerekmektedir.",
    "Tüketici hakem heyeti başvurunuz değerlendirilmektedir.",
    "Şirket kuruluş belgeleriniz hazırlanmıştır.",
    "İcra takibi hakkında bilgilendirme yazısı.",
    "Patent başvuru sonucu: Başvurunuz kabul edilmiştir.",
    "Marka tescil işlemi tamamlanmıştır. Tescil belgesi ektedir.",
    "Kamu ihale duyurusu: Proje için teklif verebilirsiniz.",
    "İşe iade davası hakkında avukatınızdan bilgi notu.",
    "Belediye imar planı değişikliği hakkında itiraz süresi başlamıştır.",
    "Noterden onaylı belgeniz hazırdır. Teslim alabilirsiniz.",
    "Dernek tüzüğü güncellenmesi hakkında genel kurul kararı.",
    "Yabancı uyruklu çalışan için çalışma izni başvurusu yapıldı.",
    "Kiracı tahliye ihtarnamesi tebliğ edilmiştir.",
    "Fikri mülkiyet hakları konusunda danışmanlık randevunuz ayarlandı.",
    "E-devlet üzerinden adli sicil kaydı sorgulama sonucunuz ektedir.",
    "Trafik cezası itiraz süreci başlatılmıştır.",
    "İşyeri açma ruhsatı başvurunuz onaylanmıştır.",
    "Kefalet sözleşmesi imza için hazırlanmıştır.",
    "Ticari dava dosyanız hakkında bilgilendirme.",
]

data_saglik = [
    "Randevunuz onaylanmıştır. Dr. Ayşe Yılmaz, 15 Nisan saat 14:30.",
    "Kan tahlili sonuçlarınız hazırdır. e-Nabız üzerinden görüntüleyebilirsiniz.",
    "İlaç hatırlatması: Günde 2 kez, sabah-akşam. İlacınızı düzenli kullanın.",
    "Sağlık sigortası poliçeniz yenilenmiştir. Yeni teminat detayları ektedir.",
    "Yıllık check-up raporu hazır. Genel sağlık durumunuz iyi.",
    "Diş hekimi randevunuz: 18 Nisan, saat 10:00. Kliniğe 15 dk erken gelin.",
    "Göz muayenesi sonuçları: Reçeteniz güncellendi. Gözlükçüye götürebilirsiniz.",
    "Aşı takvimi hatırlatması: Tetanos rapeli bu ay yapılmalıdır.",
    "Fizik tedavi seanslarınız başlamıştır. Haftada 3 gün, toplam 10 seans.",
    "Sağlıklı yaşam bülteni: Bu haftanın konusu bağışıklık güçlendirme.",
    "Ameliyat öncesi hazırlık bilgilendirmesi. Lütfen 8 saat aç kalınız.",
    "Eczane promosyonu: Vitamin takviyelerinde %30 indirim.",
    "Psikolojik danışmanlık randevunuz: 20 Nisan, Çarşamba saat 16:00.",
    "Laboratuvar sonuçları normal sınırlar içindedir. Kontrol 6 ay sonra.",
    "Online doktor görüşmesi: Video konsültasyon linki ektedir.",
    "Sağlık raporu kesilmiştir. E-devlet üzerinden erişebilirsiniz.",
    "Diyetisyen randevunuz hatırlatması: Bu Cuma saat 11:00.",
    "Organ bağışı kampanyası: Bağışçı olmak için bilgilenin.",
    "Hamilelik takip programı: Sonraki kontrol tarihi 25 Nisan.",
    "Hasta hakları hakkında bilgilendirme broşürü ektedir.",
    "Spor salonu sağlık raporu için kan tahlili gereklidir.",
    "Grip aşısı kampanyası başlamıştır. Randevu için tıklayın.",
    "Alerjik reaksiyon için acil ilaç reçetesi düzenlenmiştir.",
    "Sağlık ocağı çalışma saatleri güncellendi: 08:00-17:00.",
    "Rehabilitasyon programı ilerlemesi: %60 tamamlandı.",
    "Yeni tıbbi cihaz onayı alındı. Kullanım kılavuzu ektedir.",
    "Kronik hastalık takip programına kaydınız yapılmıştır.",
    "Evde bakım hizmeti başvurunuz onaylanmıştır.",
    "Sağlıklı beslenme listesi güncellendi. Detaylar ektedir.",
    "Acil durum iletişim bilgilerinizi güncellemenizi rica ederiz.",
]

data_sosyal = [
    "Mehmet Kaya paylaşımınızı beğendi. Görmek için tıklayın.",
    "Yeni bir arkadaşlık isteğiniz var: Ayşe Demir sizi arkadaş olarak ekledi.",
    "3 yeni bildiriminiz var. LinkedIn profilinize 15 kişi baktı.",
    "Twitter'da trend: #YapayZeka konusu gündemde. Katılmak ister misiniz?",
    "Instagram hikayenize 45 kişi baktı. Etkileşim raporu hazır.",
    "YouTube kanalınızda yeni yorum: 'Harika video, devamını bekliyoruz!'",
    "Discord sunucusunda yeni mesaj: Genel sohbet kanalında 12 yeni mesaj.",
    "WhatsApp grubuna davet edildiniz: 'Çalışma Grubu 2026'.",
    "Facebook etkinlik hatırlatması: Yarın saat 19:00'da konser.",
    "Slack kanalında bahsedildiniz: @kullanici sizden bahsetti.",
    "Reddit'te yorumunuz 50 upvote aldı!",
    "Twitch yayınınıza 23 yeni takipçi geldi. Teşekkür mesajı gönderin.",
    "Pinterest panonuza 8 yeni pin kaydedildi.",
    "Medium makаleniz 1000 okunmaya ulaştı. Tebrikler!",
    "Spotify çalma listeniz güncellendi: 5 yeni şarkı eklendi.",
    "Uygulama güncelleme bildirimi: Instagram v245.0 mevcut.",
    "Google Maps katkınız için teşekkürler: Yorumunuz yayınlandı.",
    "Zoom toplantı hatırlatması: Bug Review, 10 dakika sonra başlıyor.",
    "Telegram kanalı duyurusu: Önemli güncelleme yayınlandı.",
    "TikTok videonuz 10K görüntülemeye ulaştı!",
    "Steam: Wishlist'teki oyun indirime girdi. %60 indirimle satın alın.",
    "Duolingo hatırlatması: Bugün 15 dakika İngilizce öğrenme zamanı!",
    "Spotify Wrapped: Bu yılın en çok dinlediğiniz sanatçıları.",
    "GitHub: Repository'nize yeni bir star verildi.",
    "Apple: iCloud depolama alanınız doluyor. Yükseltme yapın.",
    "Netflix: Beğenebileceğiniz yeni dizi eklendi: 'Karanlık Oda'.",
    "Uber Eats sipariş bildirimi: Yemeğiniz hazırlanıyor.",
    "Google hesap güvenliği: Yeni cihazdan giriş yapıldı.",
    "Strava: Bu hafta 25 km koştunuz! Yeni kişisel rekor.",
    "BeReal bildirimi: Bugünkü anını paylaşma zamanı!",
]

data_spor = [
    "Galatasaray 3-1 Fenerbahçe. Maç özeti ve golleri izleyin.",
    "NBA sonuçları: Lakers 112-108 Celtics. LeBron 35 sayı attı.",
    "F1 yarış sonuçları: Verstappen birinci, Hamilton ikinci.",
    "Transfer dedikodusu: Yıldız oyuncu Süper Lig'e geliyor!",
    "Spor salonu üyeliğiniz yenilendi. Yeni program hazır.",
    "Türkiye Milli Takım kadrosu açıklandı. Avrupa Şampiyonası hazırlıkları.",
    "Antrenman programınız güncellendi. Bu hafta: Üst vücut ağırlık.",
    "Yüzme havuzu bakım duyurusu: 15-17 Nisan arası kapalı.",
    "Maraton kayıt onayı: İstanbul Maratonu, 3 Kasım. Bib numaranız: 4521.",
    "Champions League çeyrek final eşleşmeleri belli oldu.",
    "Yoga dersi takvimi güncellendi. Yeni saatler: MWF 07:00.",
    "Bisiklet turu etkinliği: İstanbul-Şile arası 80 km.",
    "Tennis kort rezervasyonu onaylandı: Cumartesi 16:00-17:00.",
    "E-spor turnuvası kayıtları açıldı. Valorant 5v5 formatı.",
    "Fitness tracker verileriniz: Bu hafta 42.000 adım, 320 kcal.",
    "Olimpiyat elemeleri sonuçları: Türk sporcu bronz madalya kazandı.",
    "Stadyum bileti satışa açıldı. Erken alım için indirimli fiyatlar.",
    "Futbol akademisi kayıtları başladı. 7-14 yaş arası.",
    "Golf kulübü turnuvası: 22 Nisan, 08:00'de başlayacak.",
    "Kayak sezonu kapanış etkinliği: Son gün 15 Nisan.",
    "Pilates dersi yeni grup açıldı. Kontenjan 12 kişi.",
    "Masa tenisi ligi fikstürü açıklandı. İlk maç bu Çarşamba.",
    "Dağcılık kulübü Ağrı Dağı tırmanışı planlanıyor.",
    "Yeni spor ekipmanları incelemesi: En iyi koşu ayakkabıları 2026.",
    "Beşiktaş taraftar kartı başvurunuz onaylanmıştır.",
    "Basketbol antrenmanı: Bu Pazar saat 10:00, Kapalı Spor Salonu.",
    "UFC fight night sonuçları ve analizleri.",
    "Kış sporları festivali: Snowboard ve kayak gösterileri.",
    "Profesyonel koç ile birebir antrenman seansı ayırtıldı.",
    "Çocuk futbol turnuvası kayıtları devam ediyor.",
]

# ============ BİRLEŞTİR ============
print("📝 Sentetik e-posta verileri oluşturuluyor...")

all_data = []
datasets = [
    (data_is_proje, 0), (data_finans, 1), (data_alisveris, 2),
    (data_teknoloji, 3), (data_pazarlama, 4), (data_kisisel, 5),
    (data_egitim, 6), (data_seyahat, 7), (data_hukuk, 8),
    (data_saglik, 9), (data_sosyal, 10), (data_spor, 11),
]

for texts, label in datasets:
    for t in texts:
        all_data.append({"text": t, "label": label})

df = pd.DataFrame(all_data)
df = df.sample(frac=1, random_state=42).reset_index(drop=True)

# Kaydet
output_path = "./VeriSetleri/email_konu_v2.csv"
df.to_csv(output_path, index=False, encoding="utf-8")

print(f"✅ Sentetik veri seti oluşturuldu: {len(df)} satır")
print(f"📁 Kaydedildi: {output_path}")
print("\nKategori dağılımı:")
for i, kat in enumerate(KATEGORILER):
    count = len(df[df['label'] == i])
    print(f"  {i:2d}. {kat:20s} → {count} örnek")

# ============ HUGGING FACE VERİ SETLERİ ============
print("\n" + "=" * 60)
print("📥 HuggingFace veri setleri indiriliyor...")

try:
    from datasets import load_dataset
    
    # 1. Interpress News (Türkçe haber — ekonomi, spor, sağlık, teknoloji, eğitim)
    print("  → interpress_news_category_tr_lite indiriliyor...")
    try:
        ds_news = load_dataset("interpress_news_category_tr_lite", trust_remote_code=True)
        df_news = ds_news['train'].to_pandas()
        
        # Kategori eşleştirme
        news_mapping = {
            "economy": 1, "ekonomi": 1,   # Finans
            "sports": 11, "spor": 11,     # Spor
            "health": 9, "sağlık": 9, "saglik": 9,  # Sağlık
            "technology": 3, "teknoloji": 3,  # Teknoloji
            "culture": 11, "kültür": 11,  # Spor/Eğlence
            "politics": 8, "siyaset": 8,  # Hukuk/Resmi
            "education": 6, "eğitim": 6, "egitim": 6,  # Eğitim
        }
        
        text_col = 'content' if 'content' in df_news.columns else 'text'
        cat_col = 'category' if 'category' in df_news.columns else 'label'
        
        if text_col in df_news.columns and cat_col in df_news.columns:
            mapped_rows = []
            for _, row in df_news.iterrows():
                cat = str(row[cat_col]).lower().strip()
                if cat in news_mapping:
                    mapped_rows.append({"text": str(row[text_col])[:512], "label": news_mapping[cat]})
            
            if mapped_rows:
                df_news_mapped = pd.DataFrame(mapped_rows)
                # Her kategoriden max 300 al
                df_news_balanced = df_news_mapped.groupby('label').apply(
                    lambda x: x.sample(min(300, len(x)), random_state=42)
                ).reset_index(drop=True)
                df = pd.concat([df, df_news_balanced], ignore_index=True)
                print(f"  ✅ Haber verisi eklendi: {len(df_news_balanced)} satır")
        else:
            print(f"  ⚠️ Kolon bulunamadı: {df_news.columns.tolist()}")
    except Exception as e:
        print(f"  ⚠️ Haber verisi atlandı: {e}")

    # 2. Turkish Spam Email
    print("  → turkish_spam_email indiriliyor...")
    try:
        ds_spam = load_dataset("anilguven/turkish_spam_email")
        df_spam = ds_spam['train'].to_pandas() if 'train' in ds_spam else pd.DataFrame()
        if len(df_spam) > 0:
            # Spam etiketli olanları Pazarlama kategorisine ekle
            text_col = [c for c in df_spam.columns if 'text' in c.lower() or 'mail' in c.lower() or 'content' in c.lower()]
            if text_col:
                spam_texts = df_spam[text_col[0]].dropna().tolist()[:200]
                spam_rows = [{"text": str(t)[:512], "label": 4} for t in spam_texts]  # 4 = pazarlama
                df = pd.concat([df, pd.DataFrame(spam_rows)], ignore_index=True)
                print(f"  ✅ Spam verisi eklendi: {len(spam_rows)} satır")
    except Exception as e:
        print(f"  ⚠️ Spam verisi atlandı: {e}")

except ImportError:
    print("  ⚠️ 'datasets' kütüphanesi yüklü değil. pip install datasets")

# ============ MEVCUT YEREL VERİLER ============
print("\n📂 Mevcut yerel veriler ekleniyor...")

# TTC-3600 ve Kemik verilerinden faydalanma
yerel_veri_eklendi = 0
for ana_klasor in ["VeriSetleri/TTC-3600_Orj", "VeriSetleri/Kemik_42k"]:
    if os.path.exists(ana_klasor):
        mapping = {
            "ekonomi": 1, "spor": 11, "siyaset": 8,
            "teknoloji": 3, "saglik": 9, "sağlık": 9,
            "kultur": 11, "kültür": 11, "magazin": 11,
        }
        for kat_adi, label_id in mapping.items():
            yol = os.path.join(ana_klasor, kat_adi)
            if not os.path.exists(yol):
                yol = os.path.join(ana_klasor, kat_adi.capitalize())
            if os.path.exists(yol) and os.path.isdir(yol):
                files = [f for f in os.listdir(yol) if f.endswith('.txt')]
                for f in files[:100]:  # Her kategoriden max 100
                    try:
                        with open(os.path.join(yol, f), "r", encoding="utf-8", errors="ignore") as fh:
                            t = fh.read().strip().replace("\n", " ")[:512]
                            if len(t) > 30:
                                df = pd.concat([df, pd.DataFrame([{"text": t, "label": label_id}])], ignore_index=True)
                                yerel_veri_eklendi += 1
                    except:
                        pass

print(f"  ✅ Yerel veri eklendi: {yerel_veri_eklendi} satır")

# ============ FİNAL ============
df = df.dropna().sample(frac=1, random_state=42).reset_index(drop=True)
df.to_csv(output_path, index=False, encoding="utf-8")

print(f"\n{'=' * 60}")
print(f"🎉 TOPLAM VERİ SETİ: {len(df)} satır")
print(f"📁 Kayıt: {output_path}")
print(f"\nFinal kategori dağılımı:")
for i, kat in enumerate(KATEGORILER):
    count = len(df[df['label'] == i])
    print(f"  {i:2d}. {kat:20s} → {count} örnek")
