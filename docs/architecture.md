# Архитектура

## Цели

- Работать без интернета после первичной настройки.
- Оставаться понятным и удобным как обычный мессенджер.
- Не иметь центрального сервера и облачного хранения.
- Поддерживать маленькую доверенную группу до 8 участников.

## Предлагаемый стек

- `Rust core`
  - домен чатов и участников
  - управление ключами и эпохами группы
  - локальное хранилище
  - discovery, transport и синхронизация
- `Tauri 2 shell`
  - один UI-код для desktop и mobile
  - доступ к системным уведомлениям и файловым вложениям
- `LAN-first networking`
  - mDNS/Zeroconf для поиска устройств в одной сети
  - QUIC для прямого защищенного канала
  - Bluetooth как запасной транспортный адаптер

## Модули системы

### 1. Identity

- Локальная генерация device identity keypair.
- Отдельный `device_id` и `member_id`.
- Safety number для ручной верификации.

### 2. Invite & Verification

- Инвайт через QR, временную ссылку или ручной код.
- Join flow обязательно привязывается к конкретной группе и конкретному устройству-инвайтеру.
- После приема участника устройства должны быть помечены как `Pending` до подтверждения QR/safety number.

### 3. Discovery

- mDNS публикует наличие клиента в LAN.
- Клиент объявляет только минимум метаданных: служебный service name, device alias, поддерживаемые transport capabilities.
- Никаких текстов сообщений или истории на discovery-уровне.

### 4. Secure Transport

- Прямое соединение устройство-устройство.
- Приоритет: LAN IP + QUIC.
- Позже: NAT hole punching и опциональные STUN/TURN-коннекторы, если пользователь сам их поднимает.

## Криптографическая модель

Для группы 4–8 человек прагматичная модель такая:

- долговременный identity key на устройство
- проверка устройства по QR/safety number
- pairwise session bootstrap между устройствами
- sender key на группу для текущей эпохи
- обязательная ротация group epoch при удалении участника или отзыве устройства

Это ближе к Signal-style group messaging, чем к "чистому" Noise-only подходу, и лучше соответствует требованию удобства и небольшой группы.

## Хранение

- Вся история локально на устройстве.
- Вложения хранятся локально с checksum и ссылкой на message envelope.
- Поиск выполняется локально по индексу сообщений.
- Экспорт истории возможен только по явному действию пользователя.

## Что реализовано сейчас

- Rust workspace с отдельными crates для core, crypto, discovery и transport.
- Криптографический foundation: identity keys, signed prekeys, one-time prekeys, X3DH bootstrap и Double Ratchet.
- Модель участников и устройств: multi-device, safety number и QR verification payloads.
- LAN discovery: mDNS presence, TXT metadata и peer registry.
- QUIC transport: endpoint bind/listen, certificate pinning, reconnect policy и framed uni-stream transport.
- Secure session integration: X3DH handshake поверх QUIC, binding responder device identity к pinned transport certificate и encrypted payload exchange через Double Ratchet.
- Storage layer: SQLite через `sqlx`, encrypted blobs для devices/peers/messages и отдельное зашифрованное хранение identity/prekey material.
- Messaging engine: encrypted user envelopes, ACK system, retry queue и in-order delivery поверх pairwise secure session.
- Group messaging: sender-key distribution, per-sender signing keys и group epoch rotation при изменении состава участников.
- Tauri 2 desktop shell c React + Zustand клиентом: chat list, chat window, LAN peer panel и verification workspace.
- Rust-side desktop commands используют реальные типы `crates/core` для QR/safety verification, а не отдельную UI-логику.
- Direct desktop chats уже ходят через живую loopback runtime-связку `transport -> secure session -> messaging engine`, а не через UI-only mock state.
- Browser fallback оставлен только для frontend-разработки без desktop-shell.
- Security hardening: bounded replay tracking в pairwise engine, sender-key replacement guard в group layer и forward-secrecy state snapshots для ratchet/session validation.

## Что еще не реализовано

- Файловое хранилище вложений и индексы поиска.
- Durable delivery state across restart и anti-entropy sync.
- Оркестрация fan-out sender-key distribution поверх реальных pairwise sessions для всей группы.
- Полное подключение mDNS discovery и storage pipeline к desktop UI вместо текущего runtime-backed peer registry.
- Mobile client и platform-specific polish.
