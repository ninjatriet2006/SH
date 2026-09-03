# Quy tắc Tư duy Kiểm thử (Automated Testing Awareness)

- **TESTABILITY FIRST:** Khi triển khai các logic cốt lõi, hãy viết các hàm sao cho chúng được tách bạch (decoupled) và dễ dàng kiểm thử tự động.
- **MOCKING & FIXTURES:** Hãy nhớ rằng các tính năng gọi API ra bên ngoài (như Cloud, OpenAI...) sẽ cần phải mock/stub trong tương lai. Thiết kế các class với ranh giới rõ ràng hoặc dependency injection để tạo thuận lợi cho việc này.
