# Menginstal Ghosty

Halaman ini mencakup setiap jalur instalasi yang didukung dan penanganan masalah umum saat instalasi gagal, termasuk **Linux ARM64** dan platform lainnya.

Jika Anda hanya menginginkan versi singkat, lihat [README utama](../README.md#install) atau [README Bahasa Indonesia](../README.id.md#instalasi).

---

## 1. Skrip Instalasi Web (macOS & Linux)

Pada macOS dan Linux, installer situs web adalah jalur instalasi dan pembaruan tersingkat:

```bash
curl -fsSL https://formmy.app/ghosty/install.sh | sh
```

Skrip ini akan mengunduh biner rilis `ghosty` dan `ghosty-tui` yang cocok, memverifikasinya terhadap `ghosty-artifacts-sha256.txt`, dan menginstalnya ke `~/.local/bin` secara bawaan. Nama aset `ghosty-tui-*` hanya dipertahankan untuk kompatibilitas updater lama dan bukan perintah ketiga.

---

## 2. Platform yang Didukung

Rilis resmi Ghosty menyediakan biner pra-kemas untuk kombinasi platform dan arsitektur berikut:

| Platform     | Arsitektur | `npm install` | `cargo install` | Aset Rilis GitHub                                     |
| ------------ | ------------ | :---------: | :-------------: | ----------------------------------------------------- |
| Linux        | x64 (x86_64) |     ✅      |       ✅        | `ghosty-linux-x64`, `ghosty-tui-linux-x64`        |
| Linux        | arm64        |     ✅      |       ✅        | `ghosty-linux-arm64`, `ghosty-tui-linux-arm64`    |
| Android / Termux | arm64 (aarch64) | ⚠️ Pratinjau | ⚠️ Pratinjau | Arsip pratinjau `ghosty-android-arm64.tar.gz` |
| macOS        | x64          |     ✅      |       ✅        | `ghosty-macos-x64`, `ghosty-tui-macos-x64`        |
| macOS        | arm64 (M-series) | ✅      |       ✅        | `ghosty-macos-arm64`, `ghosty-tui-macos-arm64`    |
| Windows      | x64          |     ✅      |       ✅        | `ghosty-windows-x64.exe`, `ghosty-tui-windows-x64.exe` |
| Windows      | arm64        |     ✅      |       ✅        | `ghosty-windows-arm64.exe`, `ghosty-tui-windows-arm64.exe` |

---

## 3. Instalasi via npm

npm adalah pengelola paket yang paling umum digunakan:

```bash
npm install -g ghosty
```

Bagi pengguna Linux/macOS, pastikan direktori biner global npm berada di dalam `$PATH` Anda.

---

## 4. Instalasi via Cargo (Kompilasi dari Sumber Kode)

Jika Anda ingin mengompilasi biner langsung dari sumber kode menggunakan Rust:

```bash
cargo install ghosty-cli --locked
```

Persyaratan sistem:
- Rust toolchain (versi stable terbaru)
- Dependensi `libdbus-1-dev` atau `pkg-config` pada Linux untuk integrasi keyring OS.

---

## 5. Android / Termux

Termux berjalan di atas Bionic libc Android dan menggunakan `$PREFIX` sebagai awalan Unix-nya.

1. Pasang paket dasar:
   ```bash
   pkg update && pkg install rust clang make pkg-config libsqlite
   ```
2. Jalankan instalasi via Cargo:
   ```bash
   cargo install ghosty-cli --locked
   ```

---

## 6. Migrasi dari `deepseek-tui`

Jika Anda sebelumnya menggunakan `deepseek-tui`, seluruh sesi dan berkas konfigurasi Anda dapat ditransisikan dengan mudah:
- Jalur konfigurasi otomatis dimigrasikan dari `~/.config/deepseek-tui` ke `~/.config/ghosty`.
- Detail selengkapnya tersedia di [docs/REBRAND.md](REBRAND.md).
