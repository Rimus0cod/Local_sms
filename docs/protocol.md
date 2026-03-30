# Протокол MVP

## 1. Создание группы

1. Создатель генерирует `group_id`, локальный профиль группы и первую `group_epoch = 1`.
2. На устройстве создается identity keypair.
3. Группа получает политику: максимум 8 участников, лимит вложений, доступные функции.

## 2. Инвайт

Инвайт должен содержать:

- `group_id`
- идентификатор приглашающего устройства
- одноразовый или ограниченный по времени код
- тип транспорта: `qr`, `link`, `manual`

QR-код должен содержать только минимальные данные для старта handshake, а не историю или постоянные секреты группы.

## 3. Discovery в LAN

- Клиент публикует локальный сервис, например `_rimus-chat._udp.local`.
- Другие устройства видят только online presence и transport endpoints.
- После нахождения устройства запускается защищенный join handshake.

## 4. Join Handshake

Упрощенный поток:

1. `join-hello`
   - group id
   - invite token
   - temporary device public key
2. `join-accept`
   - подтверждение инвайта
   - identity data приглашающего
   - параметры текущей group epoch
3. `verify`
   - QR или safety number подтверждает, что ключи не подменены
4. `epoch-sync`
   - новое устройство получает sender key текущей эпохи

Текущий pairwise secure-session слой уже реализует нижнюю часть этого потока для device-to-device канала:

1. инициатор открывает QUIC connection с pinning transport certificate
2. отправляет `secure-session-request`
   - локальный device descriptor
   - ожидаемый responder device descriptor
   - fingerprint ожидаемого transport certificate
   - X3DH initiator handshake
3. responder проверяет:
   - что запрос адресован именно этому устройству
   - что identity keys инициатора совпадают с X3DH handshake
   - что fingerprint transport certificate совпадает с локальным endpoint
4. responder возвращает `secure-session-response`
   - responder device descriptor
   - fingerprint transport certificate
   - metadata по использованному one-time prekey
5. обе стороны поднимают Double Ratchet и дальше обмениваются только encrypted payload frames

## 5. Message Envelope

Каждое сообщение логически состоит из:

- `message_id`
- `group_id`
- `group_epoch`
- `sender_device_id`
- `timestamp`
- `payload_type`
- `ciphertext`
- `attachment descriptors`

Payload types:

- text
- attachment
- voice_note
- system_event

На текущем pairwise messaging layer поверх secure session реально используется такой delivery envelope:

- `version`
- `message_id`
- `conversation_id`
- `delivery_order`
- `sent_at_unix_ms`
- `kind`
- `body`

И отдельный encrypted ACK envelope:

- `message_id`
- `delivery_order`

Этот шаг уже реализован для pairwise device-to-device канала:

- sender держит pending outbound queue
- receiver подтверждает доставку encrypted ACK-ом
- sender удаляет pending message только после ACK
- retry того же `message_id` не даёт повторной доставки
- out-of-order arrival буферизуется и выдаётся приложению только в правильном порядке

## 6. Presence

Presence-события не должны раскрывать историю чата. Они сообщают только:

- device id
- статус: `offline`, `lan_online`, `p2p_online`
- capabilities

## 7. Rekey

Перегенерация эпохи нужна, если:

- участник вышел из группы
- устройство участника отозвано
- safety number изменился и пользователь подтвердил замену

Поток:

1. `membership-change`
2. `rekey-notice`
3. доставка нового sender key только актуальным устройствам
4. переход группы на новый `group_epoch`

Текущий group layer уже реализует криптографическую основу этого шага:

- у каждого устройства есть свой sender key chain для текущей эпохи
- вместе с chain key устройство генерирует отдельный signing key для group messages
- sender key distribution содержит:
  - `group_id`
  - `epoch`
  - `sender_member_id`
  - `sender_device_id`
  - `distribution_id`
  - `chain_key_seed`
  - `signing_public_key`
- group message содержит:
  - `group_id`
  - `epoch`
  - `sender_member_id`
  - `sender_device_id`
  - `distribution_id`
  - `message_id`
  - `message_number`
  - `kind`
  - `ciphertext`
  - `signature`

Что уже работает в коде:

- импорт sender key distribution на стороне получателя
- проверка group message signature
- decrypt одного sender chain даже при out-of-order arrival
- rotation helper при добавлении участника
- rotation helper при удалении участника
- manual forward-secrecy rekey без изменения membership
- explicit device-compromise rotation reason для removal-driven rekey
- запрет same-epoch sender-key replacement для одного и того же устройства
- replay rejection для уже обработанных group messages

Что остаётся поверх этого:

- доставить sender key distribution всем актуальным участникам через pairwise sessions
- сохранить durable состояние sender chains через restart
- связать group ciphertext fan-out с настоящим networking layer

## 8. История и синхронизация

MVP сейчас хранит историю локально в SQLite:

- message records лежат как encrypted blobs
- device snapshots и peer snapshots тоже лежат в encrypted blobs
- local identity/prekey material хранится отдельно и шифруется тем же local storage key
- lookup по conversation/device/message идёт через детерминированные hash keys, а не через plaintext identifiers

Пока это локальное persistence-only хранилище без полноценного многопоточечного merge.

Следующий шаг:

- anti-entropy sync между доверенными устройствами
- durable replay/dedup state across restart
- экспорт истории в зашифрованный архив

## 9. Desktop Client Surface

Текущий desktop client в `apps/tauri-client` добавляет поверх этого протокольного фундамента:

- Rust command layer для snapshot, peer refresh, send message и verify device
- direct desktop chat send path, который реально проходит через QUIC transport, secure session и messaging engine
- verification workspace, который реально вызывает QR/safety validation из `crates/core`
- browser fallback только для разработки UI без Tauri shell

Важно:

- browser fallback не является production transport path
- live-binding для direct pairwise messaging уже работает внутри desktop runtime
- mDNS-backed peer discovery и durable storage binding в desktop client остаются следующими шагами

## 10. Hardening Rules

Текущий hardening pass добавляет следующие protocol-level guardrails:

- pairwise engine хранит bounded replay metadata window и не подтверждает конфликтующие incoming message envelopes
- group sender-key state не позволяет тихо подменить distribution того же sender в той же epoch
- локальный sender не может переиспользовать тот же `message_id` в рамках одной group epoch
- forward secrecy проверяется через state snapshots ratchet/session layer в unit tests
