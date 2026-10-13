# Üçüncü taraf bileşenler

Arunnel aşağıdaki açık kaynak bileşenleri içerir. `src-tauri/engine/` klasöründeki ikili dosyalar değiştirilmeden dağıtılmaktadır.

| Bileşen | Dosya | Lisans | Kaynak |
|---|---|---|---|
| zapret (winws) | `winws.exe` | MIT | https://github.com/bol-van/zapret |
| WinDivert | `WinDivert.dll`, `WinDivert64.sys` | LGPL-3.0 veya GPL-2.0 (çift lisans) | https://github.com/basil00/WinDivert |
| Cygwin DLL | `cygwin1.dll` | LGPL-3.0 (Cygwin bağlantı istisnası ile) | https://cygwin.com/git/newlib-cygwin.git |
| Tauri | uygulama çatısı | MIT / Apache-2.0 | https://github.com/tauri-apps/tauri |
| Inter | `ui/fonts/*.woff2` | SIL Open Font License 1.1 | https://github.com/rsms/inter |
| Tabler Icons (çizimler) | `ui/index.html` içindeki SVG'ler | MIT | https://github.com/tabler/tabler-icons |

## SHA-256 özetleri

Dağıtılan motor dosyalarının resmi sürümlerle aynı olduğunu doğrulamak için:

```
2da71e80878dc270ac83f5893ecbb841f9752a57f1da8ff9325636b4346bc632  winws.exe
103104a52e5293ce418944725df19e2bf81ad9269b9a120d71d39028e821499b  cygwin1.dll
16abd6a029e65557c6a309bea7b13bf81fff4e193567582e1cddbf6719f323e0  WinDivert.dll
8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2  WinDivert64.sys
```

Bu dosyalar zapret'in Windows sürüm paketinden (`binaries/windows-x86_64`) alınmıştır.
