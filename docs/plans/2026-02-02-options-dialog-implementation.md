# 选项对话框与帮助菜单重构实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 为 JLab 添加统一的选项对话框和关于对话框，重构帮助菜单符合 Windows 惯例。

**架构:** 新增两个对话框模块，修改现有菜单结构，复用快捷键编辑器。

**技术栈:** Rust, egui

---

## 概述

本计划将：
1. 创建选项对话框（常规 + 快捷键标签页）
2. 创建关于对话框
3. 重构文件菜单（添加"选项"）
4. 重构帮助菜单（帮助、检查更新、关于）
5. 添加完整的中英文翻译

---

### Task 1: 添加对话框状态到 AppState

**Files:**
- Modify: `lab-gui/src/state.rs:98-105`

**Step 1: 在 AppState 中添加对话框状态字段**

找到 `pub show_shortcut_settings: bool,` 后添加：

```rust
pub show_shortcut_settings: bool,

// Dialog states (新增)
pub show_options_dialog: bool,
pub show_about_dialog: bool,
```

**Step 2: 在 AppState::new() 中初始化新字段**

找到 `show_shortcut_settings: false,` 后添加：

```rust
show_shortcut_settings: false,
show_options_dialog: false,
show_about_dialog: false,
```

**Step 3: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 4: 提交**

```bash
git add lab-gui/src/state.rs
git commit -m "feat: add options and about dialog states to AppState"
```

---

### Task 2: 创建选项对话框模块

**Files:**
- Create: `lab-gui/src/app/options_dialog.rs`

**Step 1: 创建选项对话框文件**

创建 `lab-gui/src/app/options_dialog.rs`：

