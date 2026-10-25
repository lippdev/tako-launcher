import assert from "node:assert/strict";
import { test } from "node:test";
import { translate } from "../i18n/core.ts";
import {
  clock,
  collectionDetail,
  collectionsSummary,
  dayHeading,
  dayLabel,
  daysBetween,
  durationText,
  historySession,
  initial,
  lastPlayedText,
  localDay,
  playTimeText,
  profileName,
  sessionGroups,
  sessionsSummary,
  shortDate,
  weekAriaLabel,
  weekBars,
  weekdayIndex,
} from "./perfil/perfilModel.ts";

// Os textos do catálogo pt-BR (`profile.toml`, `profiles.toml` e as chaves de outras telas que a tela usa).
const messages = {
  "profile.duration.minutes": "{m} min",
  "profile.duration.hours": "{h} h",
  "profile.duration.hours_minutes": "{h} h {m} min",
  "presentation.play_time.minutes": "{n} min",
  "presentation.play_time.hours": "{n} h",
  "presentation.date.short": "%d/%m",
  "desktop.day.today": "Hoje",
  "desktop.day.yesterday": "Ontem",
  "profile.weekday.today": "hoje",
  "profile.weekday.mon": "seg",
  "profile.weekday.tue": "ter",
  "profile.weekday.wed": "qua",
  "profile.weekday.thu": "qui",
  "profile.weekday.fri": "sex",
  "profile.weekday.sat": "sáb",
  "profile.weekday.sun": "dom",
  "profile.weekday_long.mon": "Segunda",
  "profile.weekday_long.tue": "Terça",
  "profile.weekday_long.wed": "Quarta",
  "profile.weekday_long.thu": "Quinta",
  "profile.weekday_long.fri": "Sexta",
  "profile.weekday_long.sat": "Sábado",
  "profile.weekday_long.sun": "Domingo",
  "profile.this_week": "Esta semana",
  "profile.nothing": "nada",
  "profile.last": "Jogou por último {when}",
  "profile.summary.one": "{n} sessão · {time}",
  "profile.summary.other": "{n} sessões · {time}",
  "profile.collections.one": "{n} coleção",
  "profile.collections.other": "{n} coleções",
  "common.games.one": "{n} jogo",
  "common.games.other": "{n} jogos",
  "profile.installed.one": "{n} instalado",
  "profile.installed.other": "{n} instalados",
  "profile.no_games": "Nenhum jogo ainda",
  "profile.quick": "Abriu e fechou",
  "profile.in_game": "{time} no jogo",
  "profiles.main": "Principal",
};
const t = (key, vars) => translate(messages, key, vars);

const sec = (minutes) => minutes * 60;

test("a duração lê com unidades, como o app Slint", () => {
  assert.equal(durationText(sec(42), t), "42 min");
  assert.equal(durationText(sec(120), t), "2 h");
  assert.equal(durationText(sec(397), t), "6 h 37 min");
  assert.equal(durationText(sec(62), t), "1 h 02 min");
  assert.equal(durationText(0, t), "0 min");
  assert.equal(durationText(-5, t), "0 min");
});

test("o tempo de jogo total: zero é traço, menos de uma hora em minutos", () => {
  assert.equal(playTimeText(0, t), "—");
  assert.equal(playTimeText(20, t), "1 min");
  assert.equal(playTimeText(sec(59), t), "59 min");
  assert.equal(playTimeText(sec(598 * 60 + 30), t), "598 h");
});

test("datas: dias entre, dia da semana e data curta pelo padrão do idioma", () => {
  assert.equal(daysBetween("2026-10-23", "2026-10-24"), 1);
  assert.equal(daysBetween("2026-10-24", "2026-10-23"), -1);
  assert.equal(daysBetween("2026-02-28", "2026-03-01"), 1);
  assert.equal(daysBetween("lixo", "2026-03-01"), 0);
  // 24/10/2026 é sábado.
  assert.equal(weekdayIndex("2026-10-24"), 5);
  assert.equal(weekdayIndex("2026-10-19"), 0);
  assert.equal(weekdayIndex("2026-10-25"), 6);
  assert.equal(shortDate("2026-10-05", t), "05/10");
  const en = (key, vars) => translate({ ...messages, "presentation.date.short": "%m/%d" }, key, vars);
  assert.equal(shortDate("2026-10-05", en), "10/05");
  assert.equal(shortDate("lixo", t), "lixo");
});

test("hoje, ontem, o dia da semana e a data curta", () => {
  const today = "2026-10-24";
  assert.equal(dayLabel("2026-10-24", today, t), "Hoje");
  assert.equal(dayLabel("2026-10-23", today, t), "Ontem");
  assert.equal(dayLabel("2026-10-20", today, t), "20/10");
  assert.deepEqual(dayHeading("2026-10-24", today, t), { title: "Hoje", subtitle: "24/10" });
  assert.deepEqual(dayHeading("2026-10-23", today, t), { title: "Ontem", subtitle: "23/10" });
  assert.deepEqual(dayHeading("2026-10-20", today, t), { title: "Terça", subtitle: "20/10" });
  assert.equal(lastPlayedText(null, today, t), null);
});

function week(seconds) {
  // Segunda 19/10 a domingo 25/10.
  return seconds.map((s, i) => ({ day: `2026-10-${String(19 + i)}`, seconds: s }));
}

