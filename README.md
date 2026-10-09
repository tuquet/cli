<div align="center">
  <img src="https://tuquet.com/icons/cli.svg" width="76" height="76" alt="CLI Logo" />
  <h1>Specter Master CLI (`specter`)</h1>
  <p><strong>Interactive Scoped Shell, Service Multiplexer & High-Performance Automation Terminal in Rust</strong></p>

  <p>
    <a href="https://specter.tuquet.com/commands/"><img src="https://img.shields.io/badge/Docs-VitePress%20Hub-blue.svg" alt="Documentation Hub" /></a>
    <a href="https://github.com/tuquet/scoop-bucket"><img src="https://img.shields.io/badge/Scoop-specter-brightgreen.svg" alt="Scoop" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>

  <p>
    <strong><a href="https://specter.tuquet.com/commands/">📖 Đọc toàn bộ tài liệu 68 lệnh CLI tại Documentation Hub &rarr;</a></strong>
  </p>
</div>

---

## 📌 Tổng Quan (Overview)

**Specter Master CLI** (`specter`) là điểm vào thống nhất (Single Entrypoint) điều phối toàn bộ hệ sinh thái Specter. Viết bằng Rust thuần túy với thời gian khởi động sub-millisecond, Specter CLI cung cấp cả giao diện dòng lệnh truyền thống và một **Interactive Scoped REPL** giúp chuyển đổi linh hoạt giữa các subsystem mà không làm phân mảnh công cụ.

* **68 Lệnh Chuẩn hóa**: Bao quát toàn bộ 5 trụ cột microservice (`browser`, `runner`, `automa`, `bridge`, `faker`, `cloud`, `system`).
* **Lưu trữ SSOT**: Tự động giải quyết cấu hình và trạng thái về gốc chuẩn duy nhất `~/.specter/`.

## ⚡ Sử Dụng Nhanh (Quickstart)

```bash
# Khởi chạy Interactive Scoped Shell
specter

# Kiểm tra sức khỏe toàn diện của hệ thống
specter doctor
```

## 📚 Tài Liệu Kỹ Thuật Tập Trung (SSOT)

Toàn bộ danh mục 68 lệnh CLI, bảng tham số cờ, hướng dẫn tương tác REPL và phím tắt được bảo trì duy nhất tại Documentation Hub:

👉 **[https://specter.tuquet.com/commands/](https://specter.tuquet.com/commands/)**