```rust
// Options dialog for JLab
use crate::i18n::Language;
use crate::shortcuts::ShortcutEditorState;
use egui::Context;

/// Options dialog tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    General,
    Shortcuts,
}

impl OptionsTab {
    pub fn name_key(&self) -> &'static str {
        match self {
            Self::General => "options.tab_general",
            Self::Shortcuts => "options.tab_shortcuts",
        }
    }
}

/// Options dialog state
pub struct OptionsDialogState {
    pub show: bool,
    pub active_tab: OptionsTab,
    pub selected_language: Language,
    pub auto_save_enabled: bool,
    pub shortcut_editor: Option<ShortcutEditorState>,
    pub pending_changes: bool,
}

impl OptionsDialogState {
    pub fn new(language: Language, auto_save: bool) -> Self {
        Self {
            show: false,
            active_tab: OptionsTab::General,
            selected_language: language,
            auto_save_enabled: auto_save,
            shortcut_editor: None,
            pending_changes: false,
        }
    }

    /// Show the options dialog
    pub fn show_dialog(&mut self, current_config: &crate::shortcuts::ShortcutConfig) {
        self.show = true;
        self.active_tab = OptionsTab::General;
        self.shortcut_editor = Some(ShortcutEditorState::new(current_config));
        self.pending_changes = false;
    }

    /// Render the options dialog
    pub fn show(
        &mut self,
        ctx: &Context,
        i18n: &crate::i18n::I18n,
    ) -> (bool, bool, Option<DialogResult>) {
        if !self.show {
            return (false, false, None);
        }

        let mut open = true;
        let mut result = None;

        egui::Window::new(i18n.t("options.title"))
            .open(&mut open)
            .default_size([600.0, 500.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                self.show_content(ui, i18n, &mut result);
            });

        if !open {
            self.show = false;
            self.shortcut_editor = None;
        }

        (self.show, open, result)
    }

    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &crate::i18n::I18n,
        result: &mut Option<DialogResult>,
    ) {
        // Tab selector
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, OptionsTab::General, i18n.t(OptionsTab::General.name_key()));
            ui.selectable_value(&mut self.active_tab, OptionsTab::Shortcuts, i18n.t(OptionsTab::Shortcuts.name_key()));
        });

        ui.separator();

        // Tab content
        match self.active_tab {
            OptionsTab::General => self.show_general_tab(ui, i18n),
            OptionsTab::Shortcuts => self.show_shortcuts_tab(ui, i18n),
        }

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(i18n.t("options.cancel")).clicked() {
                    *result = Some(DialogResult::Cancel);
                }
                if ui.button(i18n.t("options.ok")).clicked() {
                    *result = Some(DialogResult::Ok {
                        language: self.selected_language,
                        auto_save: self.auto_save_enabled,
                        shortcut_config: self.shortcut_editor.as_ref().map(|e| e.working_config.clone()),
                    });
                }
            });
        });
    }

    fn show_general_tab(&mut self, ui: &mut egui::Ui, i18n: &crate::i18n::I18n) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // Language selection
            ui.horizontal(|ui| {
                ui.label(i18n.t("options.language"));
                let mut lang_text = self.selected_language.name();
                if egui::ComboBox::from_id_salt("options_language")
                    .selected_text(&lang_text)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.selected_language == Language::ZhCN, Language::ZhCN.name()).clicked() {
                            self.selected_language = Language::ZhCN;
                        }
                        if ui.selectable_label(self.selected_language == Language::EnUS, Language::EnUS.name()).clicked() {
                            self.selected_language = Language::EnUS;
                        }
                    })
                    .changed()
                {
                    self.pending_changes = true;
                }
            });

            ui.add_space(20.0);

            // Auto save checkbox
            if ui.checkbox(&mut self.auto_save_enabled, i18n.t("options.auto_save")).changed() {
                self.pending_changes = true;
            }
        });
    }

    fn show_shortcuts_tab(&mut self, ui: &mut egui::Ui, i18n: &crate::i18n::I18n) {
        if let Some(ref mut editor) = self.shortcut_editor {
            // Reuse the existing shortcut editor UI
            // This is a simplified version - the full implementation would
            // delegate to a method that renders the shortcut editor content
            ui.vertical(|ui| {
                ui.label(i18n.t("shortcuts.search"));
                ui.text_edit_singleline(&mut editor.search_filter);

                ui.separator();

                let filtered = editor.get_filtered_shortcuts();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (action, binding) in filtered {
                        ui.horizontal(|ui| {
                            ui.label(i18n.t(action.description_key()));
                            ui.separator();
                            let shortcut_text = format_shortcut(binding);
                            ui.label(shortcut_text);
                        });
                    }
                });
            });
        }
    }
}

/// Dialog result
pub enum DialogResult {
    Cancel,
    Ok {
        language: Language,
        auto_save: bool,
        shortcut_config: Option<crate::shortcuts::ShortcutConfig>,
    },
}

fn format_shortcut(binding: &crate::shortcuts::ShortcutBinding) -> String {
    let mut parts = Vec::new();
    if binding.ctrl { parts.push("Ctrl"); }
    if binding.shift { parts.push("Shift"); }
    if binding.alt { parts.push("Alt"); }
    parts.push(&binding.key);
    parts.join("+")
}
```

**Step 2: 将模块添加到 app.rs**

在 `lab-gui/src/app.rs` 的模块声明区域添加：

```rust
mod options_dialog;
use options_dialog::{OptionsDialogState, DialogResult};
```

**Step 3: 在 LabApp 结构体中添加字段**

找到 `shortcut_editor: Option<crate::shortcuts::ShortcutEditorState>,` 后添加：

```rust
shortcut_editor: Option<crate::shortcuts::ShortcutEditorState>,
options_dialog: OptionsDialogState,
```

**Step 4: 在 LabApp::new() 中初始化**

找到 `shortcut_editor: None,` 后添加：

```rust
shortcut_editor: None,
options_dialog: OptionsDialogState::new(crate::i18n::Language::ZhCN, false),
```

**Step 5: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译可能有错误（需要修复导入和类型）

**Step 6: 修复编译错误**

根据编译错误信息，修复：
- 缺失的导入
- 类型不匹配
- 方法调用错误

**Step 7: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 8: 提交**

```bash
git add lab-gui/src/app/options_dialog.rs lab-gui/src/app.rs
git commit -m "feat: add options dialog module"
```

