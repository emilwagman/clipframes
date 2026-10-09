// History: past rounds, newest first. Copy one again, open its folder, or delete it.

import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "../ui/icons";

export interface Entry {
  /** The folder's name, which is also when it was made: "2026-10-09_11-42-30". */
  id: string;
  title: string;
  when: string;
  count: number;
  /** Paths of up to four pictures to show. */
  images: string[];
}

export interface Page {
  entries: Entry[];
  total: number;
}

/** Where History gets its captures. The app asks the core; the lab hands in examples. */
export interface HistoryApi {
  list(from: number, count: number): Promise<Page>;
  copy(id: string): Promise<void>;
  reveal(id: string): Promise<void>;
  remove(id: string): Promise<void>;
  /** A picture's path as something an <img> can load. */
  src(path: string): string;
}

const core: HistoryApi = {
  list: (from, count) => invoke<Page>("history_list", { from, count }),
  copy: (id) => invoke("history_copy", { id }),
  reveal: (id) => invoke("history_reveal", { id }),
  remove: (id) => invoke("history_delete", { id }),
  src: convertFileSrc,
};

const PAGE = 40;

export function History({ api = core }: { api?: HistoryApi }) {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [total, setTotal] = useState<number | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const loading = useRef(false);

  const load = useCallback(async (from: number) => {
    if (loading.current) return;
    loading.current = true;
    const page = await api.list(from, PAGE);
    setEntries((before) => (from === 0 ? page.entries : [...before, ...page.entries]));
    setTotal(page.total);
    loading.current = false;
  }, [api]);

  useEffect(() => {
    document.documentElement.classList.add("window");
    void load(0);
  }, [load]);

  const onScroll = (event: React.UIEvent<HTMLElement>) => {
    const node = event.currentTarget;
    if (total !== null && entries.length < total && node.scrollTop + node.clientHeight > node.scrollHeight - 600) void load(entries.length);
  };

  const copy = async (id: string) => {
    await api.copy(id);
    setCopied(id);
    setTimeout(() => setCopied((now) => (now === id ? null : now)), 1600);
  };
  const remove = async (id: string) => {
    await api.remove(id);
    setEntries((before) => before.filter((e) => e.id !== id));
    setTotal((n) => (n === null ? n : n - 1));
    setConfirming(null);
  };

  return (
    <main className="history" onScroll={onScroll}>
      <header>
        <h1>History</h1>
        <span>{total === null ? "" : total === 1 ? "1 capture" : `${total} captures`}</span>
      </header>
      {total === 0 && <p className="nothing">Nothing yet. What you pick with Clipframes is kept here, so you can paste it again later.</p>}
      <ul>
        {entries.map((entry) => (
          <li key={entry.id}>
            <div className="thumbs">
              {entry.images.length === 0 ? <div className="thumb none">{entry.count}</div> : entry.images.slice(0, 3).map((path) => <img key={path} className="thumb" src={api.src(path)} alt="" loading="lazy" />)}
            </div>
            <div className="about">
              <strong>{entry.title}</strong>
              <span>
                {entry.when}
                {entry.count > 1 ? ` · ${entry.count} things` : ""}
              </span>
            </div>
            {confirming === entry.id ? (
              <div className="actions">
                <button className="plain danger" onClick={() => void remove(entry.id)}>
                  Delete
                </button>
                <button className="plain" onClick={() => setConfirming(null)}>
                  Keep
                </button>
              </div>
            ) : (
              <div className="actions">
                <button className="plain" onClick={() => void copy(entry.id)}>
                  <Icon name="copy" />
                  {copied === entry.id ? "Copied" : "Copy"}
                </button>
                <button className="ghost" title="Open the folder" onClick={() => void api.reveal(entry.id)}>
                  <Icon name="folder" />
                </button>
                <button className="ghost" title="Delete" onClick={() => setConfirming(entry.id)}>
                  <Icon name="trash" />
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>
    </main>
  );
}
