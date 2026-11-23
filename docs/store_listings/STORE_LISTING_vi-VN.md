# Danh sách Microsoft Store - Tiếng Việt (vi-VN)
# Win2Pod Bridge - Phiên bản 0.1.21

## 0. Tính năng mới trong phiên bản này (What's new in this version - Dành cho Partner Center)
```text
[Bản cập nhật Win2Pod Bridge 0.1.21 - Hiệu ứng Genie mượt mà & Bán trong suốt thông minh]
• 🧞 Hiệu ứng thu nhỏ Genie chuẩn phong cách macOS (Muelle Genie Effect):
  - Tích hợp hiệu ứng thu nhỏ Genie S-Curve quét từng dòng quét lấy cảm hứng từ dự án mã nguồn mở muelle. Cửa sổ được hút mượt mà vào thanh tác vụ (Taskbar) khi thu nhỏ và khôi phục tức thì mà không hề bị khựng.
• 🪟 Độ bán trong suốt phần cứng Windows gốc (WS_EX_LAYERED) & Hover Peek thông minh:
  - Cửa sổ trình trực quan hóa nổi hỗ trợ tùy chỉnh độ trong suốt phần cứng (20% ~ 100%), không che khuất không gian làm việc.
  - Tính năng độc quyền Hover Peek: khi rê chuột lại gần, cửa sổ sẽ ngay lập tức khôi phục 100% độ mờ đục để thao tác chính xác và tiện lợi.
• 🧲 Hút dính và hợp nhất cụm cửa sổ đa màn hình:
  - Khắc phục lỗi lệch vị trí trên môi trường nhiều màn hình: cửa sổ chính và cửa sổ trực quan hóa sẽ tự động hút dính và đồng bộ mượt mà khi mở cùng lúc.
  - Vòng sáng nhịp thở và chỉ dẫn trực quan cho người dùng mới:
  - Bổ sung hiệu ứng hướng dẫn tinh tế giúp người dùng mới dễ dàng làm quen với tính năng ghim cửa sổ và chỉnh độ trong suốt ngay lần đầu khởi chạy.
```

## 1. Tên sản phẩm (Product Name)
```text
Win2Pod Bridge
```

---

## 2. Mô tả ngắn (Short Description - Tối đa 270 ký tự)
*(Hiển thị trong kết quả tìm kiếm Microsoft Store, trang danh mục và thẻ quảng bá)*

```text
Kết nối âm thanh Windows & Apple hoàn hảo! Truyền âm thanh PC tới HomePod ở chất lượng ALAC chuẩn lossless, hoặc nhận âm thanh AirPlay từ iPhone/iPad với dải động tối đa Unity Gain! Hỗ trợ handoff AirPods, trực quan 3D, điều khiển từ xa và bộ cân bằng EQ 10 băng tần.
```
*(Số lượng ký tự: ~258 ký tự, nằm gọn trong giới hạn 270)*

---

## 3. Mô tả đầy đủ (Full Description)
*(Sao chép và dán vào trường Mô tả trên Microsoft Partner Center)*

