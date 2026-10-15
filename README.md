# Arunnel

Türkiye'den Discord'a tek tıkla erişim. **Yalnızca Discord trafiğine dokunur.** Oyunlar, ExitLag, tarayıcı ve diğer tüm programlar etkilenmez.

- Tek buton: aç, bağlan, kullan
- Arka planda çalışır (GoodbyeDPI gibi); pencere kapalıyken de bağlantı sürer
- Sesli sohbet, kamera ve ekran paylaşımı dahil
- İsteğe bağlı Roblox erişimi (Ayarlar > Roblox erişimi)
- VPN değildir, IP adresiniz değişmez, ping artmaz
- Kurulumlu ve kurulumsuz (taşınabilir) sürüm

## İndirme

[Releases](../../releases) sayfasından:

| Dosya | Açıklama |
|---|---|
| `Arunnel_1.1.0_Setup.exe` | Kurulum sihirbazı. Başlat menüsüne ve masaüstüne kısayol ekler. |
| `Arunnel.exe` | Kurulumsuz sürüm. Tek dosya, istediğiniz yere koyup çalıştırın. |

Gereksinim: Windows 10/11 (64 bit). Windows 11'de her şey hazırdır; eski Windows 10 sürümlerinde kurulum sihirbazı gerekirse WebView2'yi kendisi indirir.

## Kullanım

1. Arunnel'i çalıştırın (yönetici izni ister, bu normaldir).
2. Ortadaki butona basın. Birkaç saniye içinde yeşile döner.
3. Discord'u açın.

Pencereyi kapatınca uygulama saat yanındaki tepsiye küçülür; bağlantı devam eder. Tepsi ikonu yeşilse bağlısınız.

## Antivirüs uyarısı hakkında

Arunnel, ağ paketlerini işlemek için açık kaynaklı **WinDivert** sürücüsünü kullanır. Bazı antivirüs programları bu sürücüyü "riskli araç" olarak işaretler. Bu bir yanlış alarmdır; kodun tamamı bu depoda açıktır ve kendiniz derleyebilirsiniz.

## Nasıl çalışır?

Türkiye'deki Discord engeli iki katmanlıdır. Arunnel ikisini de yalnızca Discord için aşar:

| Engel | Çözüm |
|---|---|
| **DNS engeli**: Operatör DNS'i Discord için sahte adres verir | Discord adresleri şifreli DNS (DoH, 1.1.1.1) ile çözülür ve `hosts` dosyasına işaretli bir blok olarak yazılır. Sistem DNS ayarınız değişmez. Bağlantı kesilince blok silinir. |
| **SNI/DPI engeli**: Bağlantının başındaki alan adı görülüp bağlantı kesilir | [zapret](https://github.com/bol-van/zapret)'in `winws` motoru, yalnızca Discord alan adlarına giden ilk paketleri DPI'ın tanıyamayacağı şekilde böler/karıştırır. |

`winws`, `Arunnel` adlı bir Windows servisi olarak çalışır. Farklı operatörlerde farklı yöntemler işe yaradığı için Arunnel 6 hazır profili sırayla dener; çalışan profili hatırlar ve bir dahaki sefere önce onu kullanır.

### Neden diğer programlarla çakışmaz?

- Paket filtresi sadece `discord.txt` listesindeki alan adlarını ve Discord ses portlarını (UDP 50000-50099, 19294-19344) hedefler. Başka hiçbir bağlantıya müdahale edilmez.
- Sistem genelinde VPN, sanal ağ kartı, rota tablosu veya DNS ayarı değiştirilmez. Bu yüzden ExitLag, NoPing gibi programlar sorunsuz çalışmaya devam eder.
- Tanılama sayfası, gerçekten çakışabilecek programları (GoodbyeDPI, başka bir zapret, VPN) algılayıp gösterir.

### Eski araçların otomatik temizliği

Arunnel her açılışta ve her bağlanmada çakışabilecek kalıntıları temizler:

| Bulunan | Yapılan |
|---|---|
| GoodbyeDPI, zapret, ByeDPI servisleri | Durdurulur ve servis silinir (program dosyalarına dokunulmaz) |
| Cloudflare WARP, WireSock | Bağlantı kesilir, servis devre dışı bırakılır (program kaldırılmaz) |
| Elle açılmış goodbyedpi / winws / ciadpi | Kapatılır |
| Eski sürüm WinDivert sürücüleri | Kaldırılır |
| hosts dosyasındaki eski Discord kayıtları | Silinir |
| Çalışmayan yerel proxy ayarı | Kapatılır |

ExitLag gibi oyun hızlandırıcılara dokunulmaz. Yapılan her işlem Kayıtlar sayfasında görünür.

### Bilgisayarda neler değişir?

| Ne | Nerede | Ne zaman kaldırılır |
|---|---|---|
| Motor dosyaları | `C:\ProgramData\Arunnel\engine` | Ayarlar → Servisi kaldır |
| `Arunnel` Windows servisi | Servisler | Ayarlar → Servisi kaldır |
| WinDivert sürücüsü | Servisler | Ayarlar → Servisi kaldır |
| Discord DNS kayıtları | `C:\Windows\System32\drivers\etc\hosts` | Bağlantı kesilince |
| Açılışta başlatma görevi | Görev Zamanlayıcı → `Arunnel` | "Windows ile başlat" kapatılınca |
| Ayarlar ve kayıtlar | `C:\ProgramData\Arunnel` | Ayarlar → Servisi kaldır |

## Kaynak koddan derleme

Gerekenler: [Node.js](https://nodejs.org) 18+, [Rust](https://rustup.rs) (stable, MSVC) ve Visual Studio Build Tools ("C++ ile masaüstü geliştirme" iş yükü).

```bash
npm install
npm run build
```

Çıktılar `release/` klasörüne kopyalanır. Geliştirme modu için `npm run dev` (yönetici olarak açılmış bir terminalde). Font ve ikonlar depoda hazır gelir; yeniden üretmek isterseniz `npm run fonts` ve `npm run icons`.

`v` ile başlayan bir etiket gönderildiğinde (`git tag v1.0.0 && git push --tags`) GitHub Actions exe'leri derleyip Releases'a taslak olarak yükler.

Proje yapısı ve modüllerin ayrıntılı açıklaması için: [docs/MIMARI.md](docs/MIMARI.md)

## Sorun giderme

| Durum | Çözüm |
|---|---|
| "Eski sürücü hafızada kaldı" | Daha önce GoodbyeDPI/zapret kullanılmış. Bilgisayarı bir kez yeniden başlatın. |
| Bağlandıktan sonra ExitLag hata veriyor | ExitLag'de **Apply routes** yapın. |
| "Bağlanılamadı" | Tanılama sayfasındaki çakışmalara bakın; Kayıtlar sayfasından kayıtları kopyalayıp bir [issue](../../issues) açın. |
| Discord açılmıyor ama Arunnel yeşil | Discord'u tepsiden tamamen kapatıp yeniden açın. |

## Lisans

Arunnel [GPL-3.0-or-later](LICENSE) ile lisanslanmıştır. Kullanılan üçüncü taraf bileşenler ve lisansları: [THIRD_PARTY.md](THIRD_PARTY.md)

## Sorumluluk reddi

Bu yazılım, kişisel erişim amacıyla ve "olduğu gibi" sunulmaktadır. Kullanımından doğabilecek her türlü sorumluluk kullanıcıya aittir. Bulunduğunuz yerdeki yasal düzenlemeleri göz önünde bulundurun.
