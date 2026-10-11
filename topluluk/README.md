# GMID topluluk sunucuları

GMID Tool'daki **Sunucular** ekranında herkesin gördüğü "GMID" sunucuları bu klasördeki
`sunucular.json` dosyasından gelir. Dosya GitHub'da değiştirildiğinde herkesin uygulamasında
birkaç dakika içinde güncellenir — kimsenin bir şey indirmesi gerekmez.

## Sunucu eklemek

1. GitHub'da `topluluk/sunucular.json` dosyasını aç, sağ üstteki kalem (✏️) simgesine bas.
2. `"servers": []` köşeli parantezlerinin arasına sunucuyu yaz. `ornek` kısmındakileri kopyalayıp
   değiştirebilirsin (o kısım uygulamada görünmez, sadece örnek).
3. Birden fazla sunucu varsa aralarına virgül koy.
4. **Commit changes** ile kaydet.

```json
"servers": [
  { "id": "cod2", "name": "GMID · Call of Duty 2", "game": "cod2", "host": "185.12.34.56", "port": 28960 },
  { "id": "mc", "name": "GMID Survival", "game": "minecraft", "host": "mc.ornek.com" }
]
```

## Alanlar

| Alan | Gerekli mi | Açıklama |
|---|---|---|
| `id` | evet | Sunucunun kısa, değişmeyen adı (harf, rakam, `-`). Değiştirirsen yeni sunucu sayılır. |
| `name` | evet | Kutucukta görünen ad. |
| `game` | evet | Oyun (aşağıdaki tablodan). |
| `host` | evet | IP ya da alan adı. `185.12.34.56:28961` gibi port da yazılabilir. |
| `port` | hayır | Yazılmazsa oyunun varsayılan portu kullanılır. |
| `queryPort` | hayır | Sorgu portu oyun portundan farklıysa. |
| `logo` | hayır | Logo resmi (PNG/JPG/WebP/GIF, en fazla 2 MB). Bu klasördeki `logolar/` içine koyup `"logolar/cod2.png"` yazabilir ya da tam adres verebilirsin. |
| `description` | hayır | Kısa açıklama. |
| `connect` | hayır | Bağlanma notu (şifre nerede, beyaz liste vb.). |

## Oyunlar

| `game` | Oyun | Varsayılan port | Bağlan'a basınca |
|---|---|---|---|
| `cod2` | Call of Duty 2 | 28960 | Oyun açılır ve sunucuya girer |
| `cod4` | Call of Duty 4 | 28960 | Oyun açılır ve sunucuya girer |
| `coduo` | Call of Duty: United Offensive | 28960 | Oyun açılır ve sunucuya girer |
| `cod1` | Call of Duty | 28960 | Oyun açılır ve sunucuya girer |
| `minecraft` | Minecraft (Java) | 25565 | Adres kopyalanır |
| `minecraft-bedrock` | Minecraft (Bedrock) | 19132 | Sunucu Minecraft'a eklenir |
| `cs16` | Counter-Strike 1.6 | 27015 | Steam oyunu açıp sunucuya sokar |
| `cs2` | Counter-Strike 2 | 27015 | Steam oyunu açıp sunucuya sokar |
| `source` | Diğer Steam oyunları (TF2, GMod…) | 27015 | Steam oyunu açıp sunucuya sokar |
| `valheim` | Valheim | 2456 | Adres kopyalanır |
| `samp` | GTA San Andreas (SA-MP) | 7777 | SA-MP açılır |
| `mta` | MTA: San Andreas | 22003 | MTA açılır |
| `other` | Diğer | — | Adres kopyalanır |

Dosyada yazım hatası olursa uygulama "Topluluk listesi güncellenemedi" der ve eldeki listeyi
göstermeye devam eder; hatayı düzeltince kendiliğinden düzelir.
