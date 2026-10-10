import type { AgentLook, Exchange } from "./cases";
import s from "./scene.module.css";

export interface Line {
  who: "you" | "did" | "says";
  text: string;
}

/// One exchange as the lines it is drawn with.
const told = ({ ask, did, says }: Exchange): Line[] => [{ who: "you", text: ask }, { who: "did", text: did }, { who: "says", text: says }];

function Lines({ lines, look }: { lines: Line[]; look: AgentLook }) {
  return (
    <>
      {lines.map((line, i) => (
        <p key={i} className={s[line.who]} data-line={line.who}>
          <i aria-hidden="true">{line.who === "you" ? (look === "claude" ? "❯" : "›") : look === "claude" ? "●" : "•"}</i>
          <span>{line.text}</span>
        </p>
      ))}
    </>
  );
}

/// The agent's terminal, beside the thing being built. Each agent's window is drawn the way
/// that agent lays its own out, so someone who uses it knows it at once: Claude Code with its
/// name and folder at the top, its prompt between two rules and the model and mode under it;
/// Codex with its boxed heading, what you said on a band, and its prompt on a band at the foot.
/// These are our own drawings of that arrangement (studied in the app's own recordings and in
/// Codex's repository), and neither is a picture of the real thing. `typed` is what is in the
/// prompt; `before` is what was said before the example began.
export default function Terminal({ look, folder, before, lines, typed, working, promptRef }: { look: AgentLook; folder: string; before: Exchange; lines: Line[]; typed: string; working: boolean; promptRef: React.Ref<HTMLDivElement> }) {
  const claude = look === "claude";
  return (
    <div className={`${s.window} ${s.term}`} data-agent={look}>
      <div className={s.termBar} aria-hidden="true"><span>{claude ? "Claude Code" : "Codex"}</span></div>
      <div className={s.termBody}>
        {claude ? (
          <p className={s.hello}><b>Claude Code</b><span>Opus 5.5</span><span>{folder}</span></p>
        ) : (
          <p className={s.hello}><b>&gt;_ Codex</b><span>model: gpt-5.2-codex</span><span>directory: {folder}</span></p>
        )}
        <div className={s.lines}>
          <div className={s.earlier} aria-hidden="true"><Lines lines={told(before)} look={look} /></div>
          <div className={s.now} data-now="" aria-live="polite">
            <Lines lines={lines} look={look} />
            {working && <p className={s.working} data-line="working"><i aria-hidden="true">{claude ? "✻" : "•"}</i><span>Working…</span></p>}
          </div>
        </div>
        <div ref={promptRef} className={s.prompt} data-agent-prompt="" data-filled={typed ? "" : undefined}>
          <i aria-hidden="true">{claude ? "❯" : "›"}</i>
          {typed
            ? <span key="typed" className={s.typed}>{typed}</span>
            : <span key="empty" className={s.caret}>{claude && <em>Try &quot;make the header sticky&quot;</em>}</span>}
        </div>
        {claude && <p className={s.status} aria-hidden="true"><span>Opus 5.5</span><b>▸▸ auto mode on</b> <span>(shift+tab to cycle)</span></p>}
      </div>
    </div>
  );
}
