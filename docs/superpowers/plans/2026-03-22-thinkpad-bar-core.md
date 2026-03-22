# ThinkPadBar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Создать базовую статусную панель для Wayland (Hyprland), работающую как Layer Shell, с первым функциональным модулем (часы).

**Architecture:** Модульное приложение на Rust с использованием `iced` и `iced_layershell`. Архитектура MVU (Model-View-Update). Асинхронное получение данных через `iced::Subscription`.

**Tech Stack:** Rust, iced, iced_layershell, tokio, chrono.

---

### Task 1: Инициализация проекта и зависимости

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Добавить необходимые зависимости в Cargo.toml**
```toml
[dependencies]
iced = { version = "0.13", features = ["tokio", "advanced"] }
iced_layershell = "0.7"
tokio = { version = "1.0", features = ["full"] }
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
```

- [ ] **Step 2: Запустить сборку для проверки зависимостей**
Run: `cargo check`
Expected: Успешная компиляция (без ошибок отсутствующих библиотек).

- [ ] **Step 3: Commit**
```bash
git add Cargo.toml
git commit -m "chore: add initial dependencies for iced and wayland"
```

---

### Task 2: Базовое окно Layer Shell

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Создать структуру приложения в src/app.rs**
```rust
use iced_layershell::reexport::Anchor;
use iced_layershell::Settings;
use iced::{Element, Task, Theme};

pub struct ThinkPadBar {
    // Состояние будет здесь
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
}

impl ThinkPadBar {
    pub fn new() -> (Self, Task<Message>) {
        (Self {}, Task::none())
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        "ThinkPadBar MVP".into()
    }
}
```

- [ ] **Step 2: Настроить точку входа в src/main.rs**
```rust
mod app;
use app::ThinkPadBar;
use iced_layershell::Settings;
use iced_layershell::reexport::Anchor;

fn main() -> Result<(), iced_layershell::Error> {
    let settings = Settings {
        anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
        height: 32,
        ..Default::default()
    };
    
    iced_layershell::run::<ThinkPadBar, iced::Theme, iced_layershell::proxy::ProxyMessage>(settings)
}
```

- [ ] **Step 3: Запустить приложение в Hyprland**
Run: `cargo run`
Expected: Появление тонкой черной полоски сверху экрана с текстом "ThinkPadBar MVP".

- [ ] **Step 4: Commit**
```bash
git add src/main.rs src/app.rs
git commit -m "feat: basic wayland layer shell window"
```

---

### Task 3: Модуль часов (Clock)

**Files:**
- Create: `src/modules/clock.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Реализовать логику времени в src/modules/clock.rs**
```rust
use chrono::{Local, DateTime};
use std::time::Duration;
use iced::subscription;

pub fn tick() -> iced::Subscription<crate::app::Message> {
    subscription::unfold("clock-tick", (), |_| async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        (crate::app::Message::Tick, ())
    })
}
```

- [ ] **Step 2: Обновить src/app.rs для поддержки подписки и отображения времени**
```rust
// В impl ThinkPadBar
pub fn subscription(&self) -> iced::Subscription<Message> {
    crate::modules::clock::tick()
}

pub fn view(&self) -> Element<Message> {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    now.into()
}
```

- [ ] **Step 3: Проверить обновление времени**
Run: `cargo run`
Expected: Время на панели обновляется каждую секунду.

- [ ] **Step 4: Commit**
```bash
git add src/modules/clock.rs src/app.rs
git commit -m "feat: add clock module with 1s subscription"
```
