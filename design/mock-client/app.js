const state = {
  language: "ru",
  theme: "dark",
  query: "",
};

const copy = {
  ru: {
    eyebrow: "Оффлайн-first messenger",
    title: "Local Messenger",
    theme: "Светлая тема",
    rooms: "Комнаты",
    chatListTitle: "Маленькая закрытая группа",
    search: "Поиск по истории",
    activeRoom: "Активный чат",
    e2ee: "E2EE включено",
    emoji: "😊",
    messagePlaceholder: "Сообщение",
    send: "Отправить",
    roomInfo: "Комната",
    securityTitle: "Доверие и безопасность",
    inviteLabel: "Одноразовый инвайт",
    inviteExpiry: "Истекает через 10 минут",
    membersTitle: "Участники",
    securityChecklist: "Ключевые гарантии",
    checklistOne: "QR-проверка нового устройства перед полной активацией",
    checklistTwo: "Ротация эпохи группы при удалении участника",
    checklistThree: "Локальное хранение истории и вложений без облака",
    checklistFour: "LAN-first соединение с запасным Bluetooth-режимом",
  },
  en: {
    eyebrow: "Offline-first messenger",
    title: "Local Messenger",
    theme: "Dark theme",
    rooms: "Rooms",
    chatListTitle: "Small private group",
    search: "Search history",
    activeRoom: "Active room",
    e2ee: "E2EE enabled",
    emoji: "😊",
    messagePlaceholder: "Message",
    send: "Send",
    roomInfo: "Room",
    securityTitle: "Trust and safety",
    inviteLabel: "One-time invite",
    inviteExpiry: "Expires in 10 minutes",
    membersTitle: "Members",
    securityChecklist: "Core guarantees",
    checklistOne: "QR verification before a new device becomes fully trusted",
    checklistTwo: "Automatic group epoch rotation when someone leaves",
    checklistThree: "History and attachments stay local with no cloud copy",
    checklistFour: "LAN-first routing with Bluetooth as the backup path",
  },
};

const chats = {
  ru: [
    {
      title: "Домашняя сеть",
      preview: "Клип загружен и доступен только в локальной группе",
      time: "18:42",
      members: 5,
      active: true,
    },
    {
      title: "Выезд на дачу",
      preview: "Проверим QR у новых телефонов уже на месте",
      time: "16:05",
      members: 4,
      active: false,
    },
  ],
  en: [
    {
      title: "Home mesh",
      preview: "Clip uploaded and shared only inside the local group",
      time: "18:42",
      members: 5,
      active: true,
    },
    {
      title: "Weekend house",
      preview: "We will verify new phones with QR once everyone arrives",
      time: "16:05",
      members: 4,
      active: false,
    },
  ],
};

const members = {
  ru: [
    { name: "Rimus", meta: "Windows · проверено по QR · онлайн" },
    { name: "Mila", meta: "Android · проверено по коду · онлайн" },
    { name: "Artem", meta: "Android · ожидает ручную проверку" },
    { name: "Dana", meta: "macOS · онлайн" },
  ],
  en: [
    { name: "Rimus", meta: "Windows · QR verified · online" },
    { name: "Mila", meta: "Android · code verified · online" },
    { name: "Artem", meta: "Android · waiting for manual verification" },
    { name: "Dana", meta: "macOS · online" },
  ],
};

const messages = {
  ru: [
    {
      author: "Mila",
      own: false,
      time: "18:31",
      text: "Я загрузила короткое видео, проверьте, у всех ли дошло без интернета.",
      attachments: ["clip.mov · 18 MB"],
      reactions: ["👍 3", "🔥 1"],
    },
    {
      author: "Rimus",
      own: true,
      time: "18:34",
      reply: "Я загрузила короткое видео, проверьте...",
      text: "Получил сразу. Через LAN всё прошло заметно быстрее, чем через запасной режим.",
      attachments: [],
      reactions: ["✅ 2"],
    },
    {
      author: "Dana",
      own: false,
      time: "18:39",
      text: "Нужно еще показать в интерфейсе, что после удаления участника ключи группы сменятся автоматически.",
      attachments: [],
      reactions: [],
    },
  ],
  en: [
    {
      author: "Mila",
      own: false,
      time: "18:31",
      text: "I uploaded a short video. Check whether everyone received it with no internet.",
      attachments: ["clip.mov · 18 MB"],
      reactions: ["👍 3", "🔥 1"],
    },
    {
      author: "Rimus",
      own: true,
      time: "18:34",
      reply: "I uploaded a short video. Check whether...",
      text: "Received it instantly. LAN mode felt much faster than the fallback path.",
      attachments: [],
      reactions: ["✅ 2"],
    },
    {
      author: "Dana",
      own: false,
      time: "18:39",
      text: "We should also show in the UI that group keys rotate automatically after a member is removed.",
      attachments: [],
      reactions: [],
    },
  ],
};