```text
Biến PC Windows của bạn thành trung tâm truyền phát và nhận âm thanh không dây độ trung thực cao AirPlay 2!

Win2Pod Bridge là cầu nối truyền phát âm thanh hiện đại, gọn nhẹ và hiệu năng vượt trội được thiết kế riêng cho Windows 10 & 11. Nhờ công nghệ WASAPI Loopback với độ trễ cực thấp, phần mềm thu lại âm thanh hệ thống hoặc từng ứng dụng riêng biệt và truyền phát dưới định dạng ALAC không nén (Apple Lossless Audio Codec, 44.1 kHz / 16-bit stereo) trực tiếp đến Apple HomePod, HomePod mini, Apple TV và bất kỳ loa tương thích AirPlay 2 nào.

Hệ điều hành Windows không hỗ trợ AirPlay 2 nguyên bản, trong khi loa Apple HomePod yêu cầu xác thực ghép đôi mã hóa HomeKit tạm thời. Win2Pod Bridge giải quyết trọn vẹn vấn đề này bằng phần mềm thuần Rust. Không cần can thiệp phần cứng, không cần chip MFi và không chạy các dịch vụ nền nặng nề!

============================================================
🌟 TÍNH NĂNG VÀ KHẢ NĂNG NỔI BẬT (KEY FEATURES)
============================================================

• 📲 BỘ NHẬN ÂM THANH AIRPLAY (AIRPLAY RECEIVER)
  - Loa máy tính không dây: Truyền phát nhạc và video từ iPhone, iPad hoặc Mac trực tiếp đến loa PC hoặc tai nghe.
  - Giải mã ALAC Lossless: Giải mã bit-perfect bảo toàn độ rõ nét chuẩn audiophile.
  - ✨ Dải động tối đa: Giải mã cục bộ ở mức 1.0 (0 dB toàn dải), bỏ qua suy giảm âm lượng iOS.
  - Điều khiển từ xa & Bìa album: Điều khiển bài hát trực tiếp trên máy tính cùng hiển thị ảnh bìa album trực tiếp.
  - Đồng bộ âm thanh và hình ảnh: Bù trễ linh hoạt cho trải nghiệm xem YouTube mượt mà.

• 🛡️ Auto Pre-Amp & Bộ cân bằng âm thanh 10 băng tần (Pro Equalizer):
  - Bộ cân bằng đồ họa 10 băng tần tích hợp kiểm soát khoảng đệm tự động. Khi tăng bất kỳ dải tần nào, hệ thống sẽ tự động giảm mức tiền khuếch đại tương ứng kết hợp bộ giới hạn đỉnh (peak limiter) trong suốt để triệt tiêu hiện tượng rè méo âm thanh.
• 🔄 Tự động phục hồi tuyến âm thanh (Self-Healing Audio Routing):
  - Liên tục giám sát thiết bị phát Windows. Khi thiết bị mặc định bị ngắt kết nối, phần mềm tự động chuyển sang thiết bị vật lý khả dụng khác.
• 🔔 Chú thích khay hệ thống hai dòng trực quan:
  - Dòng 1 hiển thị trạng thái kết nối HomePod và mức âm lượng; dòng 2 hiển thị chi tiết phần trăm pin từng bên của AirPods.
• 🎧 Chuyển đổi thông minh hai chiều AirPods ↔ HomePod (Smart Handoff):
  - Đeo tai nghe vào thì âm thanh tự chuyển sang AirPods; cất vào hộp sạc thì âm thanh tự động trở về loa HomePod!
• 🧲 Cửa sổ hút dính từ tính (Magnetic Window Snapping):
  - Bảng điều khiển chính và cửa sổ 3D tự động hút dính vào nhau và vào cạnh màn hình, di chuyển liền mạch cùng nhau.
• 🎨 Trình trực quan hóa 3D Paint Pulse tương tác:
  - Hiệu ứng nét vẽ sơn dầu trường phái ấn tượng Van Gogh kết xuất thời gian thực bằng GPU theo nhịp điệu bài hát.
• ⌨️ Phím tắt toàn cục tùy chỉnh:
  - Bật/tắt nhanh bảng điều khiển từ mọi trò chơi hoặc ứng dụng (Mặc định: Ctrl + Shift + P).
• 🌐 Hỗ trợ đa ngôn ngữ hoàn hảo:
  - Giao diện hỗ trợ 14 ngôn ngữ với khả năng tự động nhận diện ngôn ngữ Windows.
```

---

## 4. Từ khóa tìm kiếm (Search Keywords / Tags - Tối đa 7 từ khóa)
```text
AirPlay, HomePod, Âm thanh PC, AirPods, Equalizer, Bộ nhận AirPlay, Loa không dây
```
