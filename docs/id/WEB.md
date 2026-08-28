# Klien Peramban Lokal (Local Browser Client)

Perintah `ghosty web` membuka klien peramban (*browser client*) bawaan Ghosty melalui Runtime API kanonik. Klien ini merupakan permukaan lokal: server selalu terikat pada `127.0.0.1`, tidak dapat diikat ke alamat LAN, dan tidak dapat berjalan tanpa autentikasi Runtime.

---

## Cara Menjalankan

Dari ruang kerja yang ingin Anda kelola dengan Ghosty, jalankan:

```bash
ghosty web
```

Alamat bawaan adalah `http://127.0.0.1:7878`. Untuk menghindari bentrokan port lokal, pilih port loopback lain:

```bash
ghosty web --port 8788
```

Ghosty akan mengaktifkan Runtime API, menyajikan klien bawaan yang tersemat di dalam biner, dan meminta sistem operasi untuk membuka URL di peramban bawaan Anda. Hentikan proses dengan `Ctrl+C`.

---

## Batas Autentikasi & Keamanan

URL peluncuran peramban berisi kredensial bootstrap sekali pakai (*one-time bootstrap capability*) yang berumur pendek dan acak. Nilai ini tidak pernah menyimpan token penyedia atau kunci API ke dalam penyimpanan peramban (browser storage).
