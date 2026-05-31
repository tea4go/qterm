# QTerm 窗口位置管理改造计划

基于 mdedit 的窗口位置管理方案（`C:\MyWork\AiCode\qterm\程序位置.md`），改造 QTerm 的窗口位置管理。

## 当前状态 vs 目标状态

| 特性 | 当前 QTerm | 目标（参考 mdedit） |
|------|-----------|-------------------|
| DPI 感知初始化 | 无 | `SetProcessDpiAwarenessContext()` Per-Monitor DPI V2 |
| 命令行参数 | 无 | `--reset`、`--setpos x,y` |
| 窗口定位时机 | 创建时立即定位 | 延迟到第 2 帧 |
| 坐标单位 | 混用，无 DPI 转换 | 位置用物理像素，尺寸用 egui points |
| 位置保存 | outer_rect 原始值（points） | outer_rect × ppp → 物理像素 |
| 位置恢复 | 直接用保存值 | 物理像素 ÷ ppp → points |
| 最大化状态追踪 | 仅标题栏点击时更新 | 每帧从 viewport 读取 |
| 最大化状态保存 | on_exit 中未保存 | 保存到 config.ini |
| 启动日志 | 无 | 写入 startup.log |
| 显示器枚举 | 仅有 is_position_visible | log_monitors() 枚举所有显示器 |

---

## 实施步骤

### 步骤 1：main.rs — 添加 DPI 感知初始化

**目标**：在 main() 最开始调用 Win32 API 设置 Per-Monitor DPI V2 感知。

**改动**：
- 添加 `#[cfg(windows)]` 块，调用 `SetProcessDpiAwarenessContext`
- 在 `fn main()` 第一行调用此函数

**代码位置**：`src/main.rs` 第 51 行 `fn main()` 之前和开头

**验证**：`cargo check` 编译通过

---

### 步骤 2：main.rs — 添加命令行参数解析

**目标**：支持 `--reset` 和 `--setpos x,y` 参数。

**改动**：
- 在 main() 中解析 `std::env::args()`
- `--reset`：设置 reset 标志，不恢复窗口位置
- `--setpos x,y`：解析为 (f32, f32)，作为强制窗口位置
- 优先级：`--reset` > `--setpos` > 配置文件 > 系统默认

**代码位置**：`src/main.rs` main() 函数开头，`let cfg = config::AppConfig::load();` 之后

**验证**：`cargo check` 编译通过

---

### 步骤 3：main.rs — 改为延迟定位（不再创建时定位）

**目标**：移除创建时的 `with_position()` 调用，改为通过 `target_pos` 传递给 QTermApp，在第 2 帧定位。

**改动**：
- 移除 main.rs 第 66-69 行的 `with_position()` 逻辑
- main.rs 中计算 `target_pos: Option<(f32, f32)>` 传递给 QTermApp
- target_pos 的计算逻辑：
  - reset=true → None
  - setpos=Some(x,y) → Some((x,y))
  - cfg 有保存的坐标 → 验证 `is_position_visible` → Some/None
  - 其他 → None
- 最大化恢复保留在创建时（`with_maximized`），因为最大化不受 DPI 影响

**代码位置**：
- `src/main.rs` 第 52-86 行
- `src/app.rs` QTermApp::new() 签名变更

**验证**：`cargo check` 编译通过

---

### 步骤 4：app.rs — 添加 frame_count 和 target_physical_pos 字段

**目标**：QTermApp 添加帧计数和目标物理位置字段。

**改动**：
- 在 QTermApp 结构体中添加：
  - `frame_count: u32` — 帧计数器
  - `target_physical_pos: Option<(f32, f32)>` — 目标物理像素位置
- new() 中初始化这两个字段
- new() 接受 `target_pos: Option<(f32, f32)>` 参数（已经是物理像素或 setpos 的值）

**代码位置**：
- `src/app.rs` 第 17-37 行（结构体定义）
- `src/app.rs` 第 60-103 行（new() 方法）

**验证**：`cargo check` 编译通过

---

### 步骤 5：app.rs — 实现第 2 帧延迟定位

**目标**：在 update() 的开头，当 frame_count == 2 时执行窗口定位。

**改动**：
- update() 开头递增 frame_count
- 当 frame_count == 2 且 target_physical_pos 有值时：
  - 获取 `ppp = ctx.pixels_per_point()`
  - 将物理像素转换为 egui points：`pos = egui::pos2(px / ppp, py / ppp)`
  - 发送 `ViewportCommand::OuterPosition(pos)`
  - take() 掉 target_physical_pos

**代码位置**：`src/app.rs` update() 方法开头（第 328 行附近）

**验证**：`cargo check` 编译通过

---

### 步骤 6：app.rs — 每帧正确追踪窗口状态（含 DPI 转换）

**目标**：每帧从 viewport 读取窗口状态，位置转物理像素保存。

