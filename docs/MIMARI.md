# Mimari

```
arunnel/
├── ui/                      Arayüz (saf HTML/CSS/JS, derleme adımı yok)
│   ├── index.html           Sayfalar: Ana, Durum, Tanılama, Kayıtlar, Ayarlar
│   ├── style.css            Tüm görünüm; renkler :root değişkenlerinde
│   ├── app.js               Arayüz mantığı + tarayıcı önizlemesi için sahte arka uç
│   └── fonts/               Gömülü Inter fontu
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          Uygulama durumu, arayüz komutları, tepsi, arka plan döngüsü
│   │   ├── engine.rs        winws servisini kurma/başlatma/durdurma, strateji profilleri
│   │   ├── dns.rs           DoH ile çözümleme ve hosts dosyası bloğu
│   │   ├── probe.rs         Discord erişim testleri ve ping ölçümü
│   │   ├── settings.rs      Ayarlar (C:\ProgramData\Arunnel\settings.json)
│   │   ├── logs.rs          Bellek içi kayıt tamponu + arunnel.log
│   │   └── util.rs          Gizli komut çalıştırma, süreç listesi, veri klasörü
│   ├── engine/              exe'ye gömülen motor dosyaları
│   │   ├── winws.exe, cygwin1.dll, WinDivert.dll, WinDivert64.sys
│   │   ├── discord.txt      Müdahale edilecek alan adları (alt alan adları dahil)
│   │   ├── voice.txt        Discord ses paketleri için çekirdek filtresi
│   │   └── roblox.txt       Roblox alan adları (Ayarlar > Roblox erişimi açıkken)
│   ├── icons/               Uygulama ve tepsi ikonları (scripts/make-icons.mjs üretir)
│   ├── arunnel.exe.manifest Yönetici izni isteyen Windows manifesti
│   └── tauri.conf.json      Pencere ve kurulum sihirbazı ayarları
├── scripts/                 İkon üretimi, font kopyalama, release toplama
└── release/                 Derlenmiş exe'ler (git'e eklenmez)
```

## Bağlanma akışı

1. Gömülü motor dosyaları `C:\ProgramData\Arunnel\engine` klasörüne çıkarılır (değişmediyse dokunulmaz).
2. `dns.rs`, Discord alan adlarını 1.1.1.1 (yedek: 8.8.8.8) üzerinden DoH ile çözer ve `hosts` dosyasına yazar.
3. `engine.rs`, `Arunnel` servisini son çalışan profille başlatır.
4. `probe.rs`, `discord.com` ve `gateway.discord.gg` adreslerine TLS bağlantısı kurmayı dener.
5. Başarısızsa sıradaki profile geçilir. Hiçbiri çalışmazsa servis durdurulur, hosts temizlenir ve arayüz kırmızı "Bağlanılamadı" durumuna geçer.

## winws komut satırı

```
winws.exe --wf-tcp=443 --wf-udp=443 --wf-raw-part=@voice.txt
  --filter-tcp=443 --hostlist=discord.txt <PROFİL> --new
  --filter-udp=443 --hostlist=discord.txt --dpi-desync=fake --dpi-desync-repeats=6 --new
  --filter-udp=50000-50099,19294-19344 --filter-l7=discord,stun --dpi-desync=fake --dpi-desync-repeats=6
```

Ayarlarda "Roblox erişimi" açıksa her iki filtreye `--hostlist=roblox.txt` eklenir ve Roblox adresleri de DoH ile çözülüp hosts'a yazılır. Bağlıyken ayar değişirse servis aynı profille yeniden başlatılır.

`--hostlist` eşleşmeyen bağlantılar winws'ten değiştirilmeden geçer. Ses filtresi çekirdek seviyesinde yalnızca Discord'un IP keşif ve STUN paketlerini yakalar; oyun trafiği hiç winws'e ulaşmaz.

Yeni bir profil eklemek için `engine.rs` içindeki `PROFILES` dizisine satır eklemek yeterlidir.

## Arka planda çalışma

- Motor bir Windows servisidir; arayüz kapansa da çalışır.
- "Windows ile başlat" açıkken bağlıysanız servis `auto` başlangıç türüne alınır ve Görev Zamanlayıcı'ya oturum açılışında `Arunnel.exe --tray` görevini ekler (yönetici olarak, UAC sormadan).
- Bağlantıyı kesince servis `demand` (elle) türüne döner; açılışta kendiliğinden başlamaz.
- Arayüz açıkken 2 saniyede bir ping ölçülür, 10 saniyede bir servisin hâlâ çalıştığı kontrol edilir, 30 dakikada bir Discord adresleri tazelenir.

## Arayüzü tarayıcıda denemek

`ui/index.html` herhangi bir statik sunucuyla açılabilir. Tauri bulunamazsa `app.js` içindeki sahte arka uç devreye girer; bağlanma, grafik ve tanılama sahte verilerle çalışır.
