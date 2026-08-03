import * as api from "./api.js";

const THEMES = ["auto", "light", "dark"];
const THEME_ICONS = { auto: "◐", light: "☀", dark: "☾" };
const LOG_LIMIT = 500;

const dom = {
  modes: document.querySelectorAll("[data-mode]"),
  source: document.querySelector("#source"),
  pickSource: document.querySelector("#pick-source"),
  cover: document.querySelector("#cover"),
  coverPreview: document.querySelector("#cover-preview"),
  pickCover: document.querySelector("#pick-cover"),
  clearCover: document.querySelector("#clear-cover"),
  outputName: document.querySelector("#output-name"),
  destination: document.querySelector("#destination"),
  pickDestination: document.querySelector("#pick-destination"),
  resetDestination: document.querySelector("#reset-destination"),
  start: document.querySelector("#start"),
  cancel: document.querySelector("#cancel"),
  reveal: document.querySelector("#reveal"),
  progressCard: document.querySelector("#progress-card"),
  progressLabel: document.querySelector("#progress-label"),
  progressCounter: document.querySelector("#progress-counter"),
  progressBar: document.querySelector("#progress-bar"),
  log: document.querySelector("#log"),
  clearLog: document.querySelector("#clear-log"),
  snackbar: document.querySelector("#snackbar"),
  themeToggle: document.querySelector("#theme-toggle"),
  themeIcon: document.querySelector("#theme-icon"),
};

const state = {
  mode: "archive",
  source: "",
  destination: "",
  cover: "",
  running: false,
  lastOutput: "",
};

/* --- Тема --- */

function applyTheme(theme) {
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved = theme === "auto" ? (dark ? "dark" : "light") : theme;
  document.documentElement.dataset.theme = resolved;
  dom.themeIcon.textContent = THEME_ICONS[theme];
  localStorage.setItem("theme", theme);
}

function initTheme() {
  const stored = THEMES.includes(localStorage.getItem("theme"))
    ? localStorage.getItem("theme")
    : "auto";
  applyTheme(stored);

  dom.themeToggle.addEventListener("click", () => {
    const current = localStorage.getItem("theme") ?? "auto";
    applyTheme(THEMES[(THEMES.indexOf(current) + 1) % THEMES.length]);
  });

  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", () => applyTheme(localStorage.getItem("theme") ?? "auto"));
}

/* --- Журнал и уведомления --- */

function log(message, level = "info") {
  const line = document.createElement("li");
  line.className = `is-${level}`;
  line.textContent = `${new Date().toLocaleTimeString()}  ${message}`;
  dom.log.append(line);

  while (dom.log.childElementCount > LOG_LIMIT) {
    dom.log.firstElementChild.remove();
  }
  dom.log.scrollTop = dom.log.scrollHeight;
}

let snackbarTimer;
function notify(message, variant = "") {
  dom.snackbar.textContent = message;
  dom.snackbar.className = variant ? `snackbar snackbar--${variant}` : "snackbar";
  dom.snackbar.hidden = false;

  clearTimeout(snackbarTimer);
  snackbarTimer = setTimeout(() => {
    dom.snackbar.hidden = true;
  }, 5000);
}

/* --- Прогресс --- */

function showProgress(label) {
  dom.progressCard.hidden = false;
  dom.progressLabel.textContent = label;
  dom.progressCounter.textContent = "";
  dom.progressBar.style.width = "";
  dom.progressBar.classList.add("progress__bar--indeterminate");
}

function advanceProgress(done, total) {
  dom.progressBar.classList.remove("progress__bar--indeterminate");
  dom.progressBar.style.width = `${Math.round((done / total) * 100)}%`;
  dom.progressLabel.textContent = "Собираю главы…";
  dom.progressCounter.textContent = `${done} / ${total}`;
}

/* --- Состояние формы --- */

function setRunning(running) {
  state.running = running;
  dom.start.disabled = running;
  dom.cancel.hidden = !running;
  dom.pickSource.disabled = running;
  dom.pickCover.disabled = running;
  dom.clearCover.disabled = running;
  dom.pickDestination.disabled = running;
  dom.resetDestination.disabled = running;
  dom.outputName.disabled = running;
  dom.modes.forEach((button) => (button.disabled = running));
  dom.start.textContent = running ? "Идёт сборка…" : "Подготовить";
}

