/* Página técnica isolada do React e dos temas. Não guarda estado de domínio. */
let nextId = 0;
const pending = new Map();
const output = document.querySelector("#output");
const write = (name, value) => { output.textContent += `${name}: ${JSON.stringify(value)}\n`; };
window.__takoDeliver = (text) => {
  const result = JSON.parse(text);
  if (result.event) return write("event", result.event);
  const promise = pending.get(result.id);
  if (!promise) return;
  pending.delete(result.id);
  clearTimeout(promise.timer);
  if (result.ok) promise.resolve(result.reply?.data);
  else promise.reject(result.error);
};
function call(name, payload) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`timeout: ${name}`)); }, 5000);
    pending.set(id, { resolve, reject, timer });
    try { window.takoHost.post(JSON.stringify({ v: 1, id, name, payload })); }
    catch (error) { clearTimeout(timer); pending.delete(id); reject(error); }
  });
}
for (const element of document.querySelectorAll("[data-text]")) {
  element.textContent = window.testTexts[element.dataset.text];
}
document.querySelector("#cover").alt = window.testTexts.image;
for (const button of document.querySelectorAll("[data-action]")) {
  button.addEventListener("click", () => call("window.action", button.dataset.action).catch(error => write("error", error)));
}
document.querySelector("#scan").addEventListener("click", async () => write("library.scan reply", await call("library.scan")));
document.querySelector("#dialogs").addEventListener("click", () => {
  window.onbeforeunload = event => { event.preventDefault(); event.returnValue = ""; };
  alert(window.testTexts.dialogs);
  write("alert suppressed", true);
});
document.querySelector("#save").addEventListener("click", async () => {
  await call("ui.state.set", { json: JSON.stringify({ screen: document.querySelector("#state").value }) });
  write("saved", await call("ui.state"));
});
document.querySelector("#read").addEventListener("click", async () => write("state", await call("ui.state")));
document.querySelector("#window-test").addEventListener("click", async () => {
  const size = () => [window.innerWidth, window.innerHeight];
  const before = size();
  await call("window.action", "toggle_maximize");
  await new Promise(resolve => setTimeout(resolve, 500));
  const maximized = size();
  await call("window.action", "toggle_maximize");
  await new Promise(resolve => setTimeout(resolve, 500));
  const restored = size();
  write("window roundtrip", { before, maximized, restored, pass: before.toString() === restored.toString() && before.toString() !== maximized.toString() });
});
async function check() {
  const info = await call("app.info");
  const previous = await call("ui.state");
  write("app.info", info);
  write("previous ui.state", previous);
  const before = previous.json ? JSON.parse(previous.json) : {};
  const json = JSON.stringify({ screen: before.screen || "library/steam:570", visits: (before.visits || 0) + 1 });
  await call("ui.state.set", { json });
  const current = await call("ui.state");
  if (current.json !== json || !info.version) throw new Error("bridge mismatch");
  document.querySelector("#state").value = JSON.parse(current.json).screen;
  write("ui.state roundtrip PASS", current);
  const image = document.querySelector("#cover");
  await new Promise((resolve, reject) => {
    if (image.complete) return image.naturalWidth ? resolve() : reject(new Error("image"));
    image.onload = resolve;
    image.onerror = () => reject(new Error("image"));
  });
  write("tako-img PASS", { width: image.naturalWidth, height: image.naturalHeight });
  write("library.sources", await call("library.sources"));
  write("library.scan reply", await call("library.scan"));
}
check().catch(error => { write("FAIL", String(error)); });
