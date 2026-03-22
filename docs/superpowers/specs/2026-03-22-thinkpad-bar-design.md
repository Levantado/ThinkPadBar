# Дизайн-спецификация: ThinkPadBar

**Дата:** 2026-03-22
**Статус:** Черновик (Design phase)
**Стек:** Rust, iced (GUI), wayland-layer-shell, pipewire, libinput, upower

## 1. Цели проекта
Создать специализированную статусную панель для ноутбуков ThinkPad (целевое устройство: T14 Gen 1 AMD), работающую в среде Hyprland (Wayland). Панель должна объединять стандартные функции (время, сеть) с глубоким управлением оборудованием ThinkPad (вентилятор, TrackPoint, пороги заряда).

## 2. Архитектура (Native Rust)
Панель строится на базе фреймворка **iced** с использованием архитектуры MVU (Model-View-Update).

### Основные компоненты:
*   **Bar App**: Основной цикл приложения, управляющий Layer Shell (расположение на экране).
*   **Module Engine**: Система изоляции модулей. Каждый модуль (WiFi, Battery) работает асинхронно через `iced::Subscription`.
*   **Hardware Interface (HWI)**: Прослойка для безопасного взаимодействия с `/proc/acpi/ibm/*` и `/sys/class/*`.

## 3. Модули и функциональность

### Группа "ThinkPad Hardware"
1.  **Fan Control**:
    *   Индикация RPM и режима (Auto/Manual).
    *   Попап с выбором уровней (0-7, auto, full-speed).
2.  **Input Manager (TrackPoint/Pad)**:
    *   Переключатели (Toggle) для TrackPoint и TrackPad.
    *   Слайдер чувствительности (Sensitivity) через `libinput`.
3.  **Power Management**:
    *   Отображение заряда и потребления (W).
    *   Быстрое переключение порогов заряда (80% / 100%) через `tp_smapi` или `thinkpad_acpi`.

### Группа "System & Network"
4.  **Resource Monitor**: CPU (%), RAM (GiB), Disk (%), Network (Mbps).
5.  **Connectivity**:
    *   WiFi: Список сетей в попапе (D-Bus / NetworkManager).
    *   Bluetooth: Статус и подключенные устройства.
6.  **Audio & Brightness**: Слайдеры в попапах, индикация Mute (красная иконка).

### Группа "Desktop"
7.  **Workspaces**: Интеграция с Hyprland (отображение активных столов).
8.  **Launcher & Applets**:
    *   Кнопка меню (Launchbar).
    *   Раскладка клавиатуры (RU/EN).
    *   Календарь (при клике на время).

## 4. Визуальный дизайн (Tokyo Night)
*   **Фон:** `#1a1b26` (прозрачность 95-98%).
*   **Акцент:** `#7aa2f7` (Blue), `#9ece6a` (Green) для статуса OK.
*   **Ошибки/Mute:** `#f7768e` (Red).
*   **Шрифт:** JetBrains Mono Nerd Font (для иконок и текста).

## 5. Безопасность и права
Для записи в `/proc/acpi/ibm/fan` и `/sys/class/backlight` потребуются:
*   udev-правила для предоставления прав пользователю.
*   Опционально: `polkit` для действий, требующих root.

## 6. План реализации (MVP)
1. Базовое окно Wayland через iced.
2. Модуль Workspaces (Hyprland).
3. Модуль Audio/Brightness.
4. Специфический модуль Fan Control для ThinkPad.
