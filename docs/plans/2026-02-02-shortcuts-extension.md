# 快捷键功能扩展实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 JLab 图像标注工具添加 4 个新功能的快捷键支持：自动保存开关、左右侧栏开关、预设缩放级别。

**Architecture:** 扩展现有的 `ShortcutAction` 枚举，添加新动作枚举值，在 `handle_shortcut_action` 中添加处理分支，补充国际化翻译。

**Tech Stack:** Rust, egui, serde (YAML 配置)

---

## 概述

本计划在现有快捷键系统基础上添加 12 个新动作：
- `ToggleAutoSave` - 切换自动保存
- `ToggleLeftPanel` - 切换左侧栏
- `ToggleRightPanel` - 切换右侧栏
- `Zoom25/50/75/100/125/150/200/300/400` - 9 个预设缩放级别

**涉及文件：**
- `lab-gui/src/shortcuts.rs` - 定义新动作枚举
- `lab-gui/src/app.rs` - 添加动作处理逻辑
- `lab-gui/locales/zh-CN.json` - 中文翻译
- `lab-gui/locales/en-US.json` - 英文翻译

---

### Task 1: 添加新动作枚举定义

**Files:**
- Modify: `lab-gui/src/shortcuts.rs:82-127`

**Step 1: 在 ShortcutAction 枚举中添加新动作**

找到 `pub enum ShortcutAction` 定义，在 `Cancel` 变体后添加：

```rust
pub enum ShortcutAction {
    // ... 现有所有动作保持不变 ...
    Cancel,

    // View toggles (新增)
    ToggleAutoSave,
    ToggleLeftPanel,
    ToggleRightPanel,

    // Preset zoom levels (新增)
    Zoom25,
    Zoom50,
    Zoom75,
    Zoom100,
    Zoom125,
    Zoom150,
    Zoom200,
    Zoom300,
    Zoom400,
}
```

**Step 2: 更新 all_actions() 方法**

找到 `pub fn all_actions()` 方法，在返回的 vec 中添加新动作：

```rust
pub fn all_actions() -> Vec<Self> {
    vec![
        // ... 现有动作保持不变 ...
        Self::Cancel,
        // 新增动作
        Self::ToggleAutoSave,
        Self::ToggleLeftPanel,
        Self::ToggleRightPanel,
        Self::Zoom25,
        Self::Zoom50,
        Self::Zoom75,
        Self::Zoom100,
        Self::Zoom125,
        Self::Zoom150,
        Self::Zoom200,
        Self::Zoom300,
        Self::Zoom400,
    ]
}
```

**Step 3: 为新动作添加 default_key() 实现**

找到 `pub fn default_key(&self)` 方法，在 `Cancel => Some(Escape)` 后添加：

```rust
Self::Cancel => Some(Escape),
// 新增动作的默认键
Self::ToggleAutoSave => None,
Self::ToggleLeftPanel => None,
Self::ToggleRightPanel => None,
Self::Zoom100 => Some(Z),  // Z 键快速缩放到 100%
Self::Zoom25 | Self::Zoom50 | Self::Zoom75 | Self::Zoom125
    | Self::Zoom150 | Self::Zoom200 | Self::Zoom300 | Self::Zoom400 => None,
```

**Step 4: 为新动作添加 default_modifiers() 实现**

找到 `pub fn default_modifiers(&self)` 方法，在最后的 `_ =>` 分支之前添加：

```rust
// 新增动作默认无修饰键
Self::ToggleAutoSave | Self::ToggleLeftPanel | Self::ToggleRightPanel
    | Self::Zoom25 | Self::Zoom50 | Self::Zoom75 | Self::Zoom100
    | Self::Zoom125 | Self::Zoom150 | Self::Zoom200 | Self::Zoom300
    | Self::Zoom400 => KeyModifiers {
        ctrl: false,
        shift: false,
        alt: false,
    },
```

**Step 5: 为新动作添加 scope() 实现**

找到 `pub fn scope(&self)` 方法，在 `Self::Cancel => ShortcutScope::Global,` 后添加：