const copyNodes = Array.from(document.querySelectorAll("[data-copy]"));
const chatListNode = document.getElementById("chat-list");
const memberListNode = document.getElementById("member-list");
const messageListNode = document.getElementById("message-list");
const threadTitleNode = document.getElementById("thread-title");
const onlineCountNode = document.getElementById("online-count");
const searchInputNode = document.getElementById("search-input");
const languageToggleNode = document.getElementById("language-toggle");
const themeToggleNode = document.getElementById("theme-toggle");

function renderCopy() {
  const dictionary = copy[state.language];

  document.documentElement.lang = state.language;
  copyNodes.forEach((node) => {
    const key = node.dataset.copy;
    node.textContent = dictionary[key];
  });

  languageToggleNode.textContent = state.language === "ru" ? "EN" : "RU";
  searchInputNode.placeholder =
    state.language === "ru" ? "например, ключи" : "for example, keys";
}

function renderChats() {
  chatListNode.innerHTML = "";
  chats[state.language].forEach((chat) => {
    const card = document.createElement("article");
    card.className = `chat-card${chat.active ? " active" : ""}`;
    card.innerHTML = `
      <div class="chat-card-top">
        <div>
          <p class="chat-card-title">${chat.title}</p>
          <p class="chat-card-preview">${chat.preview}</p>
        </div>
        <span class="message-meta">${chat.time}</span>
      </div>
      <p class="message-meta">${chat.members} ${state.language === "ru" ? "участников" : "members"}</p>
    `;
    chatListNode.appendChild(card);
  });

  threadTitleNode.textContent = chats[state.language][0].title;
  onlineCountNode.textContent =
    state.language === "ru" ? "4 в сети" : "4 online";
}

function renderMembers() {
  memberListNode.innerHTML = "";
  members[state.language].forEach((member) => {
    const card = document.createElement("article");
    card.className = "member-card";
    card.innerHTML = `
      <div class="member-card-top">
        <p class="member-name">${member.name}</p>
      </div>
      <p class="member-meta">${member.meta}</p>
    `;
    memberListNode.appendChild(card);
  });
}

function renderMessages() {
  messageListNode.innerHTML = "";

  const filteredMessages = messages[state.language].filter((message) => {
    if (!state.query) {
      return true;
    }

    const normalized = `${message.text} ${message.attachments.join(" ")}`.toLowerCase();
    return normalized.includes(state.query);
  });

  filteredMessages.forEach((message) => {
    const row = document.createElement("article");
    row.className = `message-row${message.own ? " own" : ""}`;

    const attachments = message.attachments
      .map((attachment) => `<span class="attachment-chip">${attachment}</span>`)
      .join("");
    const reactions = message.reactions
      .map((reaction) => `<span class="reaction-chip">${reaction}</span>`)
      .join("");
    const reply = message.reply
      ? `<p class="reply-preview">${message.reply}</p>`
      : "";

    row.innerHTML = `
      <div class="message-bubble">
        <p class="message-author">${message.author}</p>
        ${reply}
        <p class="message-text">${message.text}</p>
        <div>${attachments}</div>
        <div>${reactions}</div>
        <p class="message-meta">${message.time}</p>
      </div>
    `;

    messageListNode.appendChild(row);
  });
}

function renderTheme() {
  document.body.dataset.theme = state.theme;
  themeToggleNode.textContent =
    state.theme === "dark" ? copy[state.language].theme : state.language === "ru" ? "Тёмная тема" : "Dark theme";
}

languageToggleNode.addEventListener("click", () => {
  state.language = state.language === "ru" ? "en" : "ru";
  render();
});

themeToggleNode.addEventListener("click", () => {
  state.theme = state.theme === "dark" ? "light" : "dark";
  renderTheme();
});

searchInputNode.addEventListener("input", (event) => {
  state.query = event.target.value.trim().toLowerCase();
  renderMessages();
});

function render() {
  renderCopy();
  renderChats();
  renderMembers();
  renderMessages();
  renderTheme();
}

render();
