# RustDesk 命令行登入功能使用指南

## 功能說明

現在 RustDesk 支援通過命令行登入到您的帳號，無需打開 GUI 介面。

## 編譯

需要使用 `cli` feature 編譯：

```bash
cargo build --release --features cli
```

## 使用方法

### 基本登入命令

```bash
rustdesk --login --username <您的用戶名> --login-password <您的密碼>
```

或者使用短選項：

```bash
rustdesk --login -u <您的用戶名> --login-password <您的密碼>
```

### 範例

```bash
# 登入示例
rustdesk --login --username john@example.com --login-password mypassword123

# 成功後會顯示
# Login successful! Welcome John Doe
```

## 功能詳情

1. **自動配置 API 伺服器**：使用配置文件中設定的 API 伺服器地址
2. **保存登入狀態**：登入成功後，access token 會保存到本地配置
3. **跨平台支援**：支援 Windows、Linux、macOS

## 登入後的操作

登入成功後，您的帳號資訊會保存在本地配置中，後續使用 RustDesk 時會自動使用已登入的帳號：

- 可以訪問您的通訊錄
- 可以訪問您的群組
- 可以同步設定

## 登出

如需登出，可以使用以下命令（需要在 GUI 模式下）：

```bash
rustdesk
# 然後在設定中選擇登出
```

或者清除本地配置：

```bash
# Linux/macOS
rm ~/.config/rustdesk/RustDesk.toml

# Windows
# 刪除 %APPDATA%\RustDesk\config\RustDesk.toml
```

## 錯誤處理

如果登入失敗，會顯示錯誤資訊：

```
Error: Login failed with status 401: Invalid credentials
```

常見錯誤：

1. **API 伺服器未配置**：請先在 GUI 設定中配置 API 伺服器
2. **用戶名或密碼錯誤**：請檢查輸入的憑證
3. **網路連接失敗**：請檢查網路連接和 API 伺服器地址

## 安全建议

1. **避免在命令行中直接输入密码**: 使用环境变量或配置文件来存储敏感信息
2. **使用环境变量**: 将密码存储在环境变量中,避免在 shell 历史记录中留下痕迹
3. **清理 Shell 历史**: 如果必须在命令行中使用 --login-password,请在执行后清理 shell 历史:
   ```bash
   history -d $(history 1 | awk '{print $1}')
   ```
4. **使用强密码**: 确保使用复杂且唯一的密码
5. **定期更换密码**: 定期更新你的登入凭证

## 技術實現

登入功能通過以下步驟實現：

1. 讀取配置的 API 伺服器地址
2. 構建登入請求（包含用戶名、密碼、設備 ID 和 UUID）
3. 發送 HTTP POST 請求到 `/api/login` 端點
4. 解析回應，提取 access token 和用戶資訊
5. 將 token 和用戶資訊保存到本地配置

## 相關文件

- `src/main.rs` - 命令行參數解析
- `src/cli.rs` - 登入邏輯實現
- `flutter/lib/models/user_model.dart` - Flutter 端登入模型（參考）