```rust
Self::Cancel => ShortcutScope::Global,
// 新增动作均为全局作用域
Self::ToggleAutoSave | Self::ToggleLeftPanel | Self::ToggleRightPanel
    | Self::Zoom25 | Self::Zoom50 | Self::Zoom75 | Self::Zoom100
    | Self::Zoom125 | Self::Zoom150 | Self::Zoom200 | Self::Zoom300
    | Self::Zoom400 => ShortcutScope::Global,
```

**Step 6: 为新动作添加 category() 实现**

找到 `pub fn category(&self)` 方法，在最后的 `Self::Cancel => ShortcutCategory::Other,` 后添加：

```rust
Self::Cancel => ShortcutCategory::Other,
// 新增动作归入视图分类
Self::ToggleAutoSave | Self::ToggleLeftPanel | Self::ToggleRightPanel
    | Self::Zoom25 | Self::Zoom50 | Self::Zoom75 | Self::Zoom100
    | Self::Zoom125 | Self::Zoom150 | Self::Zoom200 | Self::Zoom300
    | Self::Zoom400 => ShortcutCategory::View,
```

**Step 7: 为新动作添加 description_key() 实现**

找到 `pub fn description_key(&self)` 方法，在 `Self::Cancel => "shortcut_actions.cancel",` 后添加：

```rust
Self::Cancel => "shortcut_actions.cancel",
Self::ToggleAutoSave => "shortcut_actions.toggle_auto_save",
Self::ToggleLeftPanel => "shortcut_actions.toggle_left_panel",
Self::ToggleRightPanel => "shortcut_actions.toggle_right_panel",
Self::Zoom25 => "shortcut_actions.zoom_25",
Self::Zoom50 => "shortcut_actions.zoom_50",
Self::Zoom75 => "shortcut_actions.zoom_75",
Self::Zoom100 => "shortcut_actions.zoom_100",
Self::Zoom125 => "shortcut_actions.zoom_125",
Self::Zoom150 => "shortcut_actions.zoom_150",
Self::Zoom200 => "shortcut_actions.zoom_200",
Self::Zoom300 => "shortcut_actions.zoom_300",
Self::Zoom400 => "shortcut_actions.zoom_400",
```

**Step 8: 为新动作添加 as_str() 实现**

找到 `pub fn as_str(&self)` 方法，在 `Self::Cancel => "Cancel",` 后添加：

```rust
Self::Cancel => "Cancel",
Self::ToggleAutoSave => "ToggleAutoSave",
Self::ToggleLeftPanel => "ToggleLeftPanel",
Self::ToggleRightPanel => "ToggleRightPanel",
Self::Zoom25 => "Zoom25",
Self::Zoom50 => "Zoom50",
Self::Zoom75 => "Zoom75",
Self::Zoom100 => "Zoom100",
Self::Zoom125 => "Zoom125",
Self::Zoom150 => "Zoom150",
Self::Zoom200 => "Zoom200",
Self::Zoom300 => "Zoom300",
Self::Zoom400 => "Zoom400",
```

**Step 9: 更新 build_bindings() 方法中的动作解析**

找到 `fn build_bindings()` 中的动作 match 语句，在 `Self::Cancel => ShortcutAction::Cancel,` 后添加：

```rust
Self::Cancel => ShortcutAction::Cancel,
Self::ToggleAutoSave => ShortcutAction::ToggleAutoSave,
Self::ToggleLeftPanel => ShortcutAction::ToggleLeftPanel,
Self::ToggleRightPanel => ShortcutAction::ToggleRightPanel,
Self::Zoom25 => ShortcutAction::Zoom25,
Self::Zoom50 => ShortcutAction::Zoom50,
Self::Zoom75 => ShortcutAction::Zoom75,
Self::Zoom100 => ShortcutAction::Zoom100,
Self::Zoom125 => ShortcutAction::Zoom125,
Self::Zoom150 => ShortcutAction::Zoom150,
Self::Zoom200 => ShortcutAction::Zoom200,
Self::Zoom300 => ShortcutAction::Zoom300,
Self::Zoom400 => ShortcutAction::Zoom400,
```