---

### Task 3: 创建关于对话框模块

**Files:**
- Create: `lab-gui/src/app/about_dialog.rs`

**Step 1: 创建关于对话框文件**

创建 `lab-gui/src/app/about_dialog.rs`：

```rust
// About dialog for JLab
use egui::Context;

/// About dialog state
pub struct AboutDialogState {
    pub show: bool,
}

impl AboutDialogState {
    pub fn new() -> Self {
        Self { show: false }
    }

    /// Show the about dialog
    pub fn show_dialog(&mut self) {
        self.show = true;
    }

    /// Render the about dialog
    pub fn show(&mut self, ctx: &Context, i18n: &crate::i18n::I18n) -> bool {
        if !self.show {
            return false;
        }

        let mut open = true;

        egui::Window::new(i18n.t("about.title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                self.show_content(ui, i18n);
            });

        if !open {
            self.show = false;
        }

        self.show
    }

    fn show_content(&mut self, ui: &mut egui::Ui, i18n: &crate::i18n::I18n) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // App name and title
            ui.heading(i18n.t("app.title"));
            ui.label(format!("{}: {}", i18n.t("about.version"), env!("CARGO_PKG_VERSION")));

            ui.add_space(20.0);

            // Description
            ui.label(i18n.t("about.description"));
            ui.label(i18n.t("about.features"));

            ui.add_space(20.0);

            // License
            ui.label(i18n.t("about.license"));

            ui.add_space(20.0);

            // Website
            if ui.link("🌐 GitHub").clicked() {
                let _ = open::that("https://github.com");
            }

            ui.add_space(20.0);

            // OK button
            if ui.button(i18n.t("options.ok")).clicked() {
                self.show = false;
            }
        });
    }
}
```

**Step 2: 将模块添加到 app.rs**

在 `lab-gui/src/app.rs` 的模块声明区域添加：

```rust
mod about_dialog;
use about_dialog::AboutDialogState;
```

**Step 3: 在 LabApp 结构体中添加字段**

找到 `options_dialog: OptionsDialogState,` 后添加：

```rust
options_dialog: OptionsDialogState,
about_dialog: AboutDialogState,
```

**Step 4: 在 LabApp::new() 中初始化**

找到 `options_dialog: OptionsDialogState::new(...),` 后添加：

```rust
options_dialog: OptionsDialogState::new(crate::i18n::Language::ZhCN, false),
about_dialog: AboutDialogState::new(),
```

**Step 5: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 6: 提交**

```bash
git add lab-gui/src/app/about_dialog.rs lab-gui/src/app.rs
git commit -m "feat: add about dialog module"
```

---

### Task 4: 修改文件菜单添加选项条目

**Files:**
- Modify: `lab-gui/src/app/menu.rs`

**Step 1: 找到文件菜单的"关闭项目"条目位置**

搜索 `menu.file_close`，在其前面的分隔符之后添加选项条目。

**Step 2: 添加选项菜单条目**

在"关闭项目"之前的分隔符后添加：

```rust
// Options menu item
ui.separator();

let options_label = self.state.i18n.t("menu.file_options");
let options_hint = self.state.i18n.t("hint.file_options");
let options_response = ui.button(options_label.clone());
Self::update_status_hint(status_hint, &options_response, options_hint);
if options_response.clicked() {
    // Show options dialog
    ui.close_menu();
}
```

**Step 3: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功（但点击无效果，后续任务处理）

**Step 4: 提交**

```bash
git add lab-gui/src/app/menu.rs
git commit -m "feat: add options menu item to file menu"
```

---

### Task 5: 重构帮助菜单

**Files:**
- Modify: `lab-gui/src/app/menu.rs`

**Step 1: 找到帮助菜单开始位置**

搜索 `menu.help` 或 `help_menu_label`。

**Step 2: 移除语言子菜单**

找到语言子菜单代码（`language_response = ui.menu_button(...)`），将其删除。

**Step 3: 移除快捷键说明内容**

