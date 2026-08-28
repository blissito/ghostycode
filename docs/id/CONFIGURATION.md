# Konfigurasi Ghosty

Ghosty membaca konfigurasi dari berkas TOML ditambah variabel lingkungan (environment variables). Saat startup proses, Ghosty juga dapat membaca kredensial penyedia dari berkas `.env` lokal di ruang kerja. Gunakan contoh templat `.env.example` yang tersedia; salin ke `.env`, lalu tambahkan hanya nilai kredensial.

---

## Konstitusi, Instruksi Proyek, dan Wewenang Repositori

Ghosty memiliki beberapa lapisan instruksi yang sengaja dipisahkan agar konstitusi pribadi, kebijakan repositori, instruksi proyek, dan kontrol keamanan runtime tidak saling bertabrakan:

1. **Konstitusi Global Bawaan (Bundled Constitution)**:
   - Hukum dasar yang terkompilasi di dalam biner. Menjadi batas dasar bawaan untuk setiap sesi.

2. **Konstitusi Global Pengguna (User-Global Constitution)**:
   - Dikelola melalui perintah `/constitution` atau `/setup`. Ditinggalkan dalam format terstruktur pada `$GHOSTY_HOME/constitution.json` (bawaan `~/.ghosty/constitution.json`).

3. **Konstitusi Lokal Repositori (Repo-Local Constitution)**:
   - Kebijakan proyek opsional pada `.ghosty/constitution.json`.

4. **Instruksi Proyek (`AGENTS.md`)**:
   - Berkas instruksi proyek lintas-agen. Berkas ini adalah dokumen kanonik untuk "bagaimana agen bekerja di repositori ini". Jalankan `/init` untuk membuatnya secara otomatis. `CLAUDE.md` dibaca sebagai fallback kompatibilitas.

5. **Memori dan Serah Terima (Memory & Handoffs)**:
   - Riwayat dan konteks yang dipanggil kembali. Berfungsi membantu, namun memiliki tingkat wewenang lebih rendah dari konstitusi dan instruksi proyek.

---

## Lokasi Berkas Konfigurasi

- Berkas Konfigurasi Utama: `~/.ghosty/config.toml`
- Berkas Konstitusi Pengguna: `~/.ghosty/constitution.json`
- Berkas Konfigurasi MCP: `~/.ghosty/mcp.json`
- Berkas Sesi: `~/.ghosty/sessions/`