**Step 10: 更新 detect_conflicts() 方法中的动作解析**

找到 `fn detect_conflicts()` 中的动作 match 语句，在 `Self::Cancel => ShortcutAction::Cancel,` 后添加与 Step 9 相同的分支。

**Step 11: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功，可能有警告（新动作还未使用）

**Step 12: 提交**

```bash
git add lab-gui/src/shortcuts.rs
git commit -m "feat: add new shortcut action enums for toggle panels and zoom levels"
```

---

### Task 2: 添加快捷键处理逻辑

**Files:**
- Modify: `lab-gui/src/app.rs:631-821`

**Step 1: 在 handle_shortcut_action() 中添加新动作处理**

找到 `fn handle_shortcut_action()` 方法，在最后的 `Self::Cancel => { ... }` 分支后添加：

```rust
Self::Cancel => {
    // ... 现有代码保持不变 ...
}

// View toggles (新增)
ShortcutAction::ToggleAutoSave => {
    if let Some(project) = &mut self.state.current_project {
        project.meta.shape.auto_save = !project.meta.shape.auto_save;
    }
}
ShortcutAction::ToggleLeftPanel => {
    self.state.show_left_panel = !self.state.show_left_panel;
}
ShortcutAction::ToggleRightPanel => {
    self.state.show_right_panel = !self.state.show_right_panel;
}

// Preset zoom levels (新增)
ShortcutAction::Zoom25 => self.set_zoom(25.0),
ShortcutAction::Zoom50 => self.set_zoom(50.0),
ShortcutAction::Zoom75 => self.set_zoom(75.0),
ShortcutAction::Zoom100 => self.set_zoom(100.0),
ShortcutAction::Zoom125 => self.set_zoom(125.0),
ShortcutAction::Zoom150 => self.set_zoom(150.0),
ShortcutAction::Zoom200 => self.set_zoom(200.0),
ShortcutAction::Zoom300 => self.set_zoom(300.0),
ShortcutAction::Zoom400 => self.set_zoom(400.0),
```

**Step 2: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 3: 运行程序手动测试**

Run: `cargo run -p lab-gui`
Expected: 程序正常启动，无运行时错误

**Step 4: 提交**

```bash
git add lab-gui/src/app.rs
git commit -m "feat: add handling for new shortcut actions (toggle panels, zoom levels)"
```

---

### Task 3: 添加中文翻译

**Files:**
- Modify: `lab-gui/locales/zh-CN.json`

**Step 1: 在 shortcut_actions 节点添加新翻译**

找到 `"shortcut_actions": {` 节点，在 `"convert_to_rectangle": "变成矩形",` 后添加：

```json
"convert_to_rectangle": "变成矩形",
"toggle_auto_save": "切换自动保存",
"toggle_left_panel": "切换左侧栏",
"toggle_right_panel": "切换右侧栏",
"zoom_25": "缩放至 25%",
"zoom_50": "缩放至 50%",
"zoom_75": "缩放至 75%",
"zoom_100": "缩放至 100%",
"zoom_125": "缩放至 125%",
"zoom_150": "缩放至 150%",
"zoom_200": "缩放至 200%",
"zoom_300": "缩放至 300%",
"zoom_400": "缩放至 400%"
```

**Step 2: 验证 JSON 格式**

Run: `cat lab-gui/locales/zh-CN.json | jq .`
Expected: JSON 格式有效，无语法错误

**Step 3: 提交**

```bash
git add lab-gui/locales/zh-CN.json
git commit -m "i18n: add Chinese translations for new shortcut actions"
```

---

### Task 4: 添加英文翻译

**Files:**
- Modify: `lab-gui/locales/en-US.json`

**Step 1: 在 shortcut_actions 节点添加新翻译**

找到 `"shortcut_actions": {` 节点，在 `"convert_to_rectangle": "Convert to Rectangle",` 后添加：