删除从帮助菜单开始到快捷键设置按钮之间的所有 `ui.label()` 快捷键说明内容。

**Step 4: 添加新的帮助菜单结构**

用以下代码替换原帮助菜单内容：

```rust
ui.button(self.state.i18n.t("menu.help_help"));
ui.separator();
ui.button(self.state.i18n.t("menu.help_check_updates"));
ui.separator();
ui.button(self.state.i18n.t("menu.help_about"));
```

**Step 5: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 6: 提交**

```bash
git add lab-gui/src/app/menu.rs
git commit -m "refactor: restructure help menu to Windows standard"
```

---

### Task 6: 在 app.rs 中集成对话框显示

**Files:**
- Modify: `lab-gui/src/app.rs`

**Step 1: 在 update() 方法中添加对话框调用**

找到 `self.show_shortcut_settings(ctx);` 后添加：

```rust
self.show_shortcut_settings(ctx);
self.show_options_dialog(ctx);
self.show_about_dialog(ctx);
```

**Step 2: 实现 show_options_dialog() 方法**

```rust
fn show_options_dialog(&mut self, ctx: &egui::Context) {
    if !self.state.show_options_dialog {
        return;
    }

    if self.options_dialog.shortcut_editor.is_none() {
        self.options_dialog.show_dialog(self.state.shortcut_manager.get_config());
    }

    let (show, open, result) = self.options_dialog.show(ctx, &self.state.i18n);

    if let Some(dialog_result) = result {
        match dialog_result {
            DialogResult::Cancel => {
                // Discard changes
            }
            DialogResult::Ok { language, auto_save, shortcut_config } => {
                // Apply language change
                if language != self.state.language {
                    let _ = self.state.set_language(language);
                }

                // Apply auto-save change
                if let Some(project) = &mut self.state.current_project {
                    project.meta.shape.auto_save = auto_save;
                }

                // Apply shortcut changes
                if let Some(config) = shortcut_config {
                    self.state.shortcut_manager.apply_config(config);
                    if let Some(path) = crate::shortcuts::ShortcutManager::user_config_path() {
                        let _ = self.state.shortcut_manager.save_to_file(&path);
                    }
                }
            }
        }
    }

    if !open {
        self.state.show_options_dialog = false;
        self.options_dialog.shortcut_editor = None;
    }
}
```

**Step 3: 实现 show_about_dialog() 方法**

```rust
fn show_about_dialog(&mut self, ctx: &egui::Context) {
    self.about_dialog.show(ctx, &self.state.i18n);
}
```

**Step 4: 连接菜单项到对话框**

修改菜单代码，在按钮点击时设置状态：

文件菜单选项按钮：
```rust
if options_response.clicked() {
    self.state.show_options_dialog = true;
    ui.close_menu();
}
```

帮助菜单关于按钮：
```rust
if ui.button(self.state.i18n.t("menu.help_about")).clicked() {
    self.about_dialog.show_dialog();
    ui.close_menu();
}
```

**Step 5: 编译检查**

Run: `cargo build -p lab-gui`
Expected: 编译成功

**Step 6: 提交**

```bash
git add lab-gui/src/app.rs lab-gui/src/app/menu.rs
git commit -m "feat: integrate options and about dialogs with menu"
```

---

### Task 7: 添加中文翻译

**Files:**
- Modify: `lab-gui/locales/zh-CN.json`

**Step 1: 添加 menu 节点新条目**

在 `"help": "帮助",` 后添加：

```json
"help_help": "帮助",
"help_check_updates": "检查更新...",
"help_about": "关于 JLab"
```

在 `"file_exit": "退出",` 后添加：

```json
"file_options": "选项..."
```

**Step 2: 添加 hint 节点新条目**

在 hints 区域添加：

```json
"hint.file_options": "打开应用程序选项",
"hint.help_help": "查看用户手册",
"hint.help_check_updates": "检查是否有新版本",
"hint.help_about": "关于本应用程序"
```

**Step 3: 添加 options 节点**

在文件末尾添加：

