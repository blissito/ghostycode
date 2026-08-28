# Rebrand: DeepSeek TUI → Ghosty

Mulai dari versi **v0.8.41**, proyek ini dirilis dengan nama baru: `ghosty`.

Dokumen ini menjelaskan apa saja yang berubah, apa yang tetap sama, dan cara melakukan migrasi. Seluruh integrasi penyedia (provider) DeepSeek tidak mengalami perubahan — hanya nama merek CLI / TUI lokal yang diperbarui.

---

## Ringkasan Migrasi

```bash
# 1. Hapus instalasi paket atau biner lama.
npm uninstall -g deepseek-tui      # atau:
cargo uninstall deepseek-tui-cli 2>/dev/null || true
cargo uninstall deepseek-tui 2>/dev/null || true

# 2. Pasang dengan nama baru.
npm install -g ghosty            # atau:
cargo install ghosty-cli --locked

# 3. Jalankan perintah baru.
ghosty doctor
ghosty
```

Berkas dan direktori Anda yang ada seperti `~/.deepseek/config.toml`, `~/.deepseek/sessions/`, `~/.deepseek/skills/`, `~/.deepseek/tasks/`, dan `~/.deepseek/mcp.json` **tidak akan dihapus**. Instalasi baru Ghosty mengutamakan `~/.ghosty/`, sementara direktori lama `~/.deepseek/` tetap dibaca sebagai fallback selama masa transisi. Variabel lingkungan `DEEPSEEK_*` tetap berfungsi sebagaimana mestinya.

---

## Apa Saja yang Berubah Nama

| Komponen | Sebelum | Sesudah |
|---|---|---|
| Perintah Terpasang | `deepseek` / `deepseek-tui` | `ghosty` / `ghosty-tui` |
| Paket Wrapper npm | `deepseek-tui` | `ghosty` |
| Crate Crates.io | `deepseek-tui-cli` / `deepseek-tui` | `ghosty-cli` / `ghosty-tui` |
| Aset Rilis | `deepseek-<platform>` / `deepseek-tui-<platform>` | `ghosty-<platform>` / `ghosty-tui-<platform>`; `ghosty-tui-<platform>` hanya nama kompatibilitas |
| Manifest Checksum | `deepseek-artifacts-sha256.txt` | `ghosty-artifacts-sha256.txt` |

---

## Apa yang TIDAK Berubah

Semua hal yang berkaitan dengan API penyedia DeepSeek tetap berjalan persis seperti sebelumnya:
- **Variabel Lingkungan**: `DEEPSEEK_API_KEY`, `DEEPSEEK_BASE_URL`, `DEEPSEEK_MODEL`, `DEEPSEEK_PROVIDER`, `DEEPSEEK_PROFILE`, `DEEPSEEK_YOLO`, dll. tetap didukung sepenuhnya.
- **Konfigurasi Penyedia**: Pengaturan rute `[providers.deepseek]` pada `config.toml` tetap valid.
