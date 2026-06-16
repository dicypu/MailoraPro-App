// Mailora v2 — Mock Data (Demo Mode)
export const mockAccounts = [
    { id: 'acc1', email: 'ali@gmail.com', displayName: 'Gmail Kişisel', color: '#3b82f6', provider: 'gmail' },
    { id: 'acc2', email: 'ali@firma.com', displayName: 'İş', color: '#10b981', provider: 'outlook' },
    { id: 'acc3', email: 'ali@icloud.com', displayName: 'iCloud', color: '#8b5cf6', provider: 'icloud' },
];
export const mockMessages = [
    { id:'m1', accountId:'acc1', from:'John Doe', email:'john@example.com', subject:'Proje Güncellemesi Q7 2026 Haziran', preview:'Merhaba, proje ile ilgili son gelişmeleri paylaşmak istiyorum. Yeni sprint planı hazır.', body:'<p>Merhaba Ali,</p><p>Proje ile ilgili son gelişmeleri paylaşmak istiyorum. Yeni sprint planı hazırlandı ve ekip toplantısı için tarih belirlememiz gerekiyor.</p><p>Detaylı raporu ekte bulabilirsiniz.</p><br><p>Saygılarımla,<br>John Doe</p>', date:'2026-04-06T10:30:00', folder:'Inbox', pinned:true, important:true, read:false, hasAttachment:true, labels:['İş','Proje'] },
    { id:'m2', accountId:'acc1', from:'Mehmet Kaya', email:'mehmet@firma.com', subject:'Toplantı notu - Pazartesi', preview:'Pazartesi günkü toplantının notlarını paylaşıyorum.', body:'<p>Merhaba,</p><p>Pazartesi günkü toplantının notlarını aşağıda bulabilirsiniz:</p><ul><li>Bütçe onayı alındı</li><li>Tasarım revizyonu gerekli</li><li>Demo tarihi: 15 Nisan</li></ul>', date:'2026-04-06T09:15:00', folder:'Inbox', read:true, labels:['İş'] },
    { id:'m3', accountId:'acc2', from:'GitHub', email:'noreply@github.com', subject:'[mailora] Pull request #42 merged', preview:'Your pull request has been successfully merged into main branch.', body:'<p>Pull request <strong>#42</strong> has been merged into <code>main</code>.</p><p>Changes:<br>- Fix attachment handler<br>- Add payload limit middleware<br>- Update tests</p>', date:'2026-04-05T18:00:00', folder:'Inbox', read:false, important:true },
    { id:'m4', accountId:'acc1', from:'Amazon', email:'siparis@amazon.com.tr', subject:'Siparişinizin kargoya verildi', preview:'Sipariş #304-1234567 kargoya verilmiştir. Tahmini teslimat: 8 Nisan.', body:'<p>Sayın Ali,</p><p>Siparişiniz kargoya verilmiştir.</p><p><strong>Sipariş No:</strong> #304-1234567<br><strong>Tahmini Teslimat:</strong> 8 Nisan 2026</p>', date:'2026-04-05T14:20:00', folder:'Inbox', read:true, isNewsletter:true },
    { id:'m5', accountId:'acc2', from:'Ayşe Şahin', email:'ayse@firma.com', subject:'Tasarım dosyaları hazır', preview:'Figma linkini ve export dosyalarını ekliyorum. Geri bildirimini bekliyorum.', body:'<p>Merhaba Ali,</p><p>İstediğin tasarım dosyalarını hazırladım. Figma linkini aşağıda bulabilirsin:</p><p><a href="#">figma.com/mailora-v2</a></p><p>Export dosyaları da ekte.</p>', date:'2026-04-04T11:00:00', folder:'Inbox', read:false, hasAttachment:true, labels:['Tasarım'] },
    { id:'m6', accountId:'acc3', from:'Netflix', email:'info@netflix.com', subject:'Bu hafta yeni içerikler!', preview:'Bu hafta eklenen yeni film ve dizileri keşfedin.', body:'<p>Bu hafta Netflix\'te neler var?</p><ul><li>Yeni Dizi: Cyber Heist</li><li>Film: The Last Algorithm</li></ul>', date:'2026-04-03T08:00:00', folder:'Inbox', read:true, isNewsletter:true },
    { id:'m7', accountId:'acc1', from:'Hepsiburada', email:'bilgi@hepsiburada.com', subject:'Siparişiniz teslim edildi 📦', preview:'123456 nolu siparişiniz başarıyla teslim edilmiştir. Bizi tercih ettiğiniz için teşekkürler.', body:'<p>Merhaba, Samsung monitör siparişiniz başarıyla teslim edilmiştir. Ürünü değerlendirerek 50 Hepsipay Papeli kazanabilirsiniz.</p>', date:'2026-04-02T16:30:00', folder:'Inbox', read:true },
    { id:'m8', accountId:'acc2', from:'Ahmet Yılmaz', email:'ahmet@sirket.com', subject:'Hukuki Süreç Hakkında Bilgilendirme', preview:'Sözleşme güncellemeleri tamamlandı. Gözden geçirmenizi rica ederim.', body:'<p>Merhaba Ali Bey,</p><p>Hukuk departmanı olarak NDA ve çalışan sözleşmelerinin 2026 versiyonlarını güncelledik.</p><p>Ekteki taslakları inceleyip Pazartesi gününe kadar dönüş yapabilirseniz sevinirim.</p>', date:'2026-04-01T11:45:00', folder:'Inbox', read:false, hasAttachment:true, labels:['Hukuk'] },
    { id:'m9', accountId:'acc1', from:'Booking.com', email:'reservations@booking.com', subject:'Paris seyahatiniz için otel tavsiyeleri ✈️', preview:'Yaklaşan Paris geziniz için seçtiğimiz butik otellere göz atın.', body:'<p>Paris\'te harika bir tatil için sizin için seçtiğimiz butik oteller:</p><ul><li>Hotel De Louvre (9.5 Puan)</li><li>Le Petit Chateau (8.8 Puan)</li></ul>', date:'2026-03-30T09:20:00', folder:'Inbox', read:true, isNewsletter:true },
    { id:'m10', accountId:'acc1', from:'Spor Dünyası', email:'bulten@spordunyasi.com', subject:'Haftanın Maç Özetleri', preview:'Şampiyonlar ligi maç özetleri ve basketbol ligindeki son durum.', body:'<p>Şampiyonlar Ligi çeyrek final rövanş maçları nefes kesti. Real Madrid son dakika golüyle yarı finale çıktı.</p>', date:'2026-03-29T21:15:00', folder:'Inbox', read:true, isNewsletter:true },
];
export async function getAccounts() { return [...mockAccounts]; }
export async function getMessages(accountId, folder) {
    let msgs = [...mockMessages];
    if (accountId) msgs = msgs.filter(m => m.accountId === accountId);
    if (folder) msgs = msgs.filter(m => m.folder === folder);
    return msgs;
}
export async function getMessage(id) { return mockMessages.find(m => m.id === id) || null; }
export async function sendMessage(data) { return { success: true, id: 'sent_' + Date.now() }; }
export async function login(u, p) { return { token: 'demo_token', username: u, role: 'Admin' }; }
export async function register(u, p) { return { success: true }; }
export async function getFolders() { return ['Inbox','Sent','Drafts','Spam','Trash']; }
export async function getSettings() { return { database:{url:'sqlite://demo.db'}, imap:{server:'imap.demo',port:993}, smtp:{server:'smtp.demo',port:587} }; }