```json
"convert_to_rectangle": "Convert to Rectangle",
"toggle_auto_save": "Toggle Auto Save",
"toggle_left_panel": "Toggle Left Panel",
"toggle_right_panel": "Toggle Right Panel",
"zoom_25": "Zoom to 25%",
"zoom_50": "Zoom to 50%",
"zoom_75": "Zoom to 75%",
"zoom_100": "Zoom to 100%",
"zoom_125": "Zoom to 125%",
"zoom_150": "Zoom to 150%",
"zoom_200": "Zoom to 200%",
"zoom_300": "Zoom to 300%",
"zoom_400": "Zoom to 400%"
```

**Step 2: 验证 JSON 格式**

Run: `cat lab-gui/locales/en-US.json | jq .`
Expected: JSON 格式有效，无语法错误

**Step 3: 提交**

```bash
git add lab-gui/locales/en-US.json
git commit -m "i18n: add English translations for new shortcut actions"
```

---

### Task 5: 手动功能测试

**Files:**
- Test: 运行中的应用程序

**Step 1: 构建并运行程序**

Run: `cargo run -p lab-gui`
Expected: 程序正常启动

**Step 2: 打开快捷键设置窗口**

操作: 菜单 -> 帮助 -> 快捷键设置
Expected: 快捷键设置窗口打开，显示所有动作包括新增的 12 个

**Step 3: 验证新动作在列表中显示**

在快捷键设置窗口中查找：
- "切换自动保存"
- "切换左侧栏"
- "切换右侧栏"
- "缩放至 25%" 到 "缩放至 400%"

Expected: 所有新动作都显示在列表中，分类为"视图"

**Step 4: 测试 Zoom100 默认快捷键**

操作: 按 `Z` 键
Expected: 画布缩放变为 100%

**Step 5: 为侧栏开关设置快捷键**

在快捷键设置中：
1. 找到 "切换左侧栏"，点击"编辑"
2. 按下 `Ctrl+1`
3. 点击"应用"
4. 对"切换右侧栏"重复，使用 `Ctrl+2`

操作: 按 `Ctrl+1` 和 `Ctrl+2`
Expected: 左右侧栏正确显示/隐藏

**Step 6: 测试自动保存开关**

为"切换自动保存"设置快捷键（如 `Ctrl+A`），测试切换功能
Expected: 菜单中自动保存状态正确切换

**Step 7: 测试预设缩放级别**

为各个缩放级别设置快捷键并测试
Expected: 按下快捷键后画布缩放到正确级别

**Step 8: 验证配置持久化**

关闭程序后重新打开，检查快捷键设置是否保存
Expected: 自定义的快捷键配置保留

**Step 9: 检查冲突检测**

尝试为两个不同动作设置相同快捷键
Expected: 显示冲突警告

---

### Task 6: 文档更新

**Files:**
- Modify: `README.md`

**Step 1: 更新 README.md 快捷键表**

在 `## 快捷键` 表格中添加新行：

```markdown
| 操作 | 快捷键 |
|------|--------|
| ... (现有快捷键) ... |
| 缩放到 100% | Z |
| 切换左侧栏 | (可自定义) |
| 切换右侧栏 | (可自定义) |
| 切换自动保存 | (可自定义) |
```

**Step 2: 提交**

```bash
git add README.md
git commit -m "docs: update shortcuts table with new actions"
```

---

## 验收标准

完成所有任务后：
1. ✅ 代码编译无错误
2. ✅ 所有 12 个新动作出现在快捷键设置界面
3. ✅ Zoom100 默认快捷键 `Z` 正常工作
4. ✅ 侧栏开关、自动保存开关、预设缩放都能正确响应快捷键
5. ✅ 中英文翻译正确显示
6. ✅ 配置文件正确保存和加载
7. ✅ 冲突检测正常工作

---

## 预计时间

- Task 1: 15 分钟
- Task 2: 10 分钟
- Task 3: 5 分钟
- Task 4: 5 分钟
- Task 5: 15 分钟
- Task 6: 5 分钟

**总计:** 约 55 分钟
