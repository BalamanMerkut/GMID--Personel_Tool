// Bir ekranda belirli bir kaydı açma isteği (ör. aramada bir nota tıklanınca Notlar o notla açılır).
// Ekran açıkken de çalışsın diye durum reaktif: ekranlar `focus.n` değişince `takeFocus` ile alır.

export const focus = $state<{ kind: string | null; id: string | null; n: number }>({ kind: null, id: null, n: 0 });

export function requestFocus(kind: string, id: string): void {
  focus.kind = kind;
  focus.id = id;
  focus.n++;
}

/** Bu türde bekleyen istek varsa kimliğini verir ve isteği siler. */
export function takeFocus(kind: string): string | null {
  if (focus.kind !== kind || !focus.id) return null;
  const id = focus.id;
  focus.kind = null;
  focus.id = null;
  return id;
}