```json
"options": {
  "title": "选项",
  "tab_general": "常规",
  "tab_shortcuts": "快捷键",
  "language": "界面语言:",
  "auto_save": "自动保存",
  "ok": "确定",
  "cancel": "取消"
}
```

**Step 4: 添加 about 节点**

在文件末尾添加：

```json
"about": {
  "title": "关于 JLab",
  "description": "JLab 是一款专业的 2D 目标检测与属性分类的图像标注工具。",
  "features": "支持多边形标注、ROI 标注，以及 YOLO、VOC、COCO 等多种格式导出。",
  "license": "许可证: MIT OR Apache-2.0",
  "version": "版本",
  "website": "项目主页"
}
```

**Step 5: 验证 JSON 格式**

Run: `cat lab-gui/locales/zh-CN.json | jq .`
Expected: JSON 格式有效

**Step 6: 提交**

```bash
git add lab-gui/locales/zh-CN.json
git commit -m "i18n: add Chinese translations for options and about dialogs"
```

---

### Task 8: 添加英文翻译

**Files:**
- Modify: `lab-gui/locales/en-US.json`

**Step 1: 添加 menu 节点新条目**

对应中文位置添加英文：

```json
"help_help": "Help",
"help_check_updates": "Check for Updates...",
"help_about": "About JLab",
"file_options": "Options..."
```

**Step 2: 添加 hint 节点新条目**

```json
"hint.file_options": "Open application options",
"hint.help_help": "View user manual",
"hint.help_check_updates": "Check for new version",
"hint.help_about": "About this application"
```

**Step 3: 添加 options 节点**

```json
"options": {
  "title": "Options",
  "tab_general": "General",
  "tab_shortcuts": "Shortcuts",
  "language": "Interface Language:",
  "auto_save": "Auto Save",
  "ok": "OK",
  "cancel": "Cancel"
}
```

**Step 4: 添加 about 节点**

```json
"about": {
  "title": "About JLab",
  "description": "JLab is a professional 2D object detection and attribute classification image annotation tool.",
  "features": "Supports polygon annotation, ROI annotation, and export to YOLO, VOC, COCO and other formats.",
  "license": "License: MIT OR Apache-2.0",
  "version": "Version",
  "website": "Website"
}
```

**Step 5: 验证 JSON 格式**

Run: `cat lab-gui/locales/en-US.json | jq .`
Expected: JSON 格式有效

**Step 6: 提交**

```bash
git add lab-gui/locales/en-US.json
git commit -m "i18n: add English translations for options and about dialogs"
```

---

### Task 9: 手动功能测试

**Files:**
- Test: 运行中的应用程序

**Step 1: 构建并运行**

Run: `cargo run -p lab-gui`
Expected: 程序正常启动

**Step 2: 测试选项对话框**

操作: 文件菜单 → 选项
Expected:
- 选项对话框打开
- 有"常规"和"快捷键"两个标签页
- 常规页显示语言和自动保存选项
- 快捷键页显示快捷键列表

**Step 3: 测试关于对话框**

操作: 帮助菜单 → 关于 JLab
Expected:
- 关于对话框打开
- 显示应用名称、版本号、描述

**Step 4: 测试语言切换**

在选项对话框中切换语言
Expected: 界面语言立即更新

**Step 5: 测试快捷键设置**

在选项对话框的快捷键标签页中修改快捷键
Expected: 可以正常修改和保存

**Step 6: 验证帮助菜单**

检查帮助菜单内容
Expected:
- 只有"帮助"、"检查更新"、"关于 JLab"三个条目
- 语言和快捷键设置已移除

---

## 验收标准

完成所有任务后：
1. ✅ 文件菜单有"选项..."条目
2. ✅ 选项对话框包含常规和快捷键两个标签页
3. ✅ 帮助菜单只有：帮助、检查更新、关于
4. ✅ 关于对话框显示版本号和描述
5. ✅ 语言设置在选项对话框中正常工作
6. ✅ 快捷键设置在选项对话框中正常工作
7. ✅ 所有变更立即生效
8. ✅ 中英文翻译完整