function setMode(mode) {
  state.mode = mode;
  dom.modes.forEach((button) => {
    const selected = button.dataset.mode === mode;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-checked", String(selected));
  });
}

async function setCover(path) {
  state.cover = path;
  dom.cover.value = path;
  dom.clearCover.hidden = !path;

  // Ссылку предыдущего превью нужно отпустить, иначе байты висят в памяти.
  URL.revokeObjectURL(dom.coverPreview.src);
  dom.coverPreview.removeAttribute("src");
  dom.coverPreview.hidden = true;
  if (!path) return;

  try {
    const blob = await api.readCover(path);
    // Пока читали файл, пользователь мог выбрать другой — это превью уже лишнее.
    if (state.cover !== path) return;

    dom.coverPreview.src = URL.createObjectURL(blob);
    dom.coverPreview.hidden = false;
  } catch (error) {
    log(`Не удалось показать превью обложки: ${error}`, "warn");
  }
}

function baseName(path) {
  return path.split(/[\\/]/).pop() ?? "";
}

function suggestOutputName(source) {
  const name = baseName(source).replace(/\.zip$/i, "");
  return name ? `${name}_kcc` : "";
}

/* --- События ядра --- */

function handlePipelineEvent(event) {
  switch (event.kind) {
    case "log":
      log(event.message, event.level);
      break;
    case "chaptersFound":
      log(`Найдено глав: ${event.total}`);
      advanceProgress(0, event.total);
      break;
    case "chapterDone":
      log(`[${event.index}/${event.total}] ${event.name} — страниц: ${event.images}`);
      advanceProgress(event.index, event.total);
      break;
    case "finished":
      log(`Готово: ${event.images} страниц из ${event.chapters} глав → ${event.output}`, "done");
      break;
  }
}

/* --- Запуск --- */

async function start() {
  if (!state.source) {
    notify("Сначала выберите источник", "error");
    return;
  }
  if (!dom.outputName.value.trim()) {
    notify("Укажите название папки для результата", "error");
    return;
  }

  setRunning(true);
  dom.reveal.hidden = true;
  showProgress("Ищу главы…");

  try {
    const outcome = await api.prepare({
      source: state.source,
      destination: state.destination || null,
      cover: state.cover || null,
      outputName: dom.outputName.value.trim(),
    });

    state.lastOutput = outcome.output;
    dom.reveal.hidden = false;
    dom.progressLabel.textContent = "Готово";
    notify(`Собрано ${outcome.images} страниц — папка готова для KCC`);
  } catch (error) {
    const message = String(error);
    log(message, "error");
    notify(message, "error");
    dom.progressLabel.textContent = "Прервано";
    dom.progressBar.classList.remove("progress__bar--indeterminate");
  } finally {
    setRunning(false);
  }
}

/* --- Инициализация --- */

function bind() {
  dom.modes.forEach((button) =>
    button.addEventListener("click", () => setMode(button.dataset.mode)),
  );

  dom.pickSource.addEventListener("click", async () => {
    const picked = await api.pickSource(state.mode);
    if (!picked) return;

    state.source = picked;
    dom.source.value = picked;
    if (!dom.outputName.value.trim()) {
      dom.outputName.value = suggestOutputName(picked);
    }
  });

  dom.pickCover.addEventListener("click", async () => {
    const picked = await api.pickCover();
    if (!picked) return;

    setCover(picked);
  });

  dom.clearCover.addEventListener("click", () => setCover(""));

  dom.pickDestination.addEventListener("click", async () => {
    const picked = await api.pickDirectory();
    if (!picked) return;

    state.destination = picked;
    dom.destination.value = picked;
  });

  dom.resetDestination.addEventListener("click", () => {
    state.destination = "";
    dom.destination.value = "";
  });

  dom.start.addEventListener("click", start);
  dom.cancel.addEventListener("click", () => {
    api.cancel();
    log("Запрошена отмена — остановлюсь после текущей главы.", "warn");
  });
  dom.reveal.addEventListener("click", () => api.reveal(state.lastOutput));
  dom.clearLog.addEventListener("click", () => dom.log.replaceChildren());

  dom.outputName.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !state.running) start();
  });
}

initTheme();
setMode("archive");
bind();
api.onPipelineEvent(handlePipelineEvent);
