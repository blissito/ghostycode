# Agent Fleet (Armada Agen)

Agent Fleet adalah control plane yang mengutamakan lokal (*local-first*) untuk eksekusi banyak pekerja (*multi-worker*) yang tahan lama. Fleet **bukanlah** mesin eksekusi terpisah: seorang pekerja fleet (*fleet worker*) adalah eksekusi `ghosty exec` tanpa antarmuka yang diluncurkan dan dilacak oleh fleet secara permanen.

Gunakan Fleet daripada pembagian tugas agen yang berumur pendek ketika pekerjaan membutuhkan percobaan ulang (*retry*), ketahanan terhadap mode tidur/restart komputer, eksekusi jarak jauh, bukti tanda terima (*receipts*), atau jejak audit ber-ledger.

---

## Perintah Dasar CLI Fleet

```sh
ghosty fleet init
ghosty fleet run tasks.json --max-workers 4
ghosty fleet status
ghosty fleet inspect <worker-id>
ghosty fleet logs <worker-id>
ghosty fleet artifacts <worker-id>
ghosty fleet interrupt <worker-id>
ghosty fleet restart <worker-id>
ghosty fleet resume <run-id>
ghosty fleet stop --all
```

`ghosty fleet resume <run-id>` adalah perintah pemulihan setelah sistem terhenti: perintah ini memutar ulang ledger, merekonsiliasi tugas yang terhenti (Mencoba lagi sesuai anggaran tugas, atau melaporkannya jika gagal), lalu menampilkan status setelah pemulihan. Perintah ini aman dijalankan setelah laptop terbangun dari mode tidur atau setelah restart runtime.

---

## Lokasi Penyimpanan Status

Status Fleet disimpan di dalam ruang kerja di bawah `.ghosty/fleet.jsonl`. Log pekerja dan log adapter disimpan di bawah `.ghosty/fleet/` dan `.ghosty/fleet-host/`.

### Perbedaan Status Interaktif dan Persisten

- Di dalam TUI: Perintah `/fleet status` (atau `/subagents`) menampilkan sub-agen yang terhubung ke sesi interaktif saat ini.
- Di dalam Shell: Perintah `ghosty fleet status` membaca riwayat eksekusi Fleet yang tersimpan di ledger `.ghosty/fleet.jsonl`.
