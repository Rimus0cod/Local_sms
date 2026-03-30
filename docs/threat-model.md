# Threat Model

## Что защищаем

- Тексты сообщений
- Вложения
- Состав группы
- Ключи устройств
- Историю переписки на локальном устройстве

## Доверительные допущения

- Участники группы знают друг друга в реальной жизни или доверяют каналу обмена QR/safety number.
- Устройство пользователя может быть потеряно или скомпрометировано, поэтому нужен отзыв устройств и rekey.
- Локальная сеть не считается доверенной.

## Основные угрозы

### MITM при подключении нового устройства

Риск:

- злоумышленник в LAN подменяет открытые ключи

Защита:

- QR verification
- manual safety number
- pending-state до ручного подтверждения
- same-epoch sender-key replacement теперь считается ошибкой протокола, а не silently accepted update

### Прослушка локальной сети

Риск:

- кто-то в Wi-Fi видит пакеты discovery и transport

Защита:

- только E2EE payload
- transport metadata минимизирована
- история и вложения никогда не передаются в открытом виде
- replayed pairwise и group messages отбрасываются на ratchet/engine уровне

### Старая ключевая эпоха после удаления участника

Риск:

- бывший участник продолжает читать новые сообщения

Защита:

- обязательная ротация group epoch
- новый sender key рассылается только оставшимся валидным устройствам

### Потеря телефона или ноутбука

Риск:

- локальная база и файлы попадают к третьему лицу

Защита:

- шифрование локального хранилища
- отзыв устройства из группы
- повторная ротация группы

Текущий статус в репозитории:

- local records в SQLite уже хранятся как encrypted blobs
- identity key material и prekeys тоже сохраняются только в зашифрованном виде
- identifiers для lookup не используются как plaintext row keys, вместо этого применяются детерминированные hash keys
- pairwise messaging уже использует encrypted ACK и duplicate-safe retry поверх secure session
- group messaging уже использует per-sender signing keys и epoch rotation foundation при изменении membership
- desktop shell ограничивает IPC зарегистрированными Tauri-командами, а verification actions внутри UI идут через реальные проверки `crates/core`
- group sender-key import теперь idempotent только для идентичного distribution; конфликтующие distribution в той же epoch отклоняются
- forward secrecy теперь валидируется state snapshots поверх Double Ratchet и secure-session layer

## Не-цели текущего инкремента

- Защита от полностью скомпрометированной ОС.
- Анонимность от самих участников группы.
- Безопасная работа без какого-либо ручного подтверждения идентичности.
- Browser fallback frontend не рассматривается как production deployment path и существует только для локальной разработки UI.

## Критичное замечание

Самодельную криптографию использовать нельзя. В текущем репозитории уже используются аудируемые примитивы для:

- key agreement
- AEAD
- secure random
- подписи и fingerprint/safety number derivation

Следующие критичные задачи после этого шага:

- durable replay protection across restarts
- end-to-end fan-out и distribution orchestration для sender keys
- encrypted attachment storage
- review group sender-key lifecycle
