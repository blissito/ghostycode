# Integrasi MCP (Server Alat Eksternal)

Ghosty dapat memuat alat tambahan melalui **MCP (Model Context Protocol)**. Server MCP dapat berupa proses stdio lokal yang dijalankan oleh TUI, atau server jarak jauh berbasis URL yang menggunakan Streamable HTTP dengan fallback SSE.

---

## Konfigurasi Server MCP

Konfigurasi server MCP disimpan di dalam `mcp.json` (baik di `$GHOSTY_HOME/mcp.json` untuk global atau `.ghosty/mcp.json` untuk proyek).

Contoh konfigurasi `mcp.json`:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allowed/dir"]
    },
    "git": {
      "command": "uvx",
      "args": ["mcp-server-git", "--repository", "."]
    }
  }
}
```

---

## Perintah CLI & TUI untuk MCP

- `/mcp` — Kelola server dan alat MCP dari dalam TUI.
- `ghosty mcp init` — Inisialisasi templat konfigurasi `mcp.json`.
- `ghosty doctor` — Periksa kesehatan dan kesiapan server MCP yang terkonfigurasi.