**改动**：
- 替换当前的位置/尺寸追踪代码（第 344-351 行）
- 新逻辑：
  ```rust
  let ppp = ctx.pixels_per_point();
  ctx.input(|i| {
      if let Some(rect) = i.viewport().inner_rect {
          self.last_window_size = Some((rect.width(), rect.height()));  // points，直接保存
      }
      if let Some(rect) = i.viewport().outer_rect {
          self.last_window_pos = Some((rect.min.x * ppp, rect.min.y * ppp));  // 转 physical pixels
      }
      self.last_maximized = i.viewport().maximized.unwrap_or(false);  // 每帧更新
  });
  ```

**代码位置**：`src/app.rs` update() 方法中第 344-351 行

**验证**：`cargo check` 编译通过

---

### 步骤 7：app.rs — on_exit 保存最大化状态

**目标**：退出时保存窗口位置、尺寸、最大化状态。

**改动**：
- on_exit() 中添加 `self.config.maximized = self.last_maximized;`
- 当前 on_exit 已保存 window_x/y/width/height/theme，只需补充 maximized

**代码位置**：`src/app.rs` on_exit() 方法（第 624-635 行）

**验证**：`cargo check` 编译通过

---

### 步骤 8：main.rs — 添加 log_monitors() 显示器枚举

**目标**：启动时枚举所有显示器信息并写入日志。

**改动**：
- 添加 `log_monitors()` 函数（Windows 使用 `EnumDisplayMonitors` Win32 API）
- 非 Windows 平台提供空实现或简单日志
- main() 中调用 log_monitors()
- 日志写入 `%APPDATA%/qterm/startup.log`

**代码位置**：`src/main.rs` 新增函数，在 main() 中调用

**验证**：`cargo check` 编译通过，运行时检查 startup.log 内容

---

### 步骤 9：main.rs — 添加启动位置决策日志

**目标**：将所有窗口位置相关的决策过程写入日志，便于调试。

**改动**：
- 在计算 target_pos 的过程中记录每一步决策
- 日志格式参考 mdedit：
  ```
  [timestamp] ========== QTerm 启动 ==========
  [timestamp] 配置: x=Some(...), y=Some(...), ...
  [timestamp] 恢复窗口位置: 物理(x, y), 大小: (w, h)
  [timestamp] is_position_visible => true/false
  ```
- 如果位置不可见，记录回退到系统默认

**代码位置**：`src/main.rs` main() 函数中的 target_pos 计算逻辑

**验证**：运行程序后检查 startup.log 内容

---

### 步骤 10：端到端测试

**目标**：验证所有功能正常工作。

**测试用例**：
1. **正常启动**：关闭窗口后重新启动，位置和大小恢复正确
2. **最大化恢复**：最大化后关闭，重新启动仍为最大化状态
3. **多显示器**：窗口在副屏 → 断开副屏 → 重启 → 窗口回到主屏默认位置
4. **`--reset`**：`qterm --reset` 启动，窗口使用系统默认位置
5. **`--setpos`**：`qterm --setpos 3840,222` 启动，窗口定位到指定坐标
6. **DPI 缩放**：在 200% 缩放的副屏使用后关闭，重启时位置正确
7. **startup.log**：检查日志内容包含显示器信息和位置决策

**验证方式**：
- `cargo build` 编译成功
- 手动运行并验证上述场景

---

## 文件改动总结

| 文件 | 改动内容 | 预计行数变化 |
|------|---------|-------------|
| `src/main.rs` | DPI 初始化、命令行解析、延迟定位、显示器枚举、启动日志 | +150 行 |
| `src/app.rs` | frame_count/target_physical_pos 字段、第 2 帧定位、DPI 转换追踪、maximized 保存 | +20 行，修改 ~15 行 |
| `src/config.rs` | 无变更（已支持 maximized 字段） | 0 行 |

## 不改动的部分

- `config.rs`：已经支持 maximized 字段的读写，无需修改
- `theme/`、`terminal/`、`ssh/`、`pty/`、`sftp/`、`connection/`、`ui/`、`tab/`：不涉及窗口位置管理

## 关键注意事项

1. **坐标单位**：window_x/y 使用物理像素保存，window_width/height 使用 egui points 保存。这与 mdedit 一致。
2. **第 2 帧定位**：这是关键改动，避免了 DPI 因子不准确的时机问题。
3. **向后兼容**：旧的 config.ini 中保存的是 points 值而非物理像素。升级后首次启动，旧坐标可能略有偏移（因为 ÷ ppp 会产生不同的值）。这是可接受的行为——最坏情况下窗口位置偏移几个像素。
4. **Windows 专属代码**：DPI 初始化和显示器枚举使用 `#[cfg(windows)]` 条件编译，非 Windows 平台提供简化实现。

## 实施顺序说明

步骤 1-7 是核心功能改造，必须按顺序执行（有依赖关系）。步骤 8-9 是辅助功能（日志），可以最后添加。步骤 10 是验证。

**断点续执行指南**：
- 如果中断在步骤 1-3 之间：main.rs 可能编译不通过（因为 app.rs 签名未更新），需要继续修改 app.rs
- 如果中断在步骤 4-7 之间：可能 app.rs 编译不通过，继续按步骤完成即可
- 如果中断在步骤 8-9 之间：核心功能已完成，日志功能可选
