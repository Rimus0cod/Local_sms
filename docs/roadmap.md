# Roadmap

## Phase 0 — Foundation

Сделано в этом репозитории:

- структура проекта
- доменные сущности и проверки ограничений
- desktop UI shell на Tauri 2 + React + Zustand
- архитектурные документы

## Phase 1 — MVP

- локальная генерация device identity
- создание одной закрытой группы
- mDNS discovery в одной Wi-Fi сети
- текстовые сообщения
- локальная история
- системные уведомления

Уже закрыто фундаментом:

- device identity и prekeys
- safety number и QR verification
- LAN discovery
- QUIC transport
- pairwise secure session
- encrypted local storage for devices, peers, messages and local key material
- reliable pairwise messaging with ACK, retry and ordering
- group sender keys with epoch rotation for membership changes
- desktop client surfaces for chats, LAN peers and device verification
- replay hardening, sender-key conflict guards and forward-secrecy validation snapshots

## Phase 2 — Base Version

- передача файлов и изображений
- локальный поиск
- экспорт истории
- реакции и ответы
- мультиустройственность для одного пользователя
- wiring desktop UI к mDNS discovery и durable storage pipeline

## Phase 3 — Full Version

- voice notes
- голосовые звонки
- видеозвонки
- интернет-P2P режим
- iOS polish

## Принцип поставки

Каждая следующая фаза должна сохранять:

- работу без облака
- локальное хранение
- криптографическую проверяемость
- управляемую сложность для группы до 8 человек