test("a semana: barras em relação ao maior dia, hoje com o rótulo próprio e o futuro vazio", () => {
  const w = week([sec(40), sec(80), 0, 0, 0, sec(20), 0]);
  const bars = weekBars(w, "2026-10-24", t);
  assert.equal(bars.length, 7);
  assert.deepEqual(
    bars.map((b) => b.label),
    ["seg", "ter", "qua", "qui", "sex", "hoje", "dom"],
  );
  assert.deepEqual(
    bars.map((b) => b.value),
    [0.5, 1, 0, 0, 0, 0.25, 0],
  );
  assert.deepEqual(
    bars.map((b) => b.isFuture),
    [false, false, false, false, false, false, true],
  );
  assert.equal(bars[5].isToday, true);
  // Semana sem tempo nenhum: tudo zero, sem dividir por zero.
  assert.ok(weekBars(week([0, 0, 0, 0, 0, 0, 0]), "2026-10-24", t).every((b) => b.value === 0));
});

test("o gráfico da semana para leitor de tela para no dia de hoje", () => {
  const w = week([sec(40), 0, sec(75), 0, 0, 0, 0]);
  assert.equal(weekAriaLabel(w, "2026-10-21", t), "Esta semana: Segunda 40 min, Terça nada, Quarta 1 h 15 min");
});

test("o nome do perfil: o principal sem nome é Principal; inicial maiúscula", () => {
  assert.equal(profileName({ name: "" }, t), "Principal");
  assert.equal(profileName({ name: "Ana" }, t), "Ana");
  assert.equal(initial("ana clara"), "A");
  assert.equal(initial("  édson"), "É");
  assert.equal(initial(""), "");
});

test("os resumos usam o plural do idioma", () => {
  assert.equal(sessionsSummary(1, sec(42), t), "1 sessão · 42 min");
  assert.equal(sessionsSummary(6, sec(397), t), "6 sessões · 6 h 37 min");
  assert.equal(sessionsSummary(0, 0, t), "0 sessões · 0 min");
  assert.equal(collectionsSummary(4, 26, t), "4 coleções · 26 jogos");
  assert.equal(collectionsSummary(1, 1, t), "1 coleção · 1 jogo");
  assert.equal(collectionDetail(0, 0, t), "Nenhum jogo ainda");
  assert.equal(collectionDetail(6, 4, t), "6 jogos · 4 instalados");
  assert.equal(collectionDetail(1, 1, t), "1 jogo · 1 instalado");
});

// Instantes locais (o teste não depende do fuso da máquina).
const at = (day, hour, minute) => new Date(2026, 9, day, hour, minute).toISOString();

test("o dia e a hora locais de um instante", () => {
  assert.equal(localDay(at(24, 23, 30)), "2026-10-24");
  assert.equal(clock(at(24, 9, 5)), "09:05");
});

function session(over = {}) {
  return {
    game: { id: "steam:1", title: "Hades II", playTimeSeconds: sec(135) },
    startedAt: at(24, 14, 2),
    durationSeconds: sec(42),
    quick: false,
    ...over,
  };
}

test("a linha da Visão geral diz o dia, a hora e o total do jogo; a da guia, o início e o fim", () => {
  const options = { today: "2026-10-24", t, cover: () => undefined };
  const overview = historySession(session(), 0, { ...options, overview: true });
  assert.equal(overview.time, "Hoje, 14:02");
  assert.equal(overview.duration, "42 min");
  assert.equal(overview.status.text, "2 h no jogo");
  assert.equal("coverSrc" in overview, false);

  const tab = historySession(session(), 3, { ...options, overview: false });
  assert.equal(tab.time, "14:02 – 14:44");
  assert.equal(tab.status, undefined);

  // Sessão rápida: o texto no lugar da duração; jogo sem tempo registrado não leva o total.
  const quick = historySession(
    session({ quick: true, durationSeconds: 20, game: { id: "x", title: "X", playTimeSeconds: 0 } }),
    0,
    { ...options, overview: true, cover: () => "tako-img://cover?id=x" },
  );
  assert.equal(quick.duration, "Abriu e fechou");
  assert.equal(quick.status, undefined);
  assert.equal(quick.coverSrc, "tako-img://cover?id=x");
});

test("os grupos por dia levam o cabeçalho, o total do dia e ids que não se repetem", () => {
  const options = { today: "2026-10-24", t, cover: () => undefined };
  const groups = sessionGroups(
    [
      { day: "2026-10-24", totalSeconds: sec(55), sessions: [session(), session({ startedAt: at(24, 8, 0) })] },
      { day: "2026-10-20", totalSeconds: sec(90), sessions: [session({ startedAt: at(20, 20, 0) })] },
    ],
    options,
  );
  assert.deepEqual(
    groups.map((g) => [g.title, g.subtitle, g.total, g.sessions.length]),
    [
      ["Hoje", "24/10", "55 min", 2],
      ["Terça", "20/10", "1 h 30 min", 1],
    ],
  );
  const ids = groups.flatMap((g) => g.sessions.map((s) => s.id));
  assert.equal(new Set(ids).size, 3);
  assert.deepEqual(sessionGroups([], options), []);
});
