# Technical Debt

## TD-TRAY-001: Нестабильный правый клик по tray-иконкам (SNI)

- Статус: `closed` (2026-03-25, release `0.6.53`)
- Приоритет: `high`
- Область: `src/modules/tray.rs`, взаимодействие SNI/DBus (`ContextMenu`/`SecondaryActivate`)
- Симптом: правый клик работает хаотично (контекстное меню открывается не всегда).
- Решение:
  - Введена детерминированная стратегия `per-item`:
    - стартовый выбор по capability (`item_is_menu`/`menu`),
    - после первого успешного клика успешный метод закрепляется для конкретного item (`ContextMenu` или `SecondaryActivate`),
    - в рамках одного клика выполняется не более одного fallback.
  - Диагностика расширена: логируются id, primary/fallback маршрут, pinned action, координаты курсора, результат и latency.
  - Добавлены регрессионные тесты на маршрутизацию и mock/stub сценарий исполнения клика.

### Наблюдаемые паттерны SNI-реализаций
1. Menu-centric items (`item_is_menu=true` или явный `menu`) стабильнее реагируют на `ContextMenu`.
2. Action-centric items (без menu capability) чаще корректно обрабатывают `SecondaryActivate`.
3. Универсального единого метода для всех приложений нет, поэтому закрепление успешного метода per-item снижает хаотичность при повторных кликах.

### Как воспроизвести
1. Запустить `thinkpadbar`.
2. Нажимать `ПКМ` по иконке в трее (включая кейс с id `152866`).
3. Наблюдать, что меню появляется нестабильно.

### Критерии готовности
- `ПКМ` открывает контекстное меню стабильно в 100% кликов в серии минимум 30 кликов на нескольких tray-приложениях.
- Нет побочных UX-эффектов (перенос курсора, задержки, “двойная реакция”).
- Полный quality gate зеленый:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`

## TD-TRAY-002: Локальное tray-меню не выполняет действия и плохо якорится

- Статус: `closed` (2026-03-25, release `0.6.74`)
- Приоритет: `high`
- Область: `src/app.rs`, `src/services/tray.rs`, `src/services/tray_ui.rs`, `src/services/tray_menu.rs`
- Симптомы:
  - локальное меню трея открывается стабильно, но пункты меню не всегда вызывают реальное действие в приложении-источнике;
  - popup меню отображается далеко от курсора/иконки трея (ошибка якорения позиции).
- Гипотезы причин:
  - неполная синхронизация актуального `DBusMenu` layout перед dispatch (`about_to_show` / submenu state / id path);
  - текущий popup-серфейс закреплен по `TOP|RIGHT` и не учитывает координату клика как anchor.

### Что уже сделано в `0.6.61`
- Popup tray-меню переведен на динамический anchor по курсору (`TOP|LEFT` + margin от текущей позиции курсора), вместо фиксированного правого края.
- Dispatch пункта меню усилен префетч-последовательностью `about_to_show(0)` + `about_to_show(selected_id)` перед `clicked`.
- В UI добавлена валидация `menu_item_id` по локальной owned-модели меню перед отправкой команды.

### Что дополнительно сделано в `0.6.62`
- Обработан `UpdateEvent::MenuDiff`: инкрементальные обновления меню теперь применяются к `menu_layout`, после чего owned-модель меню пересобирается.
- На reconnect tray-клиента сбрасывается runtime cache активации (resolved address cache, preferred secondary actions, контекстное DBus-соединение), чтобы не переиспользовать устаревший state.

### Что окончательно сделано в `0.6.74`
- Owned tray menu теперь хранит ancestry-путь для каждого action (`submenu ancestors + selected id`), и runtime dispatch префетчит полный `about_to_show` sequence вместо `root + selected` без контекста.
- Submenu-контейнеры (`children-display=submenu`) в локальном flat-menu больше не dispatch'ят “мертвый” click: они показываются как неактивируемые branch items, а реальные дочерние actions остаются кликабельными.
- Popup tray-меню теперь получает menu-aware height hint, поэтому short/medium menus открываются рядом с триггером без тяжелого фиксированного `420px` surface.
- Добавлены регрессионные тесты на ancestry-aware dispatch path, non-activatable submenu headers и popup height planning.

### Критерии готовности
- Нажатие на `enabled` пункт локального tray-меню детерминированно вызывает действие в 100% кликов на целевых приложениях.
- Меню появляется рядом с триггером (иконка/курсор) с предсказуемым смещением, без “улета” в дальний угол.
- Добавлены регрессионные тесты:
  - на dispatch menu-item команды (путь/идентификатор/`about_to_show` sequencing),
  - на расчет и применение позиции popup-якоря.
- Полный quality gate зеленый:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features`
