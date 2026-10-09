// History: past rounds, newest first. Copy one again, open its folder, or delete it.

import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "../ui/icons";

interface Entry {
  /** The folder's name, which is also when it was made: "2026-10-09_11-42-30". */
  id: string;
  title: string;
  when: string;
  count: number;
  /** Paths of up to four pictures to show. */
  images: string[];
  kinds: string[];
}

interface Page {
  entries: Entry[];
  total: number;
}

const PAGE = 40;

export function History() {
  const [entries, setEntries] = useState<Entry[]>([]);
  const [total, setTotal] = useState<number | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const loading = useRef(false);

  const load = useCallback(async (from: number) => {
    if (loading.current) return;
    loading.current = true;
    const page = await invoke<Page>("history_list", { from, count: PAGE });
    setEntries((before) => (from === 0 ? page.entries : [...before, ...page.entries]));
    setTotal(page.total);
    loading.current = false;
  }, []);

  useEffect(() => {
    document.documentElement.classList.add("window");
    void load(0);
  }, [load]);

  const onScroll = (event: React.UIEvent<HTMLElement>) => {
    const node = event.currentTarget;
    if (total !== null && entries.length < total && node.scrollTop + node.clientHeight > node.scrollHeight - 600) void load(entries.length);
  };

  const copy = async (id: string) => {
    await invoke("history_copy", { id });
    setCopied(id);
    setTimeout(() => setCopied((now) => (now === id ? null : now)), 1600);
  };
  const remove = async (id: string) => {
    await invoke("history_delete", { id });
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
              {entry.images.length === 0 ? <div className="thumb none">{entry.count}</div> : entry.images.slice(0, 3).map((path) => <img key={path} className="thumb" src={convertFileSrc(path)} alt="" loading="lazy" />)}
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
                <button className="ghost" title="Open the folder" onClick={() => void invoke("history_reveal", { id: entry.id })}>
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
