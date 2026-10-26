// Notifica o núcleo sobre a origem da entrada atual (controle vs teclado/mouse).
// Limita o envio a no máximo uma vez por segundo nas transições de origem.

export type SourceSender = (pad: boolean) => void;

let lastReportedPad: boolean | null = null;
let lastReportTime = 0;
let pendingPad: boolean | null = null;
let reportTimer: ReturnType<typeof setTimeout> | null = null;
let activeSender: SourceSender | null = null;

export function setSourceSender(sender: SourceSender): void {
  activeSender = sender;
}

export function reportInputSource(
  pad: boolean,
  now: number = Date.now(),
  senderOverride?: SourceSender,
): void {
  const sender = senderOverride ?? activeSender;
  if (lastReportedPad === pad && pendingPad === null) {
    return;
  }

  if (pendingPad === pad) {
    return;
  }

  const elapsed = now - lastReportTime;
  if (elapsed >= 1000 && lastReportedPad !== pad) {
    if (reportTimer !== null) {
      clearTimeout(reportTimer);
      reportTimer = null;
    }
    pendingPad = null;
    lastReportedPad = pad;
    lastReportTime = now;
    sender?.(pad);
  } else {
    pendingPad = pad;
    if (reportTimer === null) {
      const wait = Math.max(0, 1000 - elapsed);
      reportTimer = setTimeout(() => {
        reportTimer = null;
        if (pendingPad !== null && pendingPad !== lastReportedPad) {
          const target = pendingPad;
          pendingPad = null;
          lastReportedPad = target;
          lastReportTime = Date.now();
          sender?.(target);
        } else {
          pendingPad = null;
        }
      }, wait);
    }
  }
}

export function resetSourceReporter(): void {
  if (reportTimer !== null) {
    clearTimeout(reportTimer);
    reportTimer = null;
  }
  lastReportedPad = null;
  lastReportTime = 0;
  pendingPad = null;
  activeSender = null;
}
